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

const FRAMES_IN_FLIGHT: usize = 3;

const USE_DEBUG: bool = true;
const USE_WARP_ADAPTER: bool = false;

const CLEAR_COLOR: [f32; 4] = [0.0, 0.2, 0.3, 1.0];

type Mat4 = Matrix4<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;
type Vec2 = Vector2<f32>;

#[repr(C)]
pub struct Vertex {
    position: Vec4,
    color: Vec4,
}

impl Vertex {
    pub fn make_desc() -> InputLayout {
        vec![
            InputElementDesc::default()
                .set_name(CString::new("POSITION").unwrap())
                .set_format(DxgiFormat::R32G32B32A32_Float)
                .set_input_slot(0)
                .set_offset(Bytes::from(offset_of!(Self, position))),
            InputElementDesc::default()
                .set_name(CString::new("COLOR").unwrap())
                .set_format(DxgiFormat::R32G32B32A32_Float)
                .set_input_slot(0)
                .set_offset(Bytes::from(offset_of!(Self, color))),
        ]
    }
}

#[repr(C)]
struct SceneConstantBuffer {
    offset: Vec4,
    color: Vec4,
}

struct InterprocessCommunicationSample {
    pipeline: Pipeline,
}

impl InterprocessCommunicationSample {
    fn new(hwnd: *mut std::ffi::c_void, is_producer_process: bool) -> Self {
        let mut pipeline = Pipeline::new(hwnd, is_producer_process);
        // pipeline.render();

        InterprocessCommunicationSample { pipeline }
        // InterprocessCommunicationSample { pipeline }
    }

    fn draw(&mut self) {
        // self.pipeline.update();
        self.pipeline.render();
    }
}

impl Drop for InterprocessCommunicationSample {
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
    is_producer_process: bool,
    device: Device,
    debug_device: Option<DebugDevice>,
    info_queue: Option<Rc<InfoQueue>>,
    direct_command_queue: CommandQueue,
    swapchain: DxgiSwapchain,
    frame_index: usize,
    viewport: Viewport,
    scissor_rect: Rect,
    rtv_heap: DescriptorHeap,
    cbv_srv_heap: DescriptorHeap,
    render_targets: Vec<Resource>,
    direct_command_allocators: Vec<CommandAllocator>,
    shared_heap: Heap,
    cross_process_resource: Resource,
    root_signature: RootSignature,
    pipeline_state: PipelineState,

    direct_command_list: CommandList,

    vertex_buffer: Resource,
    vertex_buffer_upload: Resource,
    vertex_buffer_view: VertexBufferView,

    triangle_constant_buffer: Resource,

    frame_resource_fence: Fence,
    frame_resource_fence_event: Win32Event,
    shared_resource_fence: Fence,
    shared_resource_fence_event: Win32Event,
    current_frame_resource_fence_value: u64,
    current_shared_fence_value: u64,
    frame_resources_fence_values: Vec<u64>,
    // shared_fence_values: Vec<u64>,
}

