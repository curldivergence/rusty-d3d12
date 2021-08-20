use std::cmp::max;
use std::cmp::min;
use std::ffi::c_void;
use std::ffi::CStr;
use std::intrinsics::copy_nonoverlapping;
use std::mem::size_of;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::rc::Rc;
use std::slice;

use log::error;
use log::info;

use log::trace;
use log::warn;
use std::{ffi::CString, io::Read, slice::Windows};
use widestring::WideCStr;

use cgmath::{prelude::*, Vector2, Vector4};
use cgmath::{Matrix4, Vector3};
use memoffset::offset_of;
use rand::Rng;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowExtWindows,
    window::WindowBuilder,
};

use static_assertions::assert_eq_size;

use rusty_d3d12::*;

fn wait_for_debugger() {
    while unsafe { winapi::um::debugapi::IsDebuggerPresent() } == 0 {
        std::thread::sleep_ms(1000);
    }
}

#[no_mangle]
pub static D3D12SDKVersion: u32 = 4;

#[no_mangle]
pub static D3D12SDKPath: &[u8; 9] = b".\\D3D12\\\0";

#[allow(clippy::not_unsafe_ptr_arg_deref)]
#[no_mangle]
pub extern "C" fn debug_callback(
    category: i32,
    severity: i32,
    id: i32,
    description: *const c_char,
    _context: *mut c_void,
) {
    let severity: MessageSeverity = unsafe { std::mem::transmute(severity) };
    let category: MessageCategory = unsafe { std::mem::transmute(category) };
    let description = unsafe { CStr::from_ptr(description) };

    match severity {
        MessageSeverity::Message | MessageSeverity::Info => {
            info!(
                "[D3D12 Message][{}][{}][{:#x}] {}",
                severity,
                category,
                id as i32,
                description
                    .to_str()
                    .expect("Cannot make Rust string from D3D12 layer message")
            );
        }
        MessageSeverity::Warning => {
            warn!(
                "[D3D12 Message][{}][{}][{:#x}] {}",
                severity,
                category,
                id as i32,
                description
                    .to_str()
                    .expect("Cannot make Rust string from D3D12 layer message")
            );
        }
        _ => {
            error!(
                "[D3D12 Message][{}][{}][{:#x}] {}",
                severity,
                category,
                id as i32,
                description
                    .to_str()
                    .expect("Cannot make Rust string from D3D12 layer message")
            );
        }
    }
}

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;

const FRAMES_IN_FLIGHT: usize = 3;

const USE_DEBUG: bool = true;
const USE_WARP_ADAPTER: bool = false;

const CLEAR_COLOR: [f32; 4] = [0.0, 0.2, 0.3, 1.0];

type Mat4 = Matrix4<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;
type Vec2 = Vector2<f32>;

#[derive(Debug, Copy, Clone)]
pub struct Degrees(pub f32);

// ToDo: move all this to utils since it's duplicated in multiple examples
#[derive(Debug, Copy, Clone)]
pub struct Camera {
    pub near: f32,
    pub far: f32,
    pub fov: Degrees,
    pub aspect: f32,
    pub position: Vec3,
    pub look_at: Vec3,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            near: 0.01,
            far: 100.0,
            fov: Degrees(45.),
            aspect: WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32,
            // position: Vec3::new(0., 0.5, -1.),
            position: Vec3::new(0., 0., -1.),
            look_at: Vec3::new(0., 0., 1.),
        }
    }
}

fn make_projection_matrix(camera: &Camera) -> Mat4 {
    use cgmath::prelude::*;

    Matrix4::from_cols(
        Vec4 {
            x: 1. / (camera.aspect * cgmath::Deg(camera.fov.0 / 2.).tan()),
            y: 0.,
            z: 0.,
            w: 0.,
        },
        Vec4 {
            x: 0.,
            y: 1. / cgmath::Deg(camera.fov.0 / 2.).tan(),
            z: 0.,
            w: 0.,
        },
        Vec4 {
            x: 0.,
            y: 0.,
            z: camera.far / (camera.far - camera.near),
            w: 1.,
        },
        Vec4 {
            x: 0.,
            y: 0.,
            z: -camera.near * camera.far / (camera.far - camera.near),
            w: 0.,
        },
    )
}

fn make_view_matrix(camera_pos: Vec3, look_at: Vec3) -> Mat4 {
    let cam_k = (look_at - camera_pos).normalize();
    let wrld_up = Vec3::new(0., 1., 0.);
    let cam_i = wrld_up.cross(cam_k).normalize();
    let cam_j = cam_k.cross(cam_i);

    let orientation = Matrix4::from_cols(
        cam_i.extend(0.),
        cam_j.extend(0.),
        cam_k.extend(0.),
        Vec4::new(0., 0., 0., 1.),
    );
    // trace!("orientation matrix: {:?}", &orientation);

    let translation = Matrix4::from_cols(
        Vec4::new(1., 0., 0., 0.),
        Vec4::new(0., 1., 0., 0.),
        Vec4::new(0., 0., 1., 0.),
        Vec4::new(camera_pos[0], camera_pos[1], camera_pos[2], 1.),
    );

    let result = translation * orientation;
    result.invert().expect("No matrix inverse")
}

#[repr(C)]
pub struct Vertex {
    position: Vec3,
}

impl Vertex {
    pub fn make_desc() -> InputLayout {
        vec![InputElementDesc::default()
            .set_name(CString::new("POSITION").unwrap())
            .set_format(DxgiFormat::R32G32B32_Float)
            .set_input_slot(0)
            .set_offset(Bytes::from(offset_of!(Self, position)))]
    }
}

#[repr(C)]
struct SceneConstantBuffer {
    velocity: Vec4,
    offset: Vec4,
    color: Vec4,
    projection: Matrix4<f32>,

    // Constant buffers are 256-byte aligned. Add padding in the struct to allow multiple buffers
    // to be array-indexed.
    padding: [f32; 36],
}

static_assertions::const_assert!(size_of::<SceneConstantBuffer>() == 256);

struct InterprocessCommunicationSample {
    // pipeline: Pipeline,
}

impl InterprocessCommunicationSample {
    fn new(hwnd: *mut std::ffi::c_void) -> Self {
        // let mut pipeline = Pipeline::new(hwnd);
        // pipeline.update();
        // pipeline.render();

        InterprocessCommunicationSample {}
        // InterprocessCommunicationSample { pipeline }
    }

    fn draw(&mut self) {
        // self.pipeline.update();
        // self.pipeline.render();
    }
}

// impl Drop for InterprocessCommunicationSample {
//     fn drop(&mut self) {
//         if USE_DEBUG {
//             self.pipeline
//                 .debug_devices
//                 .as_ref()
//                 .expect("No debug devices created")[0]
//                 .report_live_device_objects()
//                 .expect("Device cannot report live objects");
//             self.pipeline
//                 .debug_devices
//                 .as_ref()
//                 .expect("No debug devices created")[1]
//                 .report_live_device_objects()
//                 .expect("Device cannot report live objects");
//         }
//     }
// }

struct Pipeline {
    device: Device,
    debug_device: Option<DebugDevice>,
    info_queue: Option<Rc<InfoQueue>>,
    direct_command_queue: CommandQueue,
    swapchain: DxgiSwapchain,
    frame_index: usize,
    frames_since_last_update: u32, // static var in OnUpdate()
    viewport: Viewport,
    scissor_rect: Rect,
    rtv_heap: DescriptorHeap,
    dsv_heap: DescriptorHeap,
    cbv_srv_heap: DescriptorHeap,
    render_targets: [Resource; FRAMES_IN_FLIGHT],
    direct_command_allocators: [CommandAllocator; FRAMES_IN_FLIGHT],
    heap_primary: Heap,
    heap_secondary: Heap,
    cross_process_resources: [Resource; FRAMES_IN_FLIGHT],
    secondary_adapter_textures: [Resource; FRAMES_IN_FLIGHT],
    intermediate_blur_render_target: Resource,

    root_signature: RootSignature,
    blur_root_signature: RootSignature,

    pipeline_state: PipelineState,

    direct_command_lists: CommandList,
    copy_command_list: CommandList,

    vertex_buffer: Resource,
    vertex_buffer_upload: Resource,
    vertex_buffer_view: VertexBufferView,

    depth_stencil: Resource,
    triangle_constant_buffer: Resource,
    triangle_cb_data: Vec<SceneConstantBuffer>,
    triangle_cb_mapped_data: *mut u8,

    frame_fence: Fence,
    render_fence: Fence,
    // cross_adapter_fences: [Fence; DEVICE_COUNT],
    fence_events: [Win32Event; 2],

    current_present_fence_value: u64,
    current_render_fence_value: u64,
    current_cross_adapter_fence_value: u64,
    frame_fence_values: [u64; FRAMES_IN_FLIGHT],
}

