#[macro_use]
extern crate rusty_d3d12;

use std::cmp::max;
use std::cmp::min;
use std::ffi::c_void;
use std::ffi::CStr;
use std::intrinsics::copy_nonoverlapping;
use std::mem::MaybeUninit;
use std::os::raw::c_char;
use std::rc::Rc;
use std::slice;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::AtomicU8;
use std::sync::atomic::Ordering;
use std::sync::mpsc;
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;

use cgmath::vec3;
use cgmath::vec4;
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

const FRAMES_IN_FLIGHT: usize = 2;
const THREAD_COUNT: u32 = 1;
const PARTICLE_COUNT: u32 = 10000;
const PARTICLE_SPREAD: f32 = 400.;

const USE_DEBUG: bool = true;
const USE_WARP_ADAPTER: bool = false;

const CLEAR_COLOR: [f32; 4] = [0.0, 0.2, 0.3, 1.0];

type Mat4 = Matrix4<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;
type Vec2 = Vector2<f32>;

#[repr(C)]
pub struct Vertex {
    color: Vec4,
}

impl Vertex {
    pub fn make_desc() -> Vec<InputElementDesc<'static>> {
        vec![InputElementDesc::default()
            .set_name("COLOR")
            .unwrap()
            .set_format(Format::R32G32B32A32_Float)
            .set_input_slot(0)
            .set_offset(Bytes::from(offset_of!(Self, color)))]
    }
}

#[repr(C)]
pub struct Particle {
    position: Vec4,
    velocity: Vec4,
}

#[repr(C)]
pub struct ConstantBufferCs {
    param_u32: Vector4<u32>,
    param_f32: Vec4,
}

#[repr(C)]
pub struct ConstantBufferGs {
    wvp: Mat4,
    inverse_view: Mat4,
    _padding: [f32; 32],
}

// Indices of the root signature parameters.
const GRAPHICS_ROOT_CBV: u32 = 0;
const GRAPHICS_ROOT_SRV_TABLE: u32 = GRAPHICS_ROOT_CBV + 1;
const GRAPHICS_ROOT_PARAMETERS_COUNT: u32 = GRAPHICS_ROOT_SRV_TABLE + 1;

const COMPUTE_ROOT_CBV: u32 = 0;
const COMPUTE_ROOT_SRV_TABLE: u32 = COMPUTE_ROOT_CBV + 1;
const COMPUTE_ROOT_UAV_TABLE: u32 = COMPUTE_ROOT_SRV_TABLE + 1;
const COMPUTE_ROOT_PARAMETERS_COUNT: u32 = COMPUTE_ROOT_UAV_TABLE + 1;

// Indices of shader resources in the descriptor heap
const UAV_PARTICLE_POS_VEL_0: u32 = 0;
const UAV_PARTICLE_POS_VEL_1: u32 = UAV_PARTICLE_POS_VEL_0 + 1;
const SRV_PARTICLE_POS_VEL_0: u32 = UAV_PARTICLE_POS_VEL_1 + 1;
const SRV_PARTICLE_POS_VEL_1: u32 = SRV_PARTICLE_POS_VEL_0 + 1;
const DESCRIPTOR_COUNT: u32 = SRV_PARTICLE_POS_VEL_1 + 1;

struct NBodyGravitySample {
    pipeline: Pipeline,
}

impl NBodyGravitySample {
    fn new(hwnd: *mut std::ffi::c_void) -> Self {
        let mut pipeline = Pipeline::new(hwnd);
        // pipeline.render();

        NBodyGravitySample { pipeline }
        // InterprocessCommunicationSample { pipeline }
    }

    fn draw(&mut self) {
        self.pipeline.update();
        // self.pipeline.render();
    }
}

impl Drop for NBodyGravitySample {
    fn drop(&mut self) {
        if USE_DEBUG {
            self.pipeline
                .debug_device
                .as_ref()
                .expect("No debug devices created")
                .report_live_device_objects()
                .expect("Device cannot report live objects");
        }
    }
}

struct Pipeline {
    device: Device,
    debug_device: Option<DebugDevice>,
    info_queue: Option<Rc<InfoQueue>>,
    direct_command_queue: CommandQueue,
    swapchain: Swapchain,
    swapchain_event: Win32Event,
    frame_index: usize,
    viewport: Viewport,
    scissor_rect: Rect,
    rtv_heap: DescriptorHeap,
    srv_uav_heap: DescriptorHeap,
    render_targets: Vec<Resource>,
    direct_command_allocators: Vec<CommandAllocator>,
    graphics_root_signature: RootSignature,
    compute_root_signature: RootSignature,
    graphics_pso: PipelineState,
    compute_pso: PipelineState,

    direct_command_list: CommandList,

    vertex_buffer: Resource,
    vertex_buffer_upload: Resource,
    vertex_buffer_view: VertexBufferView,

    particle_buffer_0: Resource,
    particle_buffer_0_upload: Resource,
    particle_buffer_1: Resource,
    particle_buffer_1_upload: Resource,

    constant_buffer_cs: Resource,
    constant_buffer_cs_upload: Resource,

    constant_buffer_gs: Resource,