impl Pipeline {
    // aka LoadPipeline() in the original sample
    fn new(hwnd: *mut c_void, is_producer_process: bool) -> Self {
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
        let (device, is_software_adapter) =
            create_device(&factory, !is_producer_process);

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

        let (rtv_heap, cbv_srv_heap) =
            create_descriptor_heaps(&device, &swapchain);

        let (render_targets, direct_command_allocators) =
            create_frame_resources(&device, &rtv_heap, &swapchain);

        let (texture_size, cross_adapter_desc) =
            create_shared_resource_desc(&device);

        let shared_heap;
        if is_producer_process {
            shared_heap = device
                .create_heap(
                    &HeapDesc::default()
                        .set_properties(
                            &HeapProperties::default()
                                .set_type(HeapType::Default),
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
        } else {
            let shared_heap_handle = device
                .open_shared_handle_by_name("SharedHeapHandle")
                .expect("Cannot open SharedHeapHandle");

            shared_heap = device
                .open_shared_heap_handle(shared_heap_handle)
                .expect("Cannot open shared heap");
            shared_heap_handle.close();
        }
        trace!("Successfully created and opened heaps");

        let mut cross_process_resource = device
            .create_placed_resource(
                &shared_heap,
                texture_size,
                &cross_adapter_desc,
                ResourceStates::CommonOrPresent,
                None,
            )
            .expect("Cannot create placed resource");
        cross_process_resource
            .set_name(&format!("shared texture"))
            .expect("Cannot set resource name");

        let root_signature = create_root_signature(&device);

        trace!("Created root signature");

        let pso = create_pso(&device, &root_signature);

        let direct_command_list = create_command_list(
            &device,
            frame_index,
            &direct_command_allocators,
            &pso,
        );

        trace!("Created command list");

        let (vertex_buffer, vertex_buffer_upload, vertex_buffer_view) =
            create_vertex_buffer(&device, &direct_command_list);

        let triangle_constant_buffer =
            create_scene_constant_buffer(&device, is_producer_process);

        trace!("Created triangle constant buffer");
        let (
            frame_resource_fence,
            frame_resource_fence_event,
            shared_resource_fence,
            shared_resource_fence_event,
        ) = create_fences(&device, &direct_command_queue, is_producer_process);

        trace!("Created fences");

        {
            direct_command_list
                .close()
                .expect("Cannot close command list");
            direct_command_queue
                .execute_command_lists(slice::from_ref(&direct_command_list));
            direct_command_queue
                .signal(&frame_resource_fence, 1)
                .expect("Cannot signal fence");
            frame_resource_fence
                .set_event_on_completion(1, &frame_resource_fence_event)
                .expect("Cannot set fence event");

            frame_resource_fence_event.wait();

            // Reset fence value
            direct_command_queue
                .signal(&frame_resource_fence, if is_producer_process { 0 } else { 1 })
                .expect("Cannot signal fence");
        }

        trace!("Executed command lists");

        Self {
            is_producer_process,
            device,
            debug_device,
            info_queue,
            direct_command_queue,
            swapchain,
            frame_index,
            viewport,
            scissor_rect,
            rtv_heap,
            cbv_srv_heap,
            render_targets,
            direct_command_allocators,
            shared_heap,
            cross_process_resource,
            root_signature,
            pipeline_state: pso,
            direct_command_list,
            vertex_buffer,
            vertex_buffer_upload,
            vertex_buffer_view,
            triangle_constant_buffer,

            frame_resource_fence,
            frame_resource_fence_event,
            shared_resource_fence,
            shared_resource_fence_event,
            current_frame_resource_fence_value: 0,
            current_shared_fence_value: if is_producer_process { 0 } else { 1 },
            frame_resources_fence_values: vec![0, 0, 0],
            // shared_fence_values: vec![0, 0, 0],
        }
    }

    fn populate_producer_command_list(&mut self) {
        self.direct_command_allocators[self.frame_index]
            .reset()
            .expect("Cannot reset direct command allocator");

        self.direct_command_list
            .reset(
                &self.direct_command_allocators[self.frame_index],
                Some(&self.pipeline_state),
            )
            .expect("Cannot reset direct command list");

        self.direct_command_list
            .set_graphics_root_signature(&self.root_signature);

        self.direct_command_list
            .set_viewports(slice::from_ref(&self.viewport));

        self.direct_command_list
            .set_scissor_rects(slice::from_ref(&self.scissor_rect));

        self.direct_command_list.resource_barrier(slice::from_ref(
            &ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.render_targets[self.frame_index])
                    .set_state_before(ResourceStates::CommonOrPresent)
                    .set_state_after(ResourceStates::RenderTarget),
            ),
        ));

        let rtv_handle = self
            .rtv_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(Elements::from(self.frame_index));

        self.direct_command_list.set_render_targets(
            slice::from_ref(&rtv_handle),
            false,
            None,
        );

        self.direct_command_list.clear_render_target_view(
            rtv_handle,
            CLEAR_COLOR,
            &[],
        );

        self.direct_command_list
            .set_primitive_topology(PrimitiveTopology::TriangleList);
        self.direct_command_list.set_vertex_buffers(
            Elements(0),
            slice::from_ref(&self.vertex_buffer_view),
        );

        self.direct_command_list
            .set_graphics_root_constant_buffer_view(
                Elements(0),
                GpuVirtualAddress(
                    self.triangle_constant_buffer.get_gpu_virtual_address().0,
                ),
            );

        self.direct_command_list.draw_instanced(
            Elements(3),
            Elements(1),
            Elements(0),
            Elements(0),
        );

        self.direct_command_list.resource_barrier(&[
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.render_targets[self.frame_index])
                    .set_state_before(ResourceStates::RenderTarget)
                    .set_state_after(ResourceStates::CopySource),
            ),
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.cross_process_resource)
                    .set_state_before(ResourceStates::CommonOrPresent)
                    .set_state_after(ResourceStates::CopyDest),
            ),
        ]);

        self.direct_command_list.copy_resource(
            &self.render_targets[self.frame_index],
            &self.cross_process_resource,
        );

        self.direct_command_list.resource_barrier(&[
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.render_targets[self.frame_index])
                    .set_state_before(ResourceStates::CopySource)
                    .set_state_after(ResourceStates::CommonOrPresent),
            ),
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.cross_process_resource)
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::CommonOrPresent),
            ),
        ]);

        self.direct_command_list
            .close()
            .expect("Cannot close command list");
    }

    fn populate_consumer_command_list(&mut self) {
        self.direct_command_allocators[self.frame_index]
            .reset()
            .expect("Cannot reset direct command allocator");

        self.direct_command_list
            .reset(
                &self.direct_command_allocators[self.frame_index],
                Some(&self.pipeline_state),
            )
            .expect("Cannot reset direct command list");

        self.direct_command_list
            .set_graphics_root_signature(&self.root_signature);

        self.direct_command_list
            .set_viewports(slice::from_ref(&self.viewport));

        self.direct_command_list
            .set_scissor_rects(slice::from_ref(&self.scissor_rect));

        self.direct_command_list.resource_barrier(&[
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.render_targets[self.frame_index])
                    .set_state_before(ResourceStates::CommonOrPresent)
                    .set_state_after(ResourceStates::CopyDest),
            ),
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.cross_process_resource)
                    .set_state_before(ResourceStates::CommonOrPresent)
                    .set_state_after(ResourceStates::CopySource),
            ),
        ]);

        self.direct_command_list.copy_resource(
            &self.cross_process_resource,
            &self.render_targets[self.frame_index],
        );

        self.direct_command_list.resource_barrier(&[
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.render_targets[self.frame_index])
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::RenderTarget),
            ),
            ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.cross_process_resource)
                    .set_state_before(ResourceStates::CopySource)
                    .set_state_after(ResourceStates::CommonOrPresent),
            ),
        ]);

        let rtv_handle = self
            .rtv_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(Elements::from(self.frame_index));

        self.direct_command_list.set_render_targets(
            slice::from_ref(&rtv_handle),
            false,
            None,
        );

        self.direct_command_list
            .set_primitive_topology(PrimitiveTopology::TriangleList);
        self.direct_command_list.set_vertex_buffers(
            Elements(0),
            slice::from_ref(&self.vertex_buffer_view),
        );

        self.direct_command_list
            .set_graphics_root_constant_buffer_view(
                Elements(0),
                GpuVirtualAddress(
                    self.triangle_constant_buffer.get_gpu_virtual_address().0,
                ),
            );

        self.direct_command_list.draw_instanced(
            Elements(3),
            Elements(1),
            Elements(0),
            Elements(0),
        );

        self.direct_command_list.resource_barrier(slice::from_ref(
            &ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&self.render_targets[self.frame_index])
                    .set_state_before(ResourceStates::RenderTarget)
                    .set_state_after(ResourceStates::CommonOrPresent),
            ),
        ));

        self.direct_command_list
            .close()
            .expect("Cannot close command list");
    }

    fn render(&mut self) {
        trace!("Rendering frame, idx {}", self.frame_index);
        let completed_shared_resource_fence_value =
            self.shared_resource_fence.get_completed_value();
        if completed_shared_resource_fence_value
            < self.current_shared_fence_value
        {
            self.shared_resource_fence
                .set_event_on_completion(
                    self.current_shared_fence_value,
                    &self.shared_resource_fence_event,
                )
                .expect("Cannot set fence event");

            self.shared_resource_fence_event.wait();
        }

        if self.is_producer_process {
            self.populate_producer_command_list();
        } else {
            self.populate_consumer_command_list();
        }

        self.direct_command_queue
            .execute_command_lists(slice::from_ref(&self.direct_command_list));

        self.current_frame_resource_fence_value += 1;
        self.direct_command_queue
            .signal(
                &self.frame_resource_fence,
                self.current_frame_resource_fence_value,
            )
            .expect("Cannot signal direct command queue");

        self.frame_resources_fence_values[self.frame_index] =
            self.current_frame_resource_fence_value;

        self.current_shared_fence_value += 1;
        self.direct_command_queue
            .signal(
                &self.shared_resource_fence,
                self.current_shared_fence_value,
            )
            .expect("Cannot signal direct command queue");
        // The other process will increase fence value, so we'll wait on it
        self.current_shared_fence_value += 1;

        self.swapchain.present(1, 0).expect("Cannot present");

        self.move_to_next_frame();
    }

    fn move_to_next_frame(&mut self) {
        self.frame_index =
            self.swapchain.get_current_back_buffer_index().0 as usize;

        let completed_frame_fence_value =
            self.frame_resource_fence.get_completed_value();
        if completed_frame_fence_value
            < self.frame_resources_fence_values[self.frame_index]
        {
            self.frame_resource_fence
                .set_event_on_completion(
                    self.frame_resources_fence_values[self.frame_index],
                    &self.frame_resource_fence_event,
                )
                .expect("Cannot set fence event");

            self.frame_resource_fence_event.wait();
        }
    }
}