impl Pipeline {
    // aka LoadPipeline() in the original sample
    fn new(hwnd: *mut c_void) -> Self {
        let mut factory_flags = DxgiCreateFactoryFlags::None;
        if USE_DEBUG {
            let debug_controller =
                Debug::new().expect("Cannot create debug controller");
            debug_controller.enable_debug_layer();
            debug_controller.enable_gpu_based_validation();
            debug_controller.enable_object_auto_name();
            factory_flags = DxgiCreateFactoryFlags::Debug;
        }

        let factory =
            DxgiFactory::new(factory_flags).expect("Cannot create factory");
        let (device, is_software_adapter) = create_device(&factory);

        let debug_device;
        if USE_DEBUG {
            debug_device = Some(
                DebugDevice::new(&device).expect("Cannot create debug device"),
            );
        } else {
            debug_device = None;
        }

        let info_queue;
        if USE_DEBUG {
            let temp_info_queue = Rc::from(
                InfoQueue::new(
                    &device,
                    // Some(&[
                    //     MessageSeverity::Corruption,
                    //     MessageSeverity::Error,
                    //     MessageSeverity::Warning,
                    // ]),
                    None,
                )
                .expect("Cannot create debug info queue"),
            );

            temp_info_queue
                .register_callback(
                    debug_callback,
                    MessageCallbackFlags::FlagNone,
                )
                .expect("Cannot set debug callback on info queue");

            info_queue = Some(temp_info_queue);
        } else {
            info_queue = None;
        }

        let direct_command_queue = device
            .create_command_queue(
                &CommandQueueDesc::default().set_type(CommandListType::Direct),
            )
            .expect("Cannot create direct command queue");

        let swapchain = create_swapchain(factory, &direct_command_queue, hwnd);
        let frame_index = swapchain.get_current_back_buffer_index().0 as usize;
        trace!("Swapchain returned frame index {}", frame_index);

        let viewport = Viewport::default()
            .set_top_left_x(0.)
            .set_top_left_y(0.)
            .set_width(WINDOW_WIDTH as f32)
            .set_height(WINDOW_HEIGHT as f32);

        let scissor_rect = Rect::default()
            .set_left(0)
            .set_top(0)
            .set_right(WINDOW_WIDTH as i32)
            .set_bottom(WINDOW_HEIGHT as i32);

        let (rtv_heaps, dsv_heap, cbv_srv_heap) =
            create_descriptor_heaps(&device, &swapchain);

        let (render_targets, direct_command_allocators) =
            create_frame_resources(&device, &rtv_heaps, &swapchain);

        let (texture_size, cross_adapter_desc) =
            create_shared_resource_descs(&device);

        let shared_heap = device
            .create_heap(
                &HeapDesc::default()
                    .set_properties(
                        &HeapProperties::default().set_type(HeapType::Default),
                    )
                    .set_size_in_bytes(texture_size * FRAMES_IN_FLIGHT)
                    .set_flags(
                        HeapFlags::Shared | HeapFlags::SharedCrossAdapter,
                    ),
            )
            .expect("Cannot create heap");

        let heap_as_dc: DeviceChild = shared_heap.clone().into();
        let heap_handle = device
            .create_shared_handle(&heap_as_dc, "SharedHeapHandle")
            .expect("Cannot create shared heap handle");
        heap_handle.close();

        trace!("Successfully created and opened heaps");

        let mut cross_process_resources = vec![];

        for frame_idx in 0..FRAMES_IN_FLIGHT {
            let resource_primary = device
                .create_placed_resource(
                    &shared_heap,
                    frame_idx * texture_size,
                    &cross_adapter_desc,
                    ResourceStates::CopyDest,
                    None,
                )
                .expect("Cannot create placed resource");
            resource_primary
                .set_name(&format!("resource {}", frame_idx))
                .expect("Cannot set resource name");

            cross_process_resources[frame_idx] =
                MaybeUninit::new(resource_primary);
        }

        // This block corresponds to LoadAssets() in the original sample
        let root_signature = create_root_signature(&device);

        trace!("Created root signature");

        let pso = create_pso(&device, &root_signature);

        let direct_command_lists = create_command_list(
            &device,
            frame_index,
            &direct_command_allocators,
            &pso,
        );

        trace!("Created command list");

        let (vertex_buffer, vertex_buffer_upload, vertex_buffer_view) =
            create_vertex_buffer(&device, &direct_command_list);

        let (
            quad_vertex_buffer,
            quad_vertex_buffer_upload,
            quad_vertex_buffer_view,
        ) = create_blur_vertex_buffer(&devices, &direct_command_lists);

        trace!("Created secondary adapter quad vertex buffer");

        let depth_stencil = create_depth_stencil(&devices, &dsv_heap);

        trace!("Created depth stencil on primary adapter");

        let (
            triangle_constant_buffer,
            triangle_cb_data,
            triangle_cb_mapped_data,
        ) = create_triangle_constant_buffer(&devices);

        trace!("Created triangle constant buffer");

        let ps_loop_count = 0;
        let (
            workload_constant_buffer,
            workload_cb_data,
            workload_cb_mapped_data,
        ) = create_workload_constant_buffer(&devices, ps_loop_count);

        trace!("Created workload constant buffer");

        let blur_ps_loop_count = 0;
        let (
            blur_workload_constant_buffer,
            blur_workload_cb_data,
            blur_workload_cb_mapped_data,
        ) = create_blur_workload_constant_buffer(&devices, blur_ps_loop_count);

        trace!("Created blur workload constant buffer");

        let blur_constant_buffer = create_blur_constant_buffer(&devices);

        trace!("Created blur constant buffer");

        {
            for device_idx in 0..DEVICE_COUNT {
                direct_command_lists[device_idx]
                    .close()
                    .expect("Cannot close command list");
                direct_command_queues[device_idx].execute_command_lists(
                    slice::from_ref(&direct_command_lists[device_idx]),
                );
            }
        }

        trace!("Executed command lists");

        let (
            frame_fence,
            render_fence,
            cross_adapter_fences,
            fence_events,
            cross_adapter_fence_value,
        ) = create_fences(&devices, &direct_command_queues);

        trace!("Created fences");

        Self {
            devices,
            debug_devices,
            info_queues,
            direct_command_queues,
            direct_command_queue_timestamp_frequencies,
            copy_command_queue,
            swapchain,
            frame_index,
            frames_since_last_update: 0,
            viewport,
            scissor_rect,
            triangle_count: if is_software_adapter {
                MAX_TRIANGLE_COUNT / 50
            } else {
                MAX_TRIANGLE_COUNT / 2
            },
            rtv_heaps,
            dsv_heap,
            cbv_srv_heap,
            timestamp_result_buffers,
            current_times_index: 0,
            query_heaps,
            render_targets,
            direct_command_allocators,
            copy_allocators,
            cross_adapter_textures_supported,
            heap_primary,
            heap_secondary,
            cross_process_resources,
            secondary_adapter_textures,
            intermediate_blur_render_target,

            root_signature,
            blur_root_signature,
            pipeline_state: pso,
            blur_pipeline_states: [blur_pso_u, blur_pso_v],

            direct_command_lists,
            copy_command_list,

            vertex_buffer,
            vertex_buffer_upload,
            vertex_buffer_view,

            quad_vertex_buffer,
            quad_vertex_buffer_upload,
            quad_vertex_buffer_view,

            depth_stencil,
            triangle_constant_buffer,
            triangle_cb_data,
            triangle_cb_mapped_data,
            ps_loop_count: 0,
            workload_constant_buffer,
            workload_cb_data,
            workload_cb_mapped_data,

            blur_workload_constant_buffer,
            blur_workload_cb_data,
            blur_workload_cb_mapped_data,
            blur_ps_loop_count: 0,
            blur_constant_buffer,

            frame_fence,
            render_fence,
            cross_adapter_fences,
            fence_events,

            current_present_fence_value: 1,
            current_render_fence_value: 1,
            current_cross_adapter_fence_value: cross_adapter_fence_value,
            frame_fence_values: [0; FRAMES_IN_FLIGHT],

            draw_times: [0; MOVING_AVERAGE_FRAME_COUNT],
            blur_times: [0; MOVING_AVERAGE_FRAME_COUNT],
            draw_time_moving_average: 0,
            blur_time_moving_average: 0,
        }
    }

    //     fn populate_command_lists(&mut self) {
    //         // Command list to render target the triangles on the primary adapter.
    //         self.populate_primary_adapter_direct_command_list();
    //         trace!("Populated direct command list on primary adapter");

    //         // Command list to copy the render target to the shared heap on the primary adapter.
    //         self.populate_copy_command_list();
    //         trace!("Populated copy command list");

    //         // Command list to blur the render target and present.
    //         self.populate_secondary_adapter_command_list();
    //         trace!("Populated direct command list on secondary adapter");
    //     }

    //     fn populate_secondary_adapter_command_list(&mut self) {
    //         let adapter_idx = 1usize; // secondary

    //         self.direct_command_allocators[adapter_idx][self.frame_index]
    //             .reset()
    //             .expect(
    //                 "Cannot reset direct command allocator on secondary adapter",
    //             );

    //         self.direct_command_lists[adapter_idx]
    //             .reset(
    //                 &self.direct_command_allocators[adapter_idx][self.frame_index],
    //                 Some(&self.blur_pipeline_states[0]),
    //             )
    //             .expect("Cannot reset direct command list on secondary adapter");