    render_context_fence: Fence,
    render_context_fence_event: Win32Event,
    // frame_fence_values: [u64; 2],
    // thread_fence: Fence,
    // thread_fence_event: Win32Event,
    render_context_fence_value: Arc<AtomicU64>,
    thread_fence_value: Arc<AtomicU64>,
    resource_selector: Arc<AtomicU8>, // aka m_srvIndex in the original sample
}

impl Pipeline {
    // aka LoadPipeline() in the original sample
    fn new(hwnd: *mut c_void) -> Self {
        let mut factory_flags = CreateFactoryFlags::None;
        if USE_DEBUG {
            let debug_controller =
                Debug::new().expect("Cannot create debug controller");
            debug_controller.enable_debug_layer();
            debug_controller.enable_gpu_based_validation();
            debug_controller.enable_object_auto_name();
            factory_flags = CreateFactoryFlags::Debug;
        }

        let factory =
            Factory::new(factory_flags).expect("Cannot create factory");
        let (device, is_software_adapter) =
            create_device(&factory, USE_WARP_ADAPTER);

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
                &CommandQueueDesc::default()
                    .set_queue_type(CommandListType::Direct),
            )
            .expect("Cannot create direct command queue");

        let swapchain = create_swapchain(factory, &direct_command_queue, hwnd);
        let swapchain_event = swapchain.get_frame_latency_waitable_object();

        let frame_index = swapchain.get_current_back_buffer_index() as usize;
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

        let (rtv_heap, srv_uav_heap) =
            create_descriptor_heaps(&device, &swapchain);

        let (render_targets, direct_command_allocators) =
            create_frame_resources(&device, &rtv_heap, &swapchain);

        let (graphics_root_signature, compute_root_signature) =
            create_root_signatures(&device);

        trace!("Created root signatures");

        let (graphics_pso, compute_pso) = create_psos(
            &device,
            &graphics_root_signature,
            &compute_root_signature,
        );

        let direct_command_list = create_command_list(
            &device,
            frame_index,
            &direct_command_allocators,
            &graphics_pso,
        );

        trace!("Created command list");

        let (vertex_buffer, vertex_buffer_upload, vertex_buffer_view) =
            create_vertex_buffer(&device, &direct_command_list);

        let (
            particle_buffer_0,
            particle_buffer_0_upload,
            particle_buffer_1,
            particle_buffer_1_upload,
        ) = create_particle_buffers(
            &device,
            &direct_command_list,
            &srv_uav_heap,
        );

        trace!("Created partice buffers");

        let (constant_buffer_cs, constant_buffer_cs_upload) =
            create_cs_constant_buffer(&device, &direct_command_list);
        let constant_buffer_gs =
            create_gs_constant_buffer(&device, &direct_command_list);

        let (render_context_fence, render_context_fence_event) =
            create_fences(&device);

        let mut render_context_fence_value = 1;
        {
            direct_command_list
                .close()
                .expect("Cannot close command list");
            direct_command_queue
                .execute_command_lists(slice::from_ref(&direct_command_list));

            direct_command_queue
                .signal(&render_context_fence, render_context_fence_value)
                .expect("Cannot signal fence");

            render_context_fence
                .set_event_on_completion(
                    render_context_fence_value,
                    &render_context_fence_event,
                )
                .expect("Cannot set fence event");

            render_context_fence_value += 1;

            render_context_fence_event.wait(None);
        }

        trace!("Executed command lists");