fn create_fences(
    device: &Device,
    direct_command_queue: &CommandQueue,
    is_producer_process: bool,
) -> (Fence, Win32Event, Fence, Win32Event) {
    let frame_resource_fence = device
        .create_fence(0, FenceFlags::None)
        .expect("Cannot create frame_resource_fence");
    let frame_resource_fence_event = Win32Event::default();

    let shared_resource_fence;
    let shared_resource_fence_event = Win32Event::default();

    if is_producer_process {
        shared_resource_fence = device
            .create_fence(0, FenceFlags::Shared | FenceFlags::CrossAdapter)
            .expect("Cannot create fence");

        let shared_resource_fence_handle = device
            .create_shared_handle(
                &shared_resource_fence.clone().into(),
                "CrossProcessResourceFence",
            )
            .expect(
                "Cannot create shared handle for CrossProcessResourceFence",
            );
    } else {
        let shared_resource_fence_handle = device
            .open_shared_handle_by_name("CrossProcessResourceFence")
            .expect("Cannot open CrossProcessConsumerFence handle");
        shared_resource_fence = device
            .open_shared_fence_handle(shared_resource_fence_handle)
            .expect("Cannot open frame_resource_fence handle");
        shared_resource_fence_handle.close();
    }

    // direct_command_queue
    //     .signal(&cross_adapter_fence, cross_adapter_fence_value)
    //     .expect("Cannot signal command queue");

    // cross_process_fence
    //     .set_event_on_completion(cross_process_fence_value, unsafe {
    //         fence_event
    //     })
    //     .expect("Cannot set event on cross_process_fence");

    // cross_adapter_fence_value += 1;

    (
        frame_resource_fence,
        frame_resource_fence_event,
        shared_resource_fence,
        shared_resource_fence_event,
    )
}