    //         if !self.cross_adapter_textures_supported {
    //             self.direct_command_lists[adapter_idx].resource_barrier(
    //                 slice::from_ref(&ResourceBarrier::transition(
    //                     &ResourceTransitionBarrier::default()
    //                         .set_resource(
    //                             &self.secondary_adapter_textures[self.frame_index],
    //                         )
    //                         .set_state_before(ResourceStates::PixelShaderResource)
    //                         .set_state_after(ResourceStates::CopyDest),
    //                 )),
    //             );

    //             let secondary_adapter_texture_desc =
    //                 self.secondary_adapter_textures[self.frame_index].get_desc();

    //             let (texture_layout, _, _, _) = self.devices[adapter_idx]
    //                 .get_copyable_footprints(
    //                     &secondary_adapter_texture_desc,
    //                     Elements(0),
    //                     Elements(1),
    //                     Bytes(0),
    //                 );

    //             // {
    //             //     let dest_resource_desc = self.secondary_adapter_textures
    //             //         [self.frame_index]
    //             //         .get_desc();
    //             //     trace!(
    //             //         "About to copy texture region to resource {}: {:?}",
    //             //         &self.secondary_adapter_textures[self.frame_index]
    //             //             .get_name()
    //             //             .expect("Cannot get resource name"),
    //             //         &dest_resource_desc
    //             //     );

    //             //     let src_resource_desc = self.secondary_adapter_textures
    //             //         [self.frame_index]
    //             //         .get_desc();
    //             //     trace!(
    //             //         "About to copy texture region from resource {}: {:?}",
    //             //         &self.cross_process_resources[adapter_idx]
    //             //             [self.frame_index]
    //             //             .get_name()
    //             //             .expect("Cannot get resource name"),
    //             //         &src_resource_desc
    //             //     );
    //             // }

    //             let dest = TextureCopyLocation::new(
    //                 &self.secondary_adapter_textures[self.frame_index],
    //                 &TextureLocationType::SubresourceIndex(Elements(0)),
    //             );

    //             let src = TextureCopyLocation::new(
    //                 &self.cross_process_resources[adapter_idx][self.frame_index],
    //                 &TextureLocationType::PlacedFootprint(texture_layout[0]),
    //             );

    //             let resource_box = Box::default()
    //                 .set_left(Elements(0))
    //                 .set_top(Elements(0))
    //                 .set_right(Elements::from(WINDOW_WIDTH))
    //                 .set_bottom(Elements::from(WINDOW_HEIGHT));

    //             self.direct_command_lists[adapter_idx].copy_texture_region(
    //                 &dest,
    //                 Elements(0),
    //                 Elements(0),
    //                 Elements(0),
    //                 &src,
    //                 Some(&resource_box),
    //             );

    //             self.direct_command_lists[adapter_idx].resource_barrier(
    //                 slice::from_ref(&ResourceBarrier::transition(
    //                     &ResourceTransitionBarrier::default()
    //                         .set_resource(
    //                             &self.secondary_adapter_textures[self.frame_index],
    //                         )
    //                         .set_state_before(ResourceStates::CopyDest)
    //                         .set_state_after(ResourceStates::PixelShaderResource),
    //                 )),
    //             );
    //         }

    //         let timestamp_heap_index = 2 * self.frame_index;
    //         self.direct_command_lists[adapter_idx].end_query(
    //             &self.query_heaps[adapter_idx],
    //             QueryType::Timestamp,
    //             Elements::from(timestamp_heap_index),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .set_graphics_root_signature(&self.blur_root_signature);

    //         self.direct_command_lists[adapter_idx]
    //             .set_descriptor_heaps(slice::from_ref(&self.cbv_srv_heap));

    //         self.direct_command_lists[adapter_idx]
    //             .set_viewports(slice::from_ref(&self.viewport));

    //         self.direct_command_lists[adapter_idx]
    //             .set_scissor_rects(slice::from_ref(&self.scissor_rect));

    //         self.direct_command_lists[adapter_idx].resource_barrier(
    //             slice::from_ref(&ResourceBarrier::transition(
    //                 &ResourceTransitionBarrier::default()
    //                     .set_resource(&self.intermediate_blur_render_target)
    //                     .set_state_before(ResourceStates::PixelShaderResource)
    //                     .set_state_after(ResourceStates::RenderTarget),
    //             )),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .set_primitive_topology(PrimitiveTopology::TriangleStrip);
    //         self.direct_command_lists[adapter_idx].set_vertex_buffers(
    //             Elements(0),
    //             slice::from_ref(&self.quad_vertex_buffer_view),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .set_graphics_root_constant_buffer_view(
    //                 Elements(0),
    //                 self.blur_constant_buffer.get_gpu_virtual_address(),
    //             );

    //         self.direct_command_lists[adapter_idx]
    //             .set_graphics_root_constant_buffer_view(
    //                 Elements(2),
    //                 GpuVirtualAddress(
    //                     self.blur_workload_constant_buffer
    //                         .get_gpu_virtual_address()
    //                         .0
    //                         + (self.frame_index
    //                             * size_of::<WorkloadConstantBufferData>())
    //                             as u64,
    //                 ),
    //             );

    //         // Draw the fullscreen quad - Blur pass #1.
    //         {
    //             let srv_handle = self
    //                 .cbv_srv_heap
    //                 .get_gpu_descriptor_handle_for_heap_start()
    //                 .advance(Elements::from(self.frame_index));

    //             self.direct_command_lists[adapter_idx]
    //                 .set_graphics_root_descriptor_table(Elements(1), srv_handle);

    //             let rtv_handle = self.rtv_heaps[adapter_idx]
    //                 .get_cpu_descriptor_handle_for_heap_start()
    //                 .advance(Elements::from(FRAMES_IN_FLIGHT));

    //             self.direct_command_lists[adapter_idx].set_render_targets(
    //                 slice::from_ref(&rtv_handle),
    //                 false,
    //                 None,
    //             );

    //             self.direct_command_lists[adapter_idx].draw_instanced(
    //                 Elements(4),
    //                 Elements(1),
    //                 Elements(0),
    //                 Elements(0),
    //             );
    //         }

    //         // Draw the fullscreen quad - Blur pass #2.
    //         {
    //             self.direct_command_lists[adapter_idx]
    //                 .set_pipeline_state(&self.blur_pipeline_states[1]);

    //             let barriers = [
    //                 ResourceBarrier::transition(
    //                     &ResourceTransitionBarrier::default()
    //                         .set_resource(
    //                             &self.render_targets[adapter_idx][self.frame_index],
    //                         )
    //                         .set_state_before(ResourceStates::CommonOrPresent)
    //                         .set_state_after(ResourceStates::RenderTarget),
    //                 ),
    //                 ResourceBarrier::transition(
    //                     &ResourceTransitionBarrier::default()
    //                         .set_resource(&self.intermediate_blur_render_target)
    //                         .set_state_before(ResourceStates::RenderTarget)
    //                         .set_state_after(ResourceStates::PixelShaderResource),
    //                 ),
    //             ];
    //             self.direct_command_lists[adapter_idx].resource_barrier(&barriers);

    //             let srv_handle = self
    //                 .cbv_srv_heap
    //                 .get_gpu_descriptor_handle_for_heap_start()
    //                 .advance(Elements::from(FRAMES_IN_FLIGHT));

    //             self.direct_command_lists[adapter_idx]
    //                 .set_graphics_root_descriptor_table(Elements(1), srv_handle);

    //             let rtv_handle = self.rtv_heaps[adapter_idx]
    //                 .get_cpu_descriptor_handle_for_heap_start()
    //                 .advance(Elements::from(self.frame_index));

    //             self.direct_command_lists[adapter_idx].set_render_targets(
    //                 slice::from_ref(&rtv_handle),
    //                 false,
    //                 None,
    //             );

    //             self.direct_command_lists[adapter_idx].draw_instanced(
    //                 Elements(4),
    //                 Elements(1),
    //                 Elements(0),
    //                 Elements(0),
    //             );
    //         }

    //         self.direct_command_lists[adapter_idx].resource_barrier(
    //             slice::from_ref(&ResourceBarrier::transition(
    //                 &ResourceTransitionBarrier::default()
    //                     .set_resource(
    //                         &self.render_targets[adapter_idx][self.frame_index],
    //                     )
    //                     .set_state_before(ResourceStates::RenderTarget)
    //                     .set_state_after(ResourceStates::CommonOrPresent),
    //             )),
    //         );

    //         self.direct_command_lists[adapter_idx].end_query(
    //             &self.query_heaps[adapter_idx],
    //             QueryType::Timestamp,
    //             Elements::from(timestamp_heap_index + 1),
    //         );

    //         self.direct_command_lists[adapter_idx].resolve_query_data(
    //             &self.query_heaps[adapter_idx],
    //             QueryType::Timestamp,
    //             Elements::from(timestamp_heap_index),
    //             Elements(2),
    //             &self.timestamp_result_buffers[adapter_idx],
    //             Bytes::from(timestamp_heap_index * size_of::<u64>()),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .close()
    //             .expect("Cannot close command list on secondary adapter");
    //     }