        Self {
            device,
            debug_device,
            info_queue,
            direct_command_queue,
            swapchain,
            swapchain_event,
            frame_index,
            viewport,
            scissor_rect,
            rtv_heap,
            srv_uav_heap,
            render_targets,
            direct_command_allocators,
            graphics_root_signature,
            compute_root_signature,
            graphics_pso,
            compute_pso,
            direct_command_list,
            vertex_buffer,
            vertex_buffer_upload,
            vertex_buffer_view,

            particle_buffer_0,
            particle_buffer_0_upload,
            particle_buffer_1,
            particle_buffer_1_upload,

            constant_buffer_cs,
            constant_buffer_cs_upload,
            constant_buffer_gs,

            render_context_fence,
            render_context_fence_event,
            render_context_fence_value: Arc::new(AtomicU64::new(0)),
            // thread_fence,
            // thread_fence_event, // shared_fence_values: vec![0, 0, 0],
            thread_fence_value: Arc::new(AtomicU64::new(0)),
            resource_selector: Arc::new(AtomicU8::new(0)),
        }
    }

    // fn populate_producer_command_list(&mut self) {
    //     self.direct_command_allocators[self.frame_index]
    //         .reset()
    //         .expect("Cannot reset direct command allocator");

    //     self.direct_command_list
    //         .reset(
    //             &self.direct_command_allocators[self.frame_index],
    //             Some(&self.pipeline_state),
    //         )
    //         .expect("Cannot reset direct command list");

    //     self.direct_command_list
    //         .set_graphics_root_signature(&self.root_signature);

    //     self.direct_command_list
    //         .set_viewports(slice::from_ref(&self.viewport));

    //     self.direct_command_list
    //         .set_scissor_rects(slice::from_ref(&self.scissor_rect));

    //     self.direct_command_list.resource_barrier(slice::from_ref(
    //         &ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.render_targets[self.frame_index])
    //                 .set_state_before(ResourceStates::CommonOrPresent)
    //                 .set_state_after(ResourceStates::RenderTarget),
    //         ),
    //     ));

    //     let rtv_handle = self
    //         .rtv_heap
    //         .get_cpu_descriptor_handle_for_heap_start()
    //         .advance(self.frame_index as u32);

    //     self.direct_command_list.set_render_targets(
    //         slice::from_ref(&rtv_handle),
    //         false,
    //         None,
    //     );

    //     self.direct_command_list.clear_render_target_view(
    //         rtv_handle,
    //         CLEAR_COLOR,
    //         &[],
    //     );

    //     self.direct_command_list
    //         .set_primitive_topology(PrimitiveTopology::TriangleList);
    //     self.direct_command_list
    //         .set_vertex_buffers(0, slice::from_ref(&self.vertex_buffer_view));

    //     self.direct_command_list
    //         .set_graphics_root_constant_buffer_view(
    //             0,
    //             GpuVirtualAddress(
    //                 self.triangle_constant_buffer.get_gpu_virtual_address().0,
    //             ),
    //         );

    //     self.direct_command_list.draw_instanced(3, 1, 0, 0);

    //     self.direct_command_list.resource_barrier(&[
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.render_targets[self.frame_index])
    //                 .set_state_before(ResourceStates::RenderTarget)
    //                 .set_state_after(ResourceStates::CopySource),
    //         ),
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.cross_process_resource)
    //                 .set_state_before(ResourceStates::CommonOrPresent)
    //                 .set_state_after(ResourceStates::CopyDest),
    //         ),
    //     ]);

    //     self.direct_command_list.copy_resource(
    //         &self.render_targets[self.frame_index],
    //         &self.cross_process_resource,
    //     );

    //     self.direct_command_list.resource_barrier(&[
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.render_targets[self.frame_index])
    //                 .set_state_before(ResourceStates::CopySource)
    //                 .set_state_after(ResourceStates::CommonOrPresent),
    //         ),
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.cross_process_resource)
    //                 .set_state_before(ResourceStates::CopyDest)
    //                 .set_state_after(ResourceStates::CommonOrPresent),
    //         ),
    //     ]);

    //     self.direct_command_list
    //         .close()
    //         .expect("Cannot close command list");
    // }

    // fn populate_consumer_command_list(&mut self) {
    //     self.direct_command_allocators[self.frame_index]
    //         .reset()
    //         .expect("Cannot reset direct command allocator");

    //     self.direct_command_list
    //         .reset(
    //             &self.direct_command_allocators[self.frame_index],
    //             Some(&self.pipeline_state),
    //         )
    //         .expect("Cannot reset direct command list");

    //     self.direct_command_list
    //         .set_graphics_root_signature(&self.root_signature);

    //     self.direct_command_list
    //         .set_viewports(slice::from_ref(&self.viewport));

    //     self.direct_command_list
    //         .set_scissor_rects(slice::from_ref(&self.scissor_rect));

    //     self.direct_command_list.resource_barrier(&[
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.render_targets[self.frame_index])
    //                 .set_state_before(ResourceStates::CommonOrPresent)
    //                 .set_state_after(ResourceStates::CopyDest),
    //         ),
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.cross_process_resource)
    //                 .set_state_before(ResourceStates::CommonOrPresent)
    //                 .set_state_after(ResourceStates::CopySource),
    //         ),
    //     ]);

    //     self.direct_command_list.copy_resource(
    //         &self.cross_process_resource,
    //         &self.render_targets[self.frame_index],
    //     );

    //     self.direct_command_list.resource_barrier(&[
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.render_targets[self.frame_index])
    //                 .set_state_before(ResourceStates::CopyDest)
    //                 .set_state_after(ResourceStates::RenderTarget),
    //         ),
    //         ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.cross_process_resource)
    //                 .set_state_before(ResourceStates::CopySource)
    //                 .set_state_after(ResourceStates::CommonOrPresent),
    //         ),
    //     ]);

    //     let rtv_handle = self
    //         .rtv_heap
    //         .get_cpu_descriptor_handle_for_heap_start()
    //         .advance(self.frame_index as u32);

    //     self.direct_command_list.set_render_targets(
    //         slice::from_ref(&rtv_handle),
    //         false,
    //         None,
    //     );

    //     self.direct_command_list
    //         .set_primitive_topology(PrimitiveTopology::TriangleList);
    //     self.direct_command_list
    //         .set_vertex_buffers(0, slice::from_ref(&self.vertex_buffer_view));

    //     self.direct_command_list
    //         .set_graphics_root_constant_buffer_view(
    //             0,
    //             GpuVirtualAddress(
    //                 self.triangle_constant_buffer.get_gpu_virtual_address().0,
    //             ),
    //         );

    //     self.direct_command_list.draw_instanced(3, 1, 0, 0);

    //     self.direct_command_list.resource_barrier(slice::from_ref(
    //         &ResourceBarrier::new_transition(
    //             &ResourceTransitionBarrier::default()
    //                 .set_resource(&self.render_targets[self.frame_index])
    //                 .set_state_before(ResourceStates::RenderTarget)
    //                 .set_state_after(ResourceStates::CommonOrPresent),
    //         ),
    //     ));

    //     self.direct_command_list
    //         .close()
    //         .expect("Cannot close command list");
    // }

    // fn render(&mut self) {
    //     trace!("Rendering frame, idx {}", self.frame_index);
    //     let completed_shared_resource_fence_value =
    //         self.shared_resource_fence.get_completed_value();
    //     if completed_shared_resource_fence_value
    //         < self.current_shared_fence_value
    //     {
    //         self.shared_resource_fence
    //             .set_event_on_completion(
    //                 self.current_shared_fence_value,
    //                 &self.shared_resource_fence_event,
    //             )
    //             .expect("Cannot set fence event");

    //         self.shared_resource_fence_event.wait(None);
    //     }

    //     if self.is_producer_process {
    //         self.populate_producer_command_list();
    //     } else {
    //         self.populate_consumer_command_list();
    //     }

    //     self.direct_command_queue
    //         .execute_command_lists(slice::from_ref(&self.direct_command_list));

    //     self.current_frame_resource_fence_value += 1;
    //     self.direct_command_queue
    //         .signal(
    //             &self.frame_resource_fence,
    //             self.current_frame_resource_fence_value,
    //         )
    //         .expect("Cannot signal direct command queue");

    //     self.frame_resources_fence_values[self.frame_index] =
    //         self.current_frame_resource_fence_value;

    //     self.current_shared_fence_value += 1;
    //     self.direct_command_queue
    //         .signal(
    //             &self.shared_resource_fence,
    //             self.current_shared_fence_value,
    //         )
    //         .expect("Cannot signal direct command queue");
    //     // The other process will increase fence value, so we'll wait on it
    //     self.current_shared_fence_value += 1;

    //     self.swapchain.present(1, 0).expect("Cannot present");

    //     self.move_to_next_frame();
    // }

    // fn move_to_next_frame(&mut self) {
    //     self.frame_index =
    //         self.swapchain.get_current_back_buffer_index() as usize;

    //     let completed_frame_fence_value =
    //         self.frame_resource_fence.get_completed_value();
    //     if completed_frame_fence_value
    //         < self.frame_resources_fence_values[self.frame_index]
    //     {
    //         self.frame_resource_fence
    //             .set_event_on_completion(
    //                 self.frame_resources_fence_values[self.frame_index],
    //                 &self.frame_resource_fence_event,
    //             )
    //             .expect("Cannot set fence event");

    //         self.frame_resource_fence_event.wait(None);
    //     }
    // }

    fn update(&mut self) {
        self.swapchain_event.wait(Some(100));

        let cb_data: ConstantBufferGs = unsafe { std::mem::zeroed() };
        
    }
}