fn create_scene_constant_buffer(
    device: &Device,
    is_producer_process: bool,
) -> Resource {
    let constant_buffer_size =
        Bytes::from(size_of::<SceneConstantBuffer>() as u32);

    let constant_buffer = device
        .create_committed_resource(
            &HeapProperties::default().set_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .set_dimension(ResourceDimension::Buffer)
                .set_layout(TextureLayout::RowMajor)
                .set_width(constant_buffer_size.0.into()),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create constant buffer");
    constant_buffer
        .set_name("Constant buffer")
        .expect("Cannot set name on resource");

    let mut constant_buffer_data = SceneConstantBuffer {
        offset: if is_producer_process {
            vec4(0., 0., 0., 0.)
        } else {
            vec4(1., 0., 0., 0.)
        },
        color: vec4(0.8, 0.1, 0.2, 1.),
    };

    let mapped_data = constant_buffer
        .map(Elements(0), None)
        .expect("Cannot map constant buffer");
    unsafe {
        copy_nonoverlapping(
            &constant_buffer_data,
            mapped_data as *mut SceneConstantBuffer,
            1,
        );
    }

    constant_buffer
}

fn create_vertex_buffer(
    device: &Device,
    direct_command_list: &CommandList,
) -> (Resource, Resource, VertexBufferView) {
    let triangle_vertices = [
        Vertex {
            position: vec4(-1., 0., 0., 1.),
            color: vec4(0.8, 0.2, 0.1, 1.),
        },
        Vertex {
            position: vec4(-0.5, 1., 0., 1.),
            color: vec4(0.8, 0.2, 0.1, 1.),
        },
        Vertex {
            position: vec4(0., 0., 0., 1.),
            color: vec4(0.8, 0.2, 0.1, 1.),
        },
    ];

    let vertex_buffer_size =
        Bytes::from(triangle_vertices.len() * size_of::<Vertex>());

    let vertex_buffer = device
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

    let vertex_buffer_upload = device
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

    direct_command_list
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

    direct_command_list.resource_barrier(slice::from_ref(
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

fn create_pso(
    device: &Device,
    root_signature: &RootSignature,
) -> PipelineState {
    let vertex_shader = compile_shader(
        "VertexShader",
        &std::fs::read_to_string("assets/ic_shaders.hlsl")
            .expect("Cannot open vertex shader file"),
        "VShader",
        "vs_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile vertex shader");
    let pixel_shader = compile_shader(
        "PixelShader",
        &std::fs::read_to_string("assets/ic_shaders.hlsl")
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
        let root_parameters = [RootParameter::default()
            .set_parameter_type(RootParameterType::Cbv)
            .set_shader_visibility(ShaderVisibility::Vertex)
            .set_descriptor(
                &RootDescriptor::default().set_shader_register(Elements(0)),
            )];

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

fn create_shared_resource_desc(device: &Device) -> (Bytes, ResourceDesc) {
    let texture_size: Bytes;
    let cross_adapter_desc = ResourceDesc::default()
        .set_dimension(ResourceDimension::Texture2D)
        .set_layout(TextureLayout::RowMajor)
        .set_format(DxgiFormat::R8G8B8A8_UNorm)
        .set_width(WINDOW_WIDTH.into())
        .set_height(WINDOW_HEIGHT.into())
        .set_flags(ResourceFlags::AllowCrossAdapter);

    let (layout, _, _, _) = device.get_copyable_footprints(
        &cross_adapter_desc,
        0.into(),
        1.into(),
        0.into(),
    );

    texture_size = align_to_multiple(
        (layout[0].0.Footprint.RowPitch * layout[0].0.Footprint.Height) as u64,
        DEFAULT_RESOURCE_ALIGNMENT.0,
    )
    .into();

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
) -> (DescriptorHeap, DescriptorHeap) {
    let num_descriptors = FRAMES_IN_FLIGHT;
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

    (rtv_heap, cbv_srv_heap)
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

fn create_device(factory: &DxgiFactory, use_warp: bool) -> (Device, bool) {
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
    let mut sample = InterprocessCommunicationSample::new(
        window.hwnd(),
        command_args.is_present("primary"),
    );

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