    //     fn populate_copy_command_list(&mut self) {
    //         let adapter_idx = 0usize;
    //         self.copy_allocators[self.frame_index]
    //             .reset()
    //             .expect("Cannot reset copy command allocator on primary device");
    //         self.copy_command_list
    //             .reset(&self.copy_allocators[self.frame_index], None)
    //             .expect("Cannot reset copy command list on primary adapter");
    //         if self.cross_adapter_textures_supported {
    //             self.copy_command_list.copy_resource(
    //                 &self.cross_process_resources[adapter_idx][self.frame_index],
    //                 &self.render_targets[adapter_idx][self.frame_index],
    //             );
    //         } else {
    //             let render_target_desc =
    //                 self.render_targets[adapter_idx][self.frame_index].get_desc();
    //             let (render_target_layout, _, _, _) = self.devices[adapter_idx]
    //                 .get_copyable_footprints(
    //                     &render_target_desc,
    //                     Elements(0),
    //                     Elements(1),
    //                     Bytes(0),
    //                 );

    //             let dest = TextureCopyLocation::new(
    //                 &self.cross_process_resources[adapter_idx][self.frame_index],
    //                 &TextureLocationType::PlacedFootprint(render_target_layout[0]),
    //             );

    //             let src = TextureCopyLocation::new(
    //                 &self.render_targets[adapter_idx][self.frame_index],
    //                 &TextureLocationType::SubresourceIndex(Elements(0)),
    //             );

    //             let resource_box = Box::default()
    //                 .set_left(Elements(0))
    //                 .set_top(Elements(0))
    //                 .set_right(Elements::from(WINDOW_WIDTH))
    //                 .set_bottom(Elements::from(WINDOW_HEIGHT));

    //             self.copy_command_list.copy_texture_region(
    //                 &dest,
    //                 Elements(0),
    //                 Elements(0),
    //                 Elements(0),
    //                 &src,
    //                 Some(&resource_box),
    //             );
    //         }
    //         self.copy_command_list
    //             .close()
    //             .expect("Cannot close copy command list");
    //     }

    //     fn populate_primary_adapter_direct_command_list(&mut self) {
    //         let adapter_idx = 0usize; // primary

    //         self.direct_command_allocators[adapter_idx][self.frame_index]
    //             .reset()
    //             .expect("Cannot reset direct command allocator on primary device");

    //         self.direct_command_lists[adapter_idx]
    //             .reset(
    //                 &self.direct_command_allocators[adapter_idx][self.frame_index],
    //                 Some(&self.pipeline_state),
    //             )
    //             .expect("Cannot reset direct command list on primary adapter");

    //         let timestamp_heap_index = 2 * self.frame_index;
    //         self.direct_command_lists[adapter_idx].end_query(
    //             &self.query_heaps[adapter_idx],
    //             QueryType::Timestamp,
    //             Elements::from(timestamp_heap_index),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .set_graphics_root_signature(&self.root_signature);

    //         self.direct_command_lists[adapter_idx]
    //             .set_viewports(slice::from_ref(&self.viewport));

    //         self.direct_command_lists[adapter_idx]
    //             .set_scissor_rects(slice::from_ref(&self.scissor_rect));

    //         self.direct_command_lists[adapter_idx].resource_barrier(
    //             slice::from_ref(&ResourceBarrier::transition(
    //                 &ResourceTransitionBarrier::default()
    //                     .set_resource(
    //                         &self.render_targets[adapter_idx][self.frame_index],
    //                     )
    //                     .set_state_before(ResourceStates::CommonOrPresent)
    //                     .set_state_after(ResourceStates::RenderTarget),
    //             )),
    //         );

    //         let rtv_handle = self.rtv_heaps[adapter_idx]
    //             .get_cpu_descriptor_handle_for_heap_start()
    //             .advance(Elements::from(self.frame_index));

    //         let dsv_handle =
    //             self.dsv_heap.get_cpu_descriptor_handle_for_heap_start();

    //         self.direct_command_lists[adapter_idx].set_render_targets(
    //             slice::from_ref(&rtv_handle),
    //             false,
    //             Some(dsv_handle),
    //         );

    //         self.direct_command_lists[adapter_idx].clear_render_target_view(
    //             rtv_handle,
    //             CLEAR_COLOR,
    //             &[],
    //         );
    //         self.direct_command_lists[adapter_idx].clear_depth_stencil_view(
    //             dsv_handle,
    //             ClearFlags::Depth,
    //             1.,
    //             0,
    //             &[],
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .set_primitive_topology(PrimitiveTopology::TriangleStrip);
    //         self.direct_command_lists[adapter_idx].set_vertex_buffers(
    //             Elements(0),
    //             slice::from_ref(&self.vertex_buffer_view),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .set_graphics_root_constant_buffer_view(
    //                 Elements(1),
    //                 GpuVirtualAddress(
    //                     self.workload_constant_buffer.get_gpu_virtual_address().0
    //                         + (self.frame_index
    //                             * size_of::<WorkloadConstantBufferData>())
    //                             as u64,
    //                 ),
    //             );

    //         let cb_virtual_address =
    //             self.triangle_constant_buffer.get_gpu_virtual_address();
    //         for tri_idx in 0..self.triangle_count {
    //             let cb_location = GpuVirtualAddress(
    //                 cb_virtual_address.0
    //                     + (self.frame_index
    //                         * MAX_TRIANGLE_COUNT as usize
    //                         * size_of::<SceneConstantBuffer>())
    //                         as u64
    //                     + (tri_idx as usize * size_of::<SceneConstantBuffer>())
    //                         as u64,
    //             );

    //             self.direct_command_lists[adapter_idx]
    //                 .set_graphics_root_constant_buffer_view(
    //                     Elements(0),
    //                     cb_location,
    //                 );

    //             self.direct_command_lists[adapter_idx].draw_instanced(
    //                 Elements(3),
    //                 Elements(1),
    //                 Elements(0),
    //                 Elements(0),
    //             );
    //         }

    //         self.direct_command_lists[adapter_idx].resource_barrier(
    //             slice::from_ref(&ResourceBarrier::transition(
    //                 &ResourceTransitionBarrier::default()
    //                     .set_resource(
    //                         &self.render_targets[adapter_idx][self.frame_index],
    //                     )
    //                     .set_state_before(ResourceStates::RenderTarget)
    //                     .set_state_after(ResourceStates::CommonOrPresent),
    //             )),
    //         );

    //         self.direct_command_lists[adapter_idx].end_query(
    //             &self.query_heaps[adapter_idx],
    //             QueryType::Timestamp,
    //             Elements::from(timestamp_heap_index + 1),
    //         );

    //         self.direct_command_lists[adapter_idx].resolve_query_data(
    //             &self.query_heaps[adapter_idx],
    //             QueryType::Timestamp,
    //             Elements::from(timestamp_heap_index),
    //             Elements(2),
    //             &self.timestamp_result_buffers[adapter_idx],
    //             Bytes::from(timestamp_heap_index * size_of::<u64>()),
    //         );

    //         self.direct_command_lists[adapter_idx]
    //             .close()
    //             .expect("Cannot close command list");
    //     }

    //     fn render(&mut self) {
    //         self.populate_command_lists();

    //         self.execute_command_lists();

    //         self.swapchain.present(1, 0).expect("Cannot present");

    //         self.direct_command_queues[1]
    //             .signal(&self.frame_fence, self.current_present_fence_value)
    //             .expect("Cannot signal on queue");

    //         self.frame_fence_values[self.frame_index] =
    //             self.current_present_fence_value;
    //         self.current_present_fence_value += 1;

    //         self.move_to_next_frame();
    //     }

    //     fn move_to_next_frame(&mut self) {
    //         {
    //             self.frame_index =
    //                 self.swapchain.get_current_back_buffer_index().0 as usize;

    //             let completed_fence_value = self.frame_fence.get_completed_value();
    //             if completed_fence_value < self.frame_fence_values[self.frame_index]
    //             {
    //                 self.frame_fence
    //                     .set_event_on_completion(
    //                         self.frame_fence_values[self.frame_index],
    //                         &self.fence_events[1],
    //                     )
    //                     .expect("Cannot set fence event");

    //                 self.fence_events[1].wait();
    //             }
    //         }
    //     }

    //     fn execute_command_lists(&mut self) {
    //         {
    //             self.direct_command_queues[0].execute_command_lists(
    //                 slice::from_ref(&self.direct_command_lists[0]),
    //             );
    //             self.direct_command_queues[0]
    //                 .signal(&self.render_fence, self.current_render_fence_value)
    //                 .expect("Cannot signal direct command queue 0");
    //         }

    //         {
    //             self.copy_command_queue
    //                 .wait(&self.render_fence, self.current_render_fence_value)
    //                 .expect("Cannot wait on fence");

    //             self.current_render_fence_value += 1;