fn simulate(
    resource_selector: Arc<AtomicU8>,
    uavs: &[Resource; 2],
    compute_command_list: &CommandList,
    pso: &PipelineState,
    root_sig: &RootSignature,
    srv_uav_heap: &DescriptorHeap,
    constant_buffer: &Resource,
) {
    let curr_srv_index;
    let curr_uav_index;
    let curr_uav;
    if resource_selector.load(Ordering::SeqCst) == 0 {
        curr_srv_index = SRV_PARTICLE_POS_VEL_0;
        curr_uav_index = UAV_PARTICLE_POS_VEL_1;
        curr_uav = &uavs[0];
    } else {
        curr_srv_index = SRV_PARTICLE_POS_VEL_1;
        curr_uav_index = UAV_PARTICLE_POS_VEL_0;
        curr_uav = &uavs[1];
    }
    compute_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .set_resource(curr_uav)
                .set_state_before(ResourceStates::NonPixelShaderResource)
                .set_state_after(ResourceStates::UnorderedAccess),
        ),
    ));
    compute_command_list.set_pipeline_state(pso);
    compute_command_list.set_compute_root_signature(root_sig);
    compute_command_list.set_descriptor_heaps(slice::from_ref(srv_uav_heap));
    let srv_handle = srv_uav_heap
        .get_gpu_descriptor_handle_for_heap_start()
        .advance(curr_srv_index);
    let uav_handle = srv_uav_heap
        .get_gpu_descriptor_handle_for_heap_start()
        .advance(curr_uav_index);
    compute_command_list.set_compute_root_constant_buffer_view(
        COMPUTE_ROOT_CBV,
        constant_buffer.get_gpu_virtual_address(),
    );
    compute_command_list
        .set_compute_root_descriptor_table(COMPUTE_ROOT_SRV_TABLE, srv_handle);
    compute_command_list
        .set_compute_root_descriptor_table(COMPUTE_ROOT_UAV_TABLE, uav_handle);
    compute_command_list.dispatch(
        (PARTICLE_COUNT as f32 / 128.).ceil() as u32,
        1,
        1,
    );
    compute_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .set_resource(curr_uav)
                .set_state_before(ResourceStates::UnorderedAccess)
                .set_state_after(ResourceStates::NonPixelShaderResource),
        ),
    ));
}