    //             self.copy_command_queue
    //                 .execute_command_lists(slice::from_ref(
    //                     &self.copy_command_list,
    //                 ));
    //             self.copy_command_queue
    //                 .signal(
    //                     &self.cross_adapter_fences[0],
    //                     self.current_cross_adapter_fence_value,
    //                 )
    //                 .expect("Cannot signal copy command queue");
    //         }
    //         {
    //             self.direct_command_queues[1]
    //                 .wait(
    //                     &self.cross_adapter_fences[1],
    //                     self.current_cross_adapter_fence_value,
    //                 )
    //                 .expect("Cannot wait on fence");

    //             self.current_cross_adapter_fence_value += 1;

    //             self.direct_command_queues[1].execute_command_lists(
    //                 slice::from_ref(&self.direct_command_lists[1]),
    //             );
    //         }
    //     }

    //     fn update(&mut self) {
    //         self.get_timestamps_data();
    //         trace!("Got timestamp data");

    //         self.adjust_workload_sizes();
    //         trace!("Adjusted workloads");

    //         self.update_workload_constant_buffers();
    //         trace!("Updated workload constant buffers");

    //         self.update_scene_constant_buffer();
    //         trace!("Updated scene constant buffer");
    //     }

    //     fn update_scene_constant_buffer(&mut self) {
    //         let offset_bounds = 2.5f32;
    //         let mut rng = rand::thread_rng();
    //         for tri_idx in 0..self.triangle_count as usize {
    //             self.triangle_cb_data[tri_idx].offset.x +=
    //                 self.triangle_cb_data[tri_idx].velocity.x;

    //             if self.triangle_cb_data[tri_idx].offset.x > offset_bounds {
    //                 self.triangle_cb_data[tri_idx].velocity.x =
    //                     rng.gen_range(0.01..0.02);
    //                 self.triangle_cb_data[tri_idx].offset.x = -offset_bounds;
    //             }
    //         }
    //         let dest = unsafe {
    //             self.triangle_cb_mapped_data.offset(
    //                 self.frame_index as isize
    //                     * MAX_TRIANGLE_COUNT as isize
    //                     * size_of::<SceneConstantBuffer>() as isize,
    //             ) as *mut SceneConstantBuffer
    //         };
    //         unsafe {
    //             copy_nonoverlapping(
    //                 self.triangle_cb_data.as_ptr(),
    //                 dest,
    //                 self.triangle_count as usize,
    //             )
    //         };
    //     }

    //     fn update_workload_constant_buffers(&mut self) {
    //         {
    //             let workload_dst = unsafe {
    //                 self.workload_cb_mapped_data.add(self.frame_index)
    //                     as *mut WorkloadConstantBufferData
    //             };

    //             self.workload_cb_data.loop_count = self.ps_loop_count;
    //             let workload_src =
    //                 &self.workload_cb_data as *const WorkloadConstantBufferData;

    //             unsafe {
    //                 copy_nonoverlapping(workload_src, workload_dst, 1);
    //             }

    //             let blur_workload_dst = unsafe {
    //                 self.blur_workload_cb_mapped_data.add(self.frame_index)
    //                     as *mut WorkloadConstantBufferData
    //             };

    //             self.blur_workload_cb_data.loop_count = self.blur_ps_loop_count;
    //             let blur_workload_src = &self.blur_workload_cb_data
    //                 as *const WorkloadConstantBufferData;

    //             unsafe {
    //                 copy_nonoverlapping(blur_workload_src, blur_workload_dst, 1);
    //             }
    //         }
    //     }

    //     fn adjust_workload_sizes(&mut self) {
    //         self.frames_since_last_update += 1;
    //         if self.frames_since_last_update > MOVING_AVERAGE_FRAME_COUNT as u32 {
    //             self.draw_time_moving_average = 0;
    //             self.blur_time_moving_average = 0;

    //             for frame_idx in 0..MOVING_AVERAGE_FRAME_COUNT {
    //                 self.draw_time_moving_average += self.draw_times[frame_idx];
    //                 self.blur_time_moving_average += self.blur_times[frame_idx];
    //             }

    //             self.draw_time_moving_average /= MOVING_AVERAGE_FRAME_COUNT as u64;
    //             self.blur_time_moving_average /= MOVING_AVERAGE_FRAME_COUNT as u64;

    //             self.frames_since_last_update = 0;

    //             if ALLOW_SHADER_DYNAMIC_WORKLOAD {
    //                 let desired_blur_ps_time_us = 20000u64;

    //                 if self.blur_time_moving_average < desired_blur_ps_time_us
    //                     || self.blur_ps_loop_count != 0
    //                 {
    //                     let time_delta = (desired_blur_ps_time_us as f32
    //                         - self.blur_time_moving_average as f32)
    //                         / self.blur_time_moving_average as f32;

    //                     if !(-0.05..=0.01).contains(&time_delta) {
    //                         let step_size =
    //                             1f32.max(self.blur_ps_loop_count as f32);
    //                         let loop_count_delta = (step_size * time_delta) as i32;
    //                         if loop_count_delta > 0 {
    //                             self.blur_ps_loop_count += loop_count_delta as u32;
    //                         } else {
    //                             self.blur_ps_loop_count -= loop_count_delta as u32;
    //                         }
    //                     }
    //                 }
    //             }

    //             let desired_draw_ps_time_us = self.blur_time_moving_average
    //                 + (self.blur_time_moving_average as f32 * 0.1) as u64;

    //             let time_delta = (desired_draw_ps_time_us as f32
    //                 - self.draw_time_moving_average as f32)
    //                 / self.draw_time_moving_average as f32;

    //             if !(-0.1..=0.01).contains(&time_delta) {
    //                 if ALLOW_DRAW_DYNAMIC_WORKLOAD {
    //                     let step_size = 1f32.max(self.triangle_count as f32);
    //                     self.triangle_count = min(
    //                         self.triangle_count + (step_size * time_delta) as u32,
    //                         MAX_TRIANGLE_COUNT,
    //                     );
    //                 } else if ALLOW_SHADER_DYNAMIC_WORKLOAD {
    //                     let step_size = 1f32.max(self.ps_loop_count as f32);
    //                     let loop_count_delta = (step_size * time_delta) as i32;
    //                     if loop_count_delta > 0 {
    //                         self.ps_loop_count += loop_count_delta as u32;
    //                     } else {
    //                         self.ps_loop_count -= loop_count_delta as u32;
    //                     }
    //                 }
    //             }
    //         }

    //         if self.frames_since_last_update % TIMESTAMP_PRINT_FREQUENCY as u32 == 0
    //         {
    //             trace!("{} triangles; render: {} us, ps loop count: {}; blur: {} us, ps loop count: {} ",
    //                 self.triangle_count, self.draw_time_moving_average, self.ps_loop_count, self.blur_time_moving_average, self.blur_ps_loop_count
    //             );
    //         }
    //     }

    //     fn get_timestamps_data(&mut self) {
    //         let oldest_frame_index = self.frame_index;
    //         assert!(
    //             self.frame_fence_values[oldest_frame_index]
    //                 <= self.frame_fence.get_completed_value()
    //         );
    //         let empty_range = Range::default();
    //         let moving_average = [&mut self.draw_times, &mut self.blur_times];
    //         for device_idx in 0..DEVICE_COUNT {
    //             let range_begin = Bytes::from(
    //                 2 * oldest_frame_index as u32 * size_of::<u64>() as u32,
    //             );
    //             let read_range = Range::default()
    //                 .set_begin(range_begin)
    //                 .set_end(range_begin + Bytes(2 * size_of::<u64>() as u64));

    //             let mapped_data = self.timestamp_result_buffers[device_idx]
    //                 .map(Elements(0), Some(&read_range))
    //                 .expect("Cannot map timestamp result buffer")
    //                 as *mut u64;

    //             let (begin, end) = unsafe {
    //                 (
    //                     *mapped_data.offset(read_range.get_begin().0 as isize),
    //                     *mapped_data.offset(read_range.get_begin().0 as isize + 1),
    //                 )
    //             };

    //             let timestamp_delta = end - begin;
    //             self.timestamp_result_buffers[device_idx]
    //                 .unmap(0, Some(&empty_range));

    //             let gpu_time_ms = (timestamp_delta * 1000000)
    //                 / self.direct_command_queue_timestamp_frequencies[device_idx];

    //             moving_average[device_idx][self.current_times_index] = gpu_time_ms;
    //         }

    //         self.current_times_index =
    //             (self.current_times_index + 1) % MOVING_AVERAGE_FRAME_COUNT;
    //     }
    // }

    // fn create_fences(
    //     devices: &[Device; DEVICE_COUNT],
    //     direct_command_queues: &[CommandQueue; DEVICE_COUNT],
    // ) -> (Fence, Fence, [Fence; DEVICE_COUNT], [Win32Event; 2], u64) {
    //     let frame_fence = devices[1]
    //         .create_fence(0, FenceFlags::None)
    //         .expect("Cannot create fence");

    //     let render_fence = devices[0]
    //         .create_fence(0, FenceFlags::None)
    //         .expect("Cannot create fence");

    //     let cross_adapter_fence_primary = devices[0]
    //         .create_fence(0, FenceFlags::Shared | FenceFlags::CrossAdapter)
    //         .expect("Cannot create fence");

    //     let fence_handle = devices[0]
    //         .create_shared_handle(
    //             &cross_adapter_fence_primary.clone().into(),
    //             "CrossAdapterFence",
    //         )
    //         .expect("Cannot create shared handle for cross adapter fence");

    //     let cross_adapter_fence_secondary = devices[1]
    //         .open_shared_fence_handle(fence_handle)
    //         .expect("Cannot open shared fence handle");
    //     fence_handle.close();

    //     let cross_adapter_fences =
    //         [cross_adapter_fence_primary, cross_adapter_fence_secondary];
    //     let mut cross_adapter_fence_value = 1;

    //     let mut fence_events: [MaybeUninit<Win32Event>; 2] =
    //         unsafe { MaybeUninit::uninit().assume_init() };

    //     for device_idx in 0..DEVICE_COUNT {
    //         fence_events[device_idx] = MaybeUninit::new(Win32Event::default());

    //         direct_command_queues[device_idx]
    //             .signal(
    //                 &cross_adapter_fences[device_idx],
    //                 cross_adapter_fence_value,
    //             )
    //             .expect("Cannot signal command queue");

    //         cross_adapter_fences[device_idx]
    //             .set_event_on_completion(cross_adapter_fence_value, unsafe {
    //                 fence_events[device_idx].assume_init_ref()
    //             })
    //             .expect("Cannot set event on fence");

    //         unsafe { fence_events[device_idx].assume_init_ref() }.wait();

    //         cross_adapter_fence_value += 1;
    //     }

    //     (
    //         frame_fence,
    //         render_fence,
    //         cross_adapter_fences,
    //         unsafe { std::mem::transmute(fence_events) },
    //         cross_adapter_fence_value,
    //     )
    // }

    // fn create_blur_constant_buffer(devices: &[Device; DEVICE_COUNT]) -> Resource {
    //     let blur_constant_buffer = devices[1]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Upload),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Buffer)
    //                 .set_layout(TextureLayout::RowMajor)
    //                 .set_width(size_of::<BlurConstantBufferData>().into()),
    //             ResourceStates::GenericRead,
    //             None,
    //         )
    //         .expect("Cannot create blur constant buffer");
    //     blur_constant_buffer
    //         .set_name("Blur constant buffer")
    //         .expect("Cannot set name on resource");

    //     let buffer_data = BlurConstantBufferData {
    //         texture_dimensions: Vec2::new(
    //             WINDOW_WIDTH as f32,
    //             WINDOW_HEIGHT as f32,
    //         ),
    //         offset: 0.5,
    //         padding: [0.; 61],
    //     };

    //     let mapped_data = blur_constant_buffer
    //         .map(Elements(0), None)
    //         .expect("Cannot map blur_constant_buffer");
    //     unsafe {
    //         copy_nonoverlapping(
    //             &buffer_data,
    //             mapped_data as *mut BlurConstantBufferData,
    //             1,
    //         );
    //     }
    //     blur_constant_buffer.unmap(0, None);

    //     blur_constant_buffer
    // }

    // fn create_blur_workload_constant_buffer(
    //     devices: &[Device; DEVICE_COUNT],
    //     blur_ps_loop_count: u32,
    // ) -> (Resource, WorkloadConstantBufferData, *mut u8) {
    //     let blur_workload_constant_buffer_size = Bytes::from(
    //         size_of::<WorkloadConstantBufferData>() as u32
    //             * FRAMES_IN_FLIGHT as u32,
    //     );
    //     let blur_workload_constant_buffer = devices[1]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Upload),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Buffer)
    //                 .set_layout(TextureLayout::RowMajor)
    //                 .set_width(blur_workload_constant_buffer_size.0.into()),
    //             ResourceStates::GenericRead,
    //             None,
    //         )
    //         .expect("Cannot create blur workload constant buffer");
    //     blur_workload_constant_buffer
    //         .set_name("Blur workload constant buffer")
    //         .expect("Cannot set name on resource");
    //     let buffer_data = WorkloadConstantBufferData {
    //         loop_count: blur_ps_loop_count,
    //         padding: [0.; 63],
    //     };
    //     let mapped_data = blur_workload_constant_buffer
    //         .map(Elements(0), None)
    //         .expect("Cannot map blur_workload_constant_buffer");
    //     unsafe {
    //         copy_nonoverlapping(
    //             &buffer_data,
    //             mapped_data as *mut WorkloadConstantBufferData,
    //             1,
    //         );
    //     }

    //     (blur_workload_constant_buffer, buffer_data, mapped_data)
    // }

    // fn create_workload_constant_buffer(
    //     devices: &[Device; DEVICE_COUNT],
    //     ps_loop_count: u32,
    // ) -> (Resource, WorkloadConstantBufferData, *mut u8) {
    //     let workload_constant_buffer_size = Bytes::from(
    //         size_of::<WorkloadConstantBufferData>() as u32
    //             * FRAMES_IN_FLIGHT as u32,
    //     );
    //     let workload_constant_buffer = devices[0]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Upload),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Buffer)
    //                 .set_layout(TextureLayout::RowMajor)
    //                 .set_width(workload_constant_buffer_size.0.into()),
    //             ResourceStates::GenericRead,
    //             None,
    //         )
    //         .expect("Cannot create workload constant buffer");
    //     workload_constant_buffer
    //         .set_name("Workload constant buffer")
    //         .expect("Cannot set name on resource");
    //     let buffer_data = WorkloadConstantBufferData {
    //         loop_count: ps_loop_count,
    //         padding: [0.; 63],
    //     };
    //     let mapped_data = workload_constant_buffer
    //         .map(Elements(0), None)
    //         .expect("Cannot map workload_constant_buffer buffer");
    //     unsafe {
    //         copy_nonoverlapping(
    //             &buffer_data,
    //             mapped_data as *mut WorkloadConstantBufferData,
    //             1,
    //         );
    //     }

    //     (workload_constant_buffer, buffer_data, mapped_data)
    // }

    // fn create_triangle_constant_buffer(
    //     devices: &[Device; DEVICE_COUNT],
    // ) -> (Resource, Vec<SceneConstantBuffer>, *mut u8) {
    //     let constant_buffer_size = Bytes::from(
    //         size_of::<SceneConstantBuffer>() as u32
    //             * MAX_TRIANGLE_COUNT
    //             * FRAMES_IN_FLIGHT as u32,
    //     );

    //     let constant_buffer = devices[0]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Upload),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Buffer)
    //                 .set_layout(TextureLayout::RowMajor)
    //                 .set_width(constant_buffer_size.0.into()),
    //             ResourceStates::GenericRead,
    //             None,
    //         )
    //         .expect("Cannot create constant buffer");
    //     constant_buffer
    //         .set_name("Constant buffer")
    //         .expect("Cannot set name on resource");

    //     let camera = Camera::default();

    //     let world = Mat4::identity();
    //     let view = make_view_matrix(camera.position, camera.look_at);
    //     let proj = make_projection_matrix(&camera);

    //     let mut rng = rand::thread_rng();
    //     let mut constant_buffer_data = vec![];

    //     for tri_idx in 0..MAX_TRIANGLE_COUNT as usize {
    //         constant_buffer_data.push(SceneConstantBuffer {
    //             velocity: Vec4::new(rng.gen_range(0.01..0.02), 0., 0., 0.),
    //             offset: Vec4::new(
    //                 rng.gen_range(-5. ..-1.5),
    //                 rng.gen_range(-1. ..1.),
    //                 rng.gen_range(0. ..2.),
    //                 0.,
    //             ),

    //             color: Vec4::new(
    //                 rng.gen_range(0.5..1.),
    //                 rng.gen_range(0.5..1.),
    //                 rng.gen_range(0.5..1.),
    //                 1.,
    //             ),

    //             projection: proj * view * world,
    //             padding: [0.; 36],
    //         });
    //     }
    //     let mapped_data = constant_buffer
    //         .map(Elements(0), None)
    //         .expect("Cannot map constant buffer");
    //     unsafe {
    //         copy_nonoverlapping(
    //             constant_buffer_data.as_ptr(),
    //             mapped_data as *mut SceneConstantBuffer,
    //             MAX_TRIANGLE_COUNT as usize,
    //         );
    //     }

    //     (constant_buffer, constant_buffer_data, mapped_data)
    // }