fn create_async_contexts(
    device: Device,
    srv_uav_heap: DescriptorHeap,
    uavs: [Resource; 2],
    constant_buffer: Resource,
    pso: PipelineState,
    root_sig: RootSignature,
    render_context_fence: Fence,
    render_context_fence_value: Arc<AtomicU64>,
    thread_fence_value: Arc<AtomicU64>,
    resource_selector: Arc<AtomicU8>,
) {
    let (tx, rx) = mpsc::channel();

    let async_thread_handle = std::thread::spawn(move || {
        let compute_command_queue = device
            .create_command_queue(
                &CommandQueueDesc::default()
                    .set_queue_type(CommandListType::Compute),
            )
            .expect("Cannot create compute command queue");

        let compute_command_allocator = device
            .create_command_allocator(CommandListType::Compute)
            .expect("Cannot create compute command allocator");

        let compute_command_list = device
            .create_command_list(
                CommandListType::Compute,
                &compute_command_allocator,
                None,
            )
            .expect("Cannot create compute command list");

        let thread_fence = device
            .create_fence(0, FenceFlags::None)
            .expect("Cannot create thread_fence");
        let thread_fence_event = Win32Event::default();

        loop {
            match rx.try_recv() {
                Ok(()) => break,
                Err(err) => match err {
                    TryRecvError::Empty => {}
                    TryRecvError::Disconnected => {
                        panic!("main thread destroyed its channel why async thread was alive")
                    }
                },
            }

            simulate(
                resource_selector.clone(),
                &uavs,
                &compute_command_list,
                &pso,
                &root_sig,
                &srv_uav_heap,
                &constant_buffer,
            );

            compute_command_list
                .close()
                .expect("Cannot close compute command list");

            // ToDo: pix marker

            compute_command_queue
                .execute_command_lists(slice::from_ref(&compute_command_list));

            let fence_value =
                thread_fence_value.fetch_add(1, Ordering::SeqCst) + 1;

            compute_command_queue
                .signal(&thread_fence, fence_value)
                .expect("Cannot signal on compute queue");

            thread_fence
                .set_event_on_completion(fence_value, &thread_fence_event)
                .expect("Cannot set event on thread fence");

            thread_fence_event.wait(None);

            let current_render_context_fence_value =
                render_context_fence_value.load(Ordering::SeqCst);

            if render_context_fence.get_completed_value()
                < current_render_context_fence_value
            {
                compute_command_queue
                    .wait(
                        &render_context_fence,
                        current_render_context_fence_value,
                    )
                    .expect("Cannot call wait on queue");

                render_context_fence_value.swap(0, Ordering::SeqCst);
            }

            let prev_idx = resource_selector.load(Ordering::SeqCst);
            resource_selector.store((prev_idx + 1) % 2, Ordering::SeqCst);

            compute_command_allocator
                .reset()
                .expect("Cannot reset command allocator");

            compute_command_list
                .reset(&compute_command_allocator, Some(&pso))
                .expect("Cannot reset compute command list");
        }
    });
}

fn create_fences(device: &Device) -> (Fence, Win32Event) {
    let render_context_fence = device
        .create_fence(0, FenceFlags::None)
        .expect("Cannot create render_context_fence");
    let render_context_fence_event = Win32Event::default();

    (render_context_fence, render_context_fence_event)
}

fn create_gs_constant_buffer(
    device: &Device,
    direct_command_list: &CommandList,
) -> Resource {
    let buffer_size = size_of!(ConstantBufferGs);

    let constant_buffer_gs = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(buffer_size.0.into()),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create constant_buffer_gs");
    constant_buffer_gs
        .set_name("constant_buffer_gs")
        .expect("Cannot set name on resource");

    let cb_data: ConstantBufferGs = unsafe { std::mem::zeroed() };

    let mapped_data = constant_buffer_gs
        .map(0, None)
        .expect("Cannot map constant_buffer_gs");
    unsafe {
        copy_nonoverlapping(&cb_data, mapped_data as *mut ConstantBufferGs, 1);
    }

    trace!("Created GS constant buffer");

    constant_buffer_gs
}

fn create_cs_constant_buffer(
    device: &Device,
    direct_command_list: &CommandList,
) -> (Resource, Resource) {
    let buffer_size = size_of!(ConstantBufferCs);

    let constant_buffer_cs = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(buffer_size.0.into()),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create constant_buffer_cs");
    constant_buffer_cs
        .set_name("constant_buffer_cs")
        .expect("Cannot set name on resource");

    let constant_buffer_cs_upload = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(buffer_size.0.into()),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create constant_buffer_cs_upload");
    constant_buffer_cs_upload
        .set_name("constant_buffer_cs_upload")
        .expect("Cannot set name on resource");

    let cb_data = ConstantBufferCs {
        param_u32: Vector4::new(
            PARTICLE_COUNT,
            (PARTICLE_COUNT as f32 / 128.).ceil() as u32,
            0,
            0,
        ),
        param_f32: vec4(0.1, 1., 0., 0.),
    };

    let subresource_data = SubresourceData::default()
        .set_data(slice::from_ref(&cb_data))
        .set_row_pitch(size_of!(ConstantBufferCs))
        .set_slice_pitch(size_of!(ConstantBufferCs));

    direct_command_list
        .update_subresources_heap_alloc(
            &constant_buffer_cs,
            &constant_buffer_cs_upload,
            Bytes(0),
            0,
            1,
            slice::from_ref(&subresource_data),
        )
        .expect("Cannot upload vertex buffer");
    trace!("Uploaded vertex buffer");

    direct_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .set_resource(&constant_buffer_cs)
                .set_state_before(ResourceStates::CopyDest)
                .set_state_after(ResourceStates::VertexAndConstantBuffer),
        ),
    ));

    trace!("Created CS constant buffer");

    (constant_buffer_cs, constant_buffer_cs_upload)
}