    // fn create_depth_stencil(
    //     devices: &[Device; DEVICE_COUNT],
    //     dsv_heap: &DescriptorHeap,
    // ) -> Resource {
    //     let depth_stencil_desc = DepthStencilViewDesc::default()
    //         .set_format(DxgiFormat::D32_Float)
    //         .set_view_dimension(DsvDimension::Texture2D);
    //     let clear_value = ClearValue::default()
    //         .set_format(DxgiFormat::D32_Float)
    //         .set_depth_stencil(
    //             &DepthStencilValue::default().set_depth(1.).set_stencil(0),
    //         );
    //     let depth_stencil = devices[0]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Default),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Texture2D)
    //                 .set_format(DxgiFormat::D32_Float)
    //                 .set_width(WINDOW_WIDTH.into())
    //                 .set_height(WINDOW_HEIGHT.into())
    //                 .set_flags(ResourceFlags::AllowDepthStencil),
    //             ResourceStates::DepthWrite,
    //             Some(&clear_value),
    //         )
    //         .expect("Cannot create depth stencil on primary adapter");
    //     depth_stencil
    //         .set_name("Depth stencil")
    //         .expect("Cannot set name on resource");
    //     devices[0].create_depth_stencil_view(
    //         &depth_stencil,
    //         &depth_stencil_desc,
    //         dsv_heap.get_cpu_descriptor_handle_for_heap_start(),
    //     );
    //     depth_stencil
    // }

    // fn create_blur_vertex_buffer(
    //     devices: &[Device; DEVICE_COUNT],
    //     direct_command_lists: &[CommandList; DEVICE_COUNT],
    // ) -> (Resource, Resource, VertexBufferView) {
    //     let quad_vertices = [
    //         BlurVertex {
    //             position: Vec4::new(-1.0, -1.0, 0.0, 1.0),
    //             uv: Vec2::new(0.0, 0.0),
    //         },
    //         BlurVertex {
    //             position: Vec4::new(-1.0, 1.0, 0.0, 1.0),
    //             uv: Vec2::new(0.0, 1.0),
    //         },
    //         BlurVertex {
    //             position: Vec4::new(1.0, -1.0, 0.0, 1.0),
    //             uv: Vec2::new(1.0, 0.0),
    //         },
    //         BlurVertex {
    //             position: Vec4::new(1.0, 1.0, 0.0, 1.0),
    //             uv: Vec2::new(1.0, 1.0),
    //         },
    //     ];
    //     let quad_vertex_buffer_size =
    //         Bytes::from(quad_vertices.len() * size_of::<BlurVertex>());
    //     let quad_vertex_buffer = devices[1]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Default),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Buffer)
    //                 .set_layout(TextureLayout::RowMajor)
    //                 .set_width(quad_vertex_buffer_size.0.into()),
    //             ResourceStates::CopyDest,
    //             None,
    //         )
    //         .expect("Cannot create quad_vertex_buffer");
    //     quad_vertex_buffer
    //         .set_name("Quad vertex buffer")
    //         .expect("Cannot set name on resource");
    //     let quad_vertex_buffer_upload = devices[1]
    //         .create_committed_resource(
    //             &HeapProperties::default().set_type(HeapType::Upload),
    //             HeapFlags::None,
    //             &ResourceDesc::default()
    //                 .set_dimension(ResourceDimension::Buffer)
    //                 .set_layout(TextureLayout::RowMajor)
    //                 .set_width(quad_vertex_buffer_size.0.into()),
    //             ResourceStates::GenericRead,
    //             None,
    //         )
    //         .expect("Cannot create quad_vertex_buffer");
    //     quad_vertex_buffer_upload
    //         .set_name("Quad vertex buffer upload")
    //         .expect("Cannot set name on resource");
    //     let quad_vertex_data = SubresourceData::default()
    //         .set_data(&quad_vertices)
    //         .set_row_pitch(quad_vertex_buffer_size)
    //         .set_slice_pitch(quad_vertex_buffer_size);
    //     direct_command_lists[1]
    //         .update_subresources_heap_alloc(
    //             &quad_vertex_buffer,
    //             &quad_vertex_buffer_upload,
    //             Bytes(0),
    //             Elements(0),
    //             Elements(1),
    //             slice::from_ref(&quad_vertex_data),
    //         )
    //         .expect("Cannot upload quad_vertex buffer");
    //     trace!("Uploaded quad vertex buffer");
    //     direct_command_lists[1].resource_barrier(slice::from_ref(
    //         &ResourceBarrier::transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&quad_vertex_buffer)
    //                 .set_state_before(ResourceStates::CopyDest)
    //                 .set_state_after(ResourceStates::VertexAndConstantBuffer),
    //         ),
    //     ));
    //     let quad_vertex_buffer_view = VertexBufferView::default()
    //         .set_buffer_location(quad_vertex_buffer.get_gpu_virtual_address())
    //         .set_size_in_bytes(quad_vertex_buffer_size)
    //         .set_stride_in_bytes(Bytes::from(std::mem::size_of::<BlurVertex>()));
    //     (
    //         quad_vertex_buffer,
    //         quad_vertex_buffer_upload,
    //         quad_vertex_buffer_view,
    //     )
    // }

    fn create_vertex_buffer(
        device: &Device,
        direct_command_list: &CommandList,
    ) -> (Resource, Resource, VertexBufferView) {
        let triangle_vertices = [
            Vertex {
                position: Vec3::new(0., TRIANGLE_HALF_WIDTH, TRIANGLE_DEPTH),
            },
            Vertex {
                position: Vec3::new(
                    TRIANGLE_HALF_WIDTH,
                    -TRIANGLE_HALF_WIDTH,
                    TRIANGLE_DEPTH,
                ),
            },
            Vertex {
                position: Vec3::new(
                    -TRIANGLE_HALF_WIDTH,
                    -TRIANGLE_HALF_WIDTH,
                    TRIANGLE_DEPTH,
                ),
            },
        ];

        let vertex_buffer_size =
            Bytes::from(triangle_vertices.len() * size_of::<Vertex>());

        let vertex_buffer = devices[0]
            .create_committed_resource(
                &HeapProperties::default().set_type(HeapType::Default),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_layout(TextureLayout::RowMajor)
                    .set_width(vertex_buffer_size.0.into()),
                ResourceStates::CopyDest,
                None,
            )
            .expect("Cannot create vertex_buffer");
        vertex_buffer
            .set_name("Vertex buffer")
            .expect("Cannot set name on resource");

        let vertex_buffer_upload = devices[0]
            .create_committed_resource(
                &HeapProperties::default().set_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_layout(TextureLayout::RowMajor)
                    .set_width(vertex_buffer_size.0.into()),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create vertex_buffer");
        vertex_buffer_upload
            .set_name("Vertex buffer upload")
            .expect("Cannot set name on resource");

        let vertex_data = SubresourceData::default()
            .set_data(&triangle_vertices)
            .set_row_pitch(vertex_buffer_size)
            .set_slice_pitch(vertex_buffer_size);

        direct_command_lists[0]
            .update_subresources_heap_alloc(
                &vertex_buffer,
                &vertex_buffer_upload,
                Bytes(0),
                Elements(0),
                Elements(1),
                slice::from_ref(&vertex_data),
            )
            .expect("Cannot upload vertex buffer");
        trace!("Uploaded vertex buffer");

        direct_command_lists[0].resource_barrier(slice::from_ref(
            &ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&vertex_buffer)
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::VertexAndConstantBuffer),
            ),
        ));

        let vertex_buffer_view = VertexBufferView::default()
            .set_buffer_location(vertex_buffer.get_gpu_virtual_address())
            .set_stride_in_bytes(Bytes::from(std::mem::size_of::<Vertex>()))
            .set_size_in_bytes(vertex_buffer_size);
        trace!("Created primary adapter vertex buffer");

        (vertex_buffer, vertex_buffer_upload, vertex_buffer_view)
    }
}

fn create_command_list(
    device: &Device,
    frame_index: usize,
    direct_command_allocators: &[CommandAllocator],
    pso: &PipelineState,
) -> CommandList {
    device
        .create_command_list(
            CommandListType::Direct,
            &direct_command_allocators[0][frame_index],
            Some(pso),
            // None,
        )
        .expect("Cannot create direct command list")
}

fn create_pso(
    device: &Device,
    root_signature: &RootSignature,
) -> PipelineState {
    let vertex_shader = compile_shader(
        "VertexShader",
        &std::fs::read_to_string("assets/hm_shaders.hlsl")
            .expect("Cannot open vertex shader file"),
        "VShader",
        "vs_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile vertex shader");
    let pixel_shader = compile_shader(
        "PixelShader",
        &std::fs::read_to_string("assets/hm_shaders.hlsl")
            .expect("Cannot open pixel shader file"),
        "PShader",
        "ps_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile pixel shader");

    let input_layout = Vertex::make_desc();
    let vs_bytecode = ShaderBytecode::from_bytes(&vertex_shader);
    let ps_bytecode = ShaderBytecode::from_bytes(&pixel_shader);

    let pso_desc = GraphicsPipelineStateDesc::default()
        .set_input_layout(
            &InputLayoutDesc::default().from_input_layout(&input_layout),
        )
        .set_root_signature(root_signature)
        .set_vertex_shader_bytecode(&vs_bytecode)
        .set_pixel_shader_bytecode(&ps_bytecode)
        .set_rasterizer_state(&RasterizerDesc::default())
        .set_blend_state(&BlendDesc::default())
        .set_depth_stencil_state(&DepthStencilDesc::default())
        .set_primitive_topology_type(PrimitiveTopologyType::Triangle)
        .set_num_render_targets(Elements(1))
        .set_rtv_formats(&[DxgiFormat::R8G8B8A8_UNorm])
        .set_dsv_format(DxgiFormat::D32_Float);

    let pso = device
        .create_graphics_pipeline_state(&pso_desc)
        .expect("Cannot create PSO");
    pso.set_name("Main PSO").expect("Cannot set name on pso");
    trace!("Created PSO");

    pso
}