fn create_particle_buffers(
    device: &Device,
    direct_command_list: &CommandList,
    srv_uav_heap: &DescriptorHeap,
) -> (Resource, Resource, Resource, Resource) {
    let center_spread = PARTICLE_SPREAD / 2.;

    let mut data = vec![];

    let center = vec3(center_spread, 0., 0.);
    let velocity = vec4(0., 0., -20., 1e-8);
    for idx in 0..PARTICLE_COUNT / 2 {
        let mut delta = vec3(PARTICLE_SPREAD, PARTICLE_SPREAD, PARTICLE_SPREAD);

        while delta.dot(delta) > PARTICLE_SPREAD * PARTICLE_SPREAD {
            delta.x = rand::thread_rng().gen_range(-1. ..1.) * PARTICLE_SPREAD;
            delta.y = rand::thread_rng().gen_range(-1. ..1.) * PARTICLE_SPREAD;
            delta.z = rand::thread_rng().gen_range(-1. ..1.) * PARTICLE_SPREAD;
        }

        data.push(Particle {
            position: center.extend(1e8 / 2.) + delta.extend(1e8 / 2.),
            velocity,
        });
    }

    let center = vec3(-center_spread, 0., 0.);
    let velocity = vec4(0., 0., 20., 1e-8);
    for idx in 0..PARTICLE_COUNT / 2 {
        let mut delta = vec3(PARTICLE_SPREAD, PARTICLE_SPREAD, PARTICLE_SPREAD);

        while delta.dot(delta) > PARTICLE_SPREAD * PARTICLE_SPREAD {
            delta.x = rand::thread_rng().gen_range(-1. ..1.) * PARTICLE_SPREAD;
            delta.y = rand::thread_rng().gen_range(-1. ..1.) * PARTICLE_SPREAD;
            delta.z = rand::thread_rng().gen_range(-1. ..1.) * PARTICLE_SPREAD;
        }

        data.push(Particle {
            position: center.extend(1e8 / 2.) + delta.extend(1e8 / 2.),
            velocity,
        });
    }

    trace!("Inserted {} particles", data.len());
    let data_size = data.len() * size_of!(Particle);

    let particle_buffer_0 = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(data_size.0)
                .set_flags(ResourceFlags::AllowUnorderedAccess),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create particle_buffer_0");
    particle_buffer_0
        .set_name("particle_buffer_0")
        .expect("Cannot set name on resource");

    let particle_buffer_1 = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(data_size.0)
                .set_flags(ResourceFlags::AllowUnorderedAccess),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create vertex_buffer");
    particle_buffer_1
        .set_name("particle_buffer_1")
        .expect("Cannot set name on resource");

    let particle_buffer_0_upload = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(data_size.0),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create particle_buffer_0_upload");
    particle_buffer_0_upload
        .set_name("particle_buffer_0_upload")
        .expect("Cannot set name on resource");

    let particle_buffer_1_upload = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(data_size.0),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create particle_buffer_1_upload");
    particle_buffer_1_upload
        .set_name("particle_buffer_1_upload")
        .expect("Cannot set name on resource");

    let particle_data = SubresourceData::default()
        .set_data(&data)
        .set_row_pitch(data_size)
        .set_slice_pitch(data_size);

    direct_command_list
        .update_subresources_heap_alloc(
            &particle_buffer_0,
            &particle_buffer_0_upload,
            Bytes(0),
            0,
            1,
            slice::from_ref(&particle_data),
        )
        .expect("Cannot upload particle buffer");

    direct_command_list
        .update_subresources_heap_alloc(
            &particle_buffer_1,
            &particle_buffer_1_upload,
            Bytes(0),
            0,
            1,
            slice::from_ref(&particle_data),
        )
        .expect("Cannot upload particle buffer");
    trace!("Uploaded particle buffers");

    let srv_desc = ShaderResourceViewDesc::default()
        .new_buffer(
            &BufferSrv::default()
                .set_first_element(0)
                .set_num_elements(PARTICLE_COUNT)
                .set_structure_byte_stride(size_of!(Particle)),
        )
        .set_shader4_component_mapping(ShaderComponentMapping::default());

    device.create_shader_resource_view(
        &particle_buffer_0,
        Some(&srv_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(SRV_PARTICLE_POS_VEL_0),
    );
    device.create_shader_resource_view(
        &particle_buffer_1,
        Some(&srv_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(SRV_PARTICLE_POS_VEL_1),
    );

    let uav_desc = UnorderedAccessViewDesc::default().new_buffer(
        &BufferUav::default()
            .set_first_element(0)
            .set_num_elements(PARTICLE_COUNT)
            .set_structure_byte_stride(size_of!(Particle))
            .set_counter_offset_in_bytes(Bytes(0)),
    );

    device.create_unordered_access_view(
        &particle_buffer_0,
        None,
        Some(&uav_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(UAV_PARTICLE_POS_VEL_0),
    );

    device.create_unordered_access_view(
        &particle_buffer_1,
        None,
        Some(&uav_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(UAV_PARTICLE_POS_VEL_1),
    );

    (
        particle_buffer_0,
        particle_buffer_0_upload,
        particle_buffer_1,
        particle_buffer_1_upload,
    )
}

fn create_vertex_buffer(
    device: &Device,
    direct_command_list: &CommandList,
) -> (Resource, Resource, VertexBufferView) {
    let mut particle_vertices = vec![];
    for idx in 0..PARTICLE_COUNT {
        particle_vertices.push(Vertex {
            color: vec4(1., 1., 0.2, 1.),
        });
    }

    let vertex_buffer_size = size_of!(Vertex);

    let vertex_buffer = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Default),
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

    let vertex_buffer_upload = device
        .create_committed_resource(
            &HeapProperties::default().set_heap_type(HeapType::Upload),
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
        .set_data(&particle_vertices)
        .set_row_pitch(vertex_buffer_size)
        .set_slice_pitch(vertex_buffer_size);

    direct_command_list
        .update_subresources_heap_alloc(
            &vertex_buffer,
            &vertex_buffer_upload,
            Bytes(0),
            0,
            1,
            slice::from_ref(&vertex_data),
        )
        .expect("Cannot upload vertex buffer");
    trace!("Uploaded vertex buffer");

    direct_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .set_resource(&vertex_buffer)
                .set_state_before(ResourceStates::CopyDest)
                .set_state_after(ResourceStates::VertexAndConstantBuffer),
        ),
    ));

    let vertex_buffer_view = VertexBufferView::default()
        .set_buffer_location(vertex_buffer.get_gpu_virtual_address())
        .set_stride_in_bytes(size_of!(Vertex))
        .set_size_in_bytes(vertex_buffer_size);
    trace!("Created primary adapter vertex buffer");

    (vertex_buffer, vertex_buffer_upload, vertex_buffer_view)
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
            &direct_command_allocators[frame_index],
            Some(pso),
            // None,
        )
        .expect("Cannot create direct command list")
}

fn create_psos(
    device: &Device,
    graphics_root_signature: &RootSignature,
    compute_root_signature: &RootSignature,
) -> (PipelineState, PipelineState) {
    let vertex_shader = compile_shader(
        "VertexShader",
        &std::fs::read_to_string("assets/nbg_gs.hlsl")
            .expect("Cannot open vertex shader file"),
        "VSParticleDraw",
        "vs_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile vertex shader");
    let geometry_shader = compile_shader(
        "GeometryShader",
        &std::fs::read_to_string("assets/nbg_gs.hlsl")
            .expect("Cannot open geometry shader file"),
        "GSParticleDraw",
        "gs_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile geometry shader");
    let pixel_shader = compile_shader(
        "PixelShader",
        &std::fs::read_to_string("assets/nbg_gs.hlsl")
            .expect("Cannot open pixel shader file"),
        "PSParticleDraw",
        "ps_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile pixel shader");

    let input_layout = Vertex::make_desc();

    let vs_bytecode = ShaderBytecode::from_bytes(&vertex_shader);
    let gs_bytecode = ShaderBytecode::from_bytes(&geometry_shader);
    let ps_bytecode = ShaderBytecode::from_bytes(&pixel_shader);

    let input_layout =
        InputLayoutDesc::default().from_input_elements(&input_layout);
    let graphics_pso_desc = GraphicsPipelineStateDesc::default()
        .set_input_layout(&input_layout)
        .set_root_signature(graphics_root_signature)
        .set_vs_bytecode(&vs_bytecode)
        .set_gs_bytecode(&gs_bytecode)
        .set_ps_bytecode(&ps_bytecode)
        .set_rasterizer_state(&RasterizerDesc::default())
        .set_blend_state(
            &BlendDesc::default().set_render_targets(slice::from_ref(
                &RenderTargetBlendDesc::default()
                    .set_blend_enable(true)
                    .set_src_blend(Blend::SrcAlpha)
                    .set_dest_blend(Blend::One)
                    .set_src_blend_alpha(Blend::Zero)
                    .set_dest_blend_alpha(Blend::Zero),
            )),
        )
        .set_depth_stencil_state(
            &DepthStencilDesc::default()
                .set_depth_write_mask(DepthWriteMask::Zero),
        )
        .set_primitive_topology_type(PrimitiveTopologyType::Point)
        .set_rtv_formats(&[Format::R8G8B8A8_UNorm])
        .set_dsv_format(Format::D32_Float);

    let graphics_pso = device
        .create_graphics_pipeline_state(&graphics_pso_desc)
        .expect("Cannot create PSO");
    graphics_pso
        .set_name("graphics_pso")
        .expect("Cannot set name on pso");
    trace!("Created graphics_pso");

    let compute_shader = compile_shader(
        "ComputeShader",
        &std::fs::read_to_string("assets/nbg_cs.hlsl")
            .expect("Cannot open pixel shader file"),
        "CSMain",
        "cs_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile compute shader");
    let cs_bytecode = ShaderBytecode::from_bytes(&compute_shader);

    let compute_pso_desc = ComputePipelineStateDesc::default()
        .set_root_signature(compute_root_signature)
        .set_cs_bytecode(&cs_bytecode);

    let compute_pso = device
        .create_compute_pipeline_state(&compute_pso_desc)
        .expect("Cannot create compute PSO");

    trace!("Created compute_pso");

    (graphics_pso, compute_pso)
}