fn create_root_signature(device: &Device) -> RootSignature {
    let root_signature = {
        let root_parameters = [
            RootParameter::default()
                .set_parameter_type(RootParameterType::Cbv)
                .set_shader_visibility(ShaderVisibility::Vertex)
                .set_descriptor(
                    &RootDescriptor::default().set_shader_register(Elements(0)),
                ),
            RootParameter::default()
                .set_parameter_type(RootParameterType::Cbv)
                .set_shader_visibility(ShaderVisibility::Pixel)
                .set_descriptor(
                    &RootDescriptor::default().set_shader_register(Elements(1)),
                ),
        ];

        let root_signature_desc = VersionedRootSignatureDesc::default()
            .set_version(RootSignatureVersion::V1_1)
            .set_desc_1_1(
                &RootSignatureDesc::default()
                    .set_parameters(&root_parameters)
                    .set_flags(
                        RootSignatureFlags::AllowInputAssemblerInputLayout,
                    ),
            );

        let (serialized_signature, serialization_result) =
            RootSignature::serialize_versioned(&root_signature_desc);
        assert!(
            serialization_result.is_ok(),
            "Result: {}",
            &serialization_result.err().unwrap()
        );

        let root_signature = device
            .create_root_signature(
                0,
                &ShaderBytecode::from_bytes(serialized_signature.get_buffer()),
            )
            .expect("Cannot create root signature on device 0");
        root_signature
    };
    root_signature
}

fn create_shared_resource_descs(device: &Device) -> (Bytes, ResourceDesc) {
    let texture_size: Bytes;
    let cross_adapter_desc;

    let (layout, _, _, _) = device.get_copyable_footprints(
        &ResourceDesc::default()
            .set_dimension(ResourceDimension::Texture2D)
            .set_format(DxgiFormat::R8G8B8A8_UNorm)
            .set_width(WINDOW_WIDTH.into())
            .set_height(WINDOW_HEIGHT.into())
            .set_flags(ResourceFlags::AllowRenderTarget),
        0.into(),
        1.into(),
        0.into(),
    );

    texture_size = align_to_multiple(
        (layout[0].0.Footprint.RowPitch * layout[0].0.Footprint.Height) as u64,
        DEFAULT_RESOURCE_ALIGNMENT.0,
    )
    .into();

    cross_adapter_desc = ResourceDesc::default()
        .set_dimension(ResourceDimension::Buffer)
        .set_width(texture_size.0.into())
        .set_layout(TextureLayout::RowMajor)
        .set_flags(ResourceFlags::AllowCrossAdapter);

    (texture_size, cross_adapter_desc)
}

fn create_frame_resources(
    device: &Device,
    rtv_heap: &DescriptorHeap,
    swapchain: &DxgiSwapchain,
) -> (Vec<Resource>, Vec<CommandAllocator>) {
    let clear_value = ClearValue::default()
        .set_format(DxgiFormat::R8G8B8A8_UNorm)
        .set_color(CLEAR_COLOR);

    let render_target_desc = ResourceDesc::default()
        .set_dimension(ResourceDimension::Texture2D)
        .set_format(DxgiFormat::R8G8B8A8_UNorm)
        .set_width(WINDOW_WIDTH.into())
        .set_height(WINDOW_HEIGHT.into())
        .set_flags(ResourceFlags::AllowRenderTarget);

    let mut render_targets = vec![];

    for frame_idx in 0..FRAMES_IN_FLIGHT {
        render_targets.push(
            swapchain
                .get_buffer(frame_idx.into())
                .expect("Cannot get buffer from swapchain"),
        );
    }
    let mut direct_command_allocators = vec![];

    let mut rtv_handle = rtv_heap.get_cpu_descriptor_handle_for_heap_start();
    for frame_idx in 0..FRAMES_IN_FLIGHT {
        device
            .create_render_target_view(&render_targets[frame_idx], rtv_handle);

        rtv_handle = rtv_handle.advance(Elements(1));

        direct_command_allocators.push(
            device
                .create_command_allocator(CommandListType::Direct)
                .expect("Cannot create command allocator"),
        );
    }

    trace!("created command allocators");

    (render_targets, direct_command_allocators)
}

fn create_descriptor_heaps(
    device: &Device,
    swapchain: &DxgiSwapchain,
) -> (DescriptorHeap, DescriptorHeap, DescriptorHeap) {
    let num_descriptors = FRAMES_IN_FLIGHT + 1; // add space for the intermediate render target
    let mut rtv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_type(DescriptorHeapType::RTV)
                .set_num_descriptors(Elements::from(num_descriptors)),
        )
        .expect("Cannot create RTV heap");
    rtv_heap
        .set_name("RTV heap")
        .expect("Cannot set RTV heap name");

    let dsv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_type(DescriptorHeapType::DSV)
                .set_num_descriptors(Elements(1)),
        )
        .expect("Cannot create DSV heap");
    dsv_heap
        .set_name("DSV heap")
        .expect("Cannot set DSV heap name");

    let cbv_srv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_type(DescriptorHeapType::CBV_SRV_UAV)
                .set_num_descriptors(Elements::from(FRAMES_IN_FLIGHT + 1))
                .set_flags(DescriptorHeapFlags::ShaderVisible),
        )
        .expect("Cannot create CBV_SRV heap");
    cbv_srv_heap
        .set_name("CBV_SRV heap")
        .expect("Cannot set CBV_SRV heap name");

    (rtv_heap, dsv_heap, cbv_srv_heap)
}

fn create_swapchain(
    factory: DxgiFactory,
    command_queue: &CommandQueue,
    hwnd: *mut std::ffi::c_void,
) -> DxgiSwapchain {
    let swapchain_desc = DxgiSwapchainDesc::default()
        .set_width(WINDOW_WIDTH)
        .set_height(WINDOW_HEIGHT)
        .set_buffer_count(FRAMES_IN_FLIGHT.into());
    let swapchain = factory
        .create_swapchain(&command_queue, hwnd as *mut HWND__, &swapchain_desc)
        .expect("Cannot create swapchain");
    factory
        .make_window_association(
            hwnd,
            DxgiMakeWindowAssociationFlags::NoAltEnter,
        )
        .expect("Cannot make window association");
    swapchain
}

// fn create_device(factory: &DxgiFactory) -> Device {
//     let device;
//     if USE_WARP_ADAPTER {
//         let warp_adapter = factory
//             .enum_warp_adapter()
//             .expect("Cannot enum warp adapter");
//         device = Device::new(&warp_adapter)
//             .expect("Cannot create device on WARP adapter");
//     } else {
//         let hw_adapter = factory
//             .enum_adapters()
//             .expect("Cannot enumerate adapters")
//             .remove(0);
//         device = Device::new(&hw_adapter).expect("Cannot create device");
//     }
//     device
// }

fn get_hardware_adapter(factory: &DxgiFactory) -> DxgiAdapter {
    let mut adapters = factory
        .enum_adapters_by_gpu_preference(DxgiGpuPreference::HighPerformance)
        .expect("Cannot enumerate adapters");

    for adapter in &adapters {
        let desc = adapter.get_desc().expect("Cannot get adapter desc");
        info!("found adapter: {}", desc);
    }
    adapters.remove(0)
}

fn create_device(factory: &DxgiFactory) -> (Device, bool) {
    let adapter;
    if USE_WARP_ADAPTER {
        adapter = factory
            .enum_warp_adapter()
            .expect("Cannot enum warp adapter");
    } else {
        adapter = get_hardware_adapter(factory);
    }

    let adapter_desc = adapter.get_desc().expect("Cannot get adapter desc");

    info!("Enumerated adapter: \n\t{}", adapter_desc,);
    (
        Device::new(&adapter).unwrap_or_else(|_| {
            panic!("Cannot create device on adapter {}", adapter_desc)
        }),
        adapter_desc.is_software(),
    )
}

fn main() {
    //wait_for_debugger();
    let command_args = clap::App::new("InterprocessCommunicationSample")
        .arg(
            clap::Arg::with_name("breakonerr")
                .short("b")
                .takes_value(false)
                .help("Break on validation errors"),
        )
        .arg(
            clap::Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Verbosity level"),
        )
        .arg(
            clap::Arg::with_name("primary")
                .short("p")
                .takes_value(false)
                .help("Defines if current instance is the primary one"),
        )
        .get_matches();

    let log_level: log::Level;
    match command_args.occurrences_of("v") {
        0 => log_level = log::Level::Debug,
        1 | _ => log_level = log::Level::Trace,
    };

    simple_logger::init_with_level(log_level).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Cannot create window");
    window.set_inner_size(winit::dpi::LogicalSize::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
    ));
    let mut sample = InterprocessCommunicationSample::new(window.hwnd());

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                sample.draw();
            }
            _ => (),
        }
    });
}