fn create_root_signatures(device: &Device) -> (RootSignature, RootSignature) {
    let graphics_root_signature = {
        let root_parameters = [
            RootParameter::default()
                .new_descriptor(
                    &RootDescriptor::default()
                        .set_shader_register(0)
                        .set_flags(RootDescriptorFlags::DataStatic),
                    RootParameterType::Cbv,
                )
                .set_shader_visibility(ShaderVisibility::All),
            RootParameter::default().new_descriptor_table(
                &RootDescriptorTable::default().set_descriptor_ranges(
                    slice::from_ref(
                        &DescriptorRange::default()
                            .set_range_type(DescriptorRangeType::Srv)
                            .set_num_descriptors(1)
                            .set_base_shader_register(0)
                            .set_flags(DescriptorRangeFlags::DataStatic),
                    ),
                ),
            ),
        ];

        let root_signature_desc = VersionedRootSignatureDesc::default()
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

    let compute_root_signature = {
        let root_parameters = [
            RootParameter::default()
                .new_descriptor(
                    &RootDescriptor::default()
                        .set_shader_register(0)
                        .set_flags(RootDescriptorFlags::DataStatic),
                    RootParameterType::Cbv,
                )
                .set_shader_visibility(ShaderVisibility::All),
            RootParameter::default().new_descriptor_table(
                &RootDescriptorTable::default().set_descriptor_ranges(
                    slice::from_ref(
                        &DescriptorRange::default()
                            .set_range_type(DescriptorRangeType::Srv)
                            .set_num_descriptors(1)
                            .set_base_shader_register(0)
                            .set_flags(
                                DescriptorRangeFlags::DescriptorsVolatile,
                            ),
                    ),
                ),
            ),
            RootParameter::default().new_descriptor_table(
                &RootDescriptorTable::default().set_descriptor_ranges(
                    slice::from_ref(
                        &DescriptorRange::default()
                            .set_range_type(DescriptorRangeType::Uav)
                            .set_num_descriptors(1)
                            .set_base_shader_register(0)
                            .set_flags(DescriptorRangeFlags::DataVolatile),
                    ),
                ),
            ),
        ];

        let root_signature_desc = VersionedRootSignatureDesc::default()
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
    (graphics_root_signature, compute_root_signature)
}

fn create_frame_resources(
    device: &Device,
    rtv_heap: &DescriptorHeap,
    swapchain: &Swapchain,
) -> (Vec<Resource>, Vec<CommandAllocator>) {
    let clear_value = ClearValue::default()
        .set_format(Format::R8G8B8A8_UNorm)
        .set_color(CLEAR_COLOR);

    let render_target_desc = ResourceDesc::default()
        .set_dimension(ResourceDimension::Texture2D)
        .set_format(Format::R8G8B8A8_UNorm)
        .set_width(WINDOW_WIDTH.into())
        .set_height(WINDOW_HEIGHT)
        .set_flags(ResourceFlags::AllowRenderTarget);

    let mut render_targets = vec![];

    for frame_idx in 0..FRAMES_IN_FLIGHT {
        render_targets.push(
            swapchain
                .get_buffer(frame_idx as u32)
                .expect("Cannot get buffer from swapchain"),
        );
    }
    let mut direct_command_allocators = vec![];

    let mut rtv_handle = rtv_heap.get_cpu_descriptor_handle_for_heap_start();
    for frame_idx in 0..FRAMES_IN_FLIGHT {
        device
            .create_render_target_view(&render_targets[frame_idx], rtv_handle);

        rtv_handle = rtv_handle.advance(1);

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
    swapchain: &Swapchain,
) -> (DescriptorHeap, DescriptorHeap) {
    let rtv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_heap_type(DescriptorHeapType::RTV)
                .set_num_descriptors(FRAMES_IN_FLIGHT as u32),
        )
        .expect("Cannot create RTV heap");
    rtv_heap
        .set_name("RTV heap")
        .expect("Cannot set RTV heap name");

    let srv_uav_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_heap_type(DescriptorHeapType::CBV_SRV_UAV)
                .set_num_descriptors(DESCRIPTOR_COUNT)
                .set_flags(DescriptorHeapFlags::ShaderVisible),
        )
        .expect("Cannot create srv_uav_heap");
    srv_uav_heap
        .set_name("CBV_SRV heap")
        .expect("Cannot set srv_uav_heap name");

    (rtv_heap, srv_uav_heap)
}

fn create_swapchain(
    factory: Factory,
    command_queue: &CommandQueue,
    hwnd: *mut std::ffi::c_void,
) -> Swapchain {
    let swapchain_desc = SwapchainDesc::default()
        .set_width(WINDOW_WIDTH)
        .set_height(WINDOW_HEIGHT)
        .set_buffer_count(FRAMES_IN_FLIGHT as u32);
    let swapchain = factory
        .create_swapchain(&command_queue, hwnd as *mut HWND__, &swapchain_desc)
        .expect("Cannot create swapchain");
    factory
        .make_window_association(hwnd, MakeWindowAssociationFlags::NoAltEnter)
        .expect("Cannot make window association");
    swapchain
}

fn get_hardware_adapter(factory: &Factory) -> Adapter {
    let mut adapters = factory
        .enum_adapters_by_gpu_preference(GpuPreference::HighPerformance)
        .expect("Cannot enumerate adapters");

    for adapter in &adapters {
        let desc = adapter.get_desc().expect("Cannot get adapter desc");
        info!("found adapter: {}", desc);
    }
    adapters.remove(0)
}

fn create_device(factory: &Factory, use_warp: bool) -> (Device, bool) {
    let adapter;
    if use_warp {
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
    let mut sample = NBodyGravitySample::new(window.hwnd());

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
