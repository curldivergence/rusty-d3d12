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
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::sync::mpsc::TryRecvError;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

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
pub static D3D12SDKVersion: u32 = 600;

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

#[derive(Debug, Copy, Clone)]
pub struct Radians(pub f32);

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
    color: Vec4,
}

impl Vertex {
    pub fn make_desc() -> Vec<InputElementDesc<'static>> {
        vec![InputElementDesc::default()
            .with_semantic_name("COLOR")
            .unwrap()
            .with_format(Format::R32G32B32A32Float)
            .with_input_slot(0)
            .with_aligned_byte_offset(ByteCount::from(offset_of!(Self, color)))]
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
    }

    fn draw(&mut self) {
        self.pipeline.update();
        self.pipeline.render();
    }
}

impl Drop for NBodyGravitySample {
    fn drop(&mut self) {
        self.pipeline
            .graphics_tx
            .send(None)
            .expect("Cannot shutdown graphics thread");

        self.pipeline
            .graphics_thread
            .take()
            .expect("Graphics thread was already joined")
            .join()
            .expect("Cannot join graphics thread");

        self.pipeline
            .compute_tx
            .send(None)
            .expect("Cannot shutdown compute thread");

        self.pipeline
            .compute_thread
            .take()
            .expect("Compute thread was already joined")
            .join()
            .expect("Cannot join compute thread");

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

#[derive(Debug)]
enum ContextType {
    Graphics,
    Compute,
}

struct GraphicsContext {
    direct_command_queue: CommandQueue,
    render_targets: Vec<Resource>,
    direct_command_allocators: Vec<CommandAllocator>,
    graphics_root_signature: RootSignature,
    graphics_pso: PipelineState,
    direct_command_list: CommandList,
    vertex_buffer: Resource,
    vertex_buffer_upload: Resource,
    vertex_buffer_view: VertexBufferView,
    constant_buffer_gs: Resource,
    constant_buffer_gs_mapped_data: *mut u8,
}

impl GraphicsContext {
    fn create_thread(
        render_start_rx: Receiver<Option<usize>>,
        render_finish_tx: Sender<ContextType>,
        device: Device,
        direct_command_queue: CommandQueue,
        initial_frame_index: usize,
        heaps: [DescriptorHeap; 2],
        render_targets: Vec<Resource>,

        vertex_buffer: Resource,
        vertex_buffer_upload: Resource,
        vertex_buffer_view: VertexBufferView,

        constant_buffer_gs: Resource,
        constant_buffer_gs_mapped_data: usize, //*mut u8,

        producer_fence: Fence,
        consumer_fence: Fence,
        frame_fence: Fence,
        frame_fence_value: Arc<AtomicU64>,

        rtv_descriptor_handle_size: ByteCount,
        cbv_srv_descriptor_handle_size: ByteCount,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let constant_buffer_gs_mapped_data =
                constant_buffer_gs_mapped_data as *mut u8;

            let viewport = Viewport::default()
                .with_top_left_x(0.)
                .with_top_left_y(0.)
                .with_width(WINDOW_WIDTH as f32)
                .with_height(WINDOW_HEIGHT as f32);

            let scissor_rect = Rect::default()
                .with_left(0)
                .with_top(0)
                .with_right(WINDOW_WIDTH as i32)
                .with_bottom(WINDOW_HEIGHT as i32);

            let graphics_root_signature =
                create_graphics_root_signature(&device);
            trace!("Created graphics root signature");

            let graphics_pso =
                create_graphics_pso(&device, &graphics_root_signature);

            let direct_command_allocators = (0..FRAMES_IN_FLIGHT)
                .map(|idx| {
                    let alloc = device
                        .create_command_allocator(CommandListType::Direct)
                        .expect("Cannot create graphics command allocator");
                    alloc
                        .set_name(&format!(
                            "graphics command allocator {}",
                            idx
                        ))
                        .expect("Cannot set name on command allocator");
                    alloc
                })
                .collect::<Vec<_>>();

            let direct_command_list = device
                .create_command_list(
                    CommandListType::Direct,
                    &direct_command_allocators[initial_frame_index],
                    Some(&graphics_pso),
                )
                .expect("Cannot create direct command list");

            direct_command_list
                .close()
                .expect("Cannot close command list");
            trace!("Created direct command list");

            let mut context = Self {
                direct_command_queue,
                render_targets,
                direct_command_allocators,
                graphics_root_signature,
                graphics_pso,
                direct_command_list,
                vertex_buffer,
                vertex_buffer_upload,
                vertex_buffer_view,
                constant_buffer_gs,
                constant_buffer_gs_mapped_data,
            };

            let mut producer_fence_value = 0;
            let mut consumer_fence_value = 1;

            loop {
                let frame_idx = match render_start_rx.recv() {
                    Ok(idx) => idx,
                    Err(_) => panic!("main thread destroyed its channel why async thread was alive")
                };

                if let Some(frame_idx) = frame_idx {
                    trace!(
                        "Graphics thread received message, frame #{}",
                        frame_idx
                    );

                    context.update(frame_idx);

                    context.direct_command_allocators[frame_idx]
                        .reset()
                        .expect("Cannot reset graphics command allocator");

                    context
                        .direct_command_list
                        .reset(
                            &context.direct_command_allocators[frame_idx],
                            Some(&context.graphics_pso),
                        )
                        .expect("Cannot reset direct command list");

                    context
                        .direct_command_list
                        .set_pipeline_state(&context.graphics_pso);

                    context.direct_command_list.set_graphics_root_signature(
                        &context.graphics_root_signature,
                    );

                    context
                        .direct_command_list
                        .set_graphics_root_constant_buffer_view(
                            GRAPHICS_ROOT_CBV,
                            GpuVirtualAddress(
                                context
                                    .constant_buffer_gs
                                    .get_gpu_virtual_address()
                                    .0
                                    + (size_of!(ConstantBufferGs) * frame_idx)
                                        .0,
                            ),
                        );

                    // srv_uav heap
                    context
                        .direct_command_list
                        .set_descriptor_heaps(slice::from_ref(&heaps[1]));

                    context.direct_command_list.set_vertex_buffers(
                        0,
                        std::slice::from_ref(&vertex_buffer_view),
                    );

                    context
                        .direct_command_list
                        .set_primitive_topology(PrimitiveTopology::PointList);

                    context
                        .direct_command_list
                        .set_viewports(slice::from_ref(&viewport));

                    context
                        .direct_command_list
                        .set_scissor_rects(slice::from_ref(&scissor_rect));

                    context.direct_command_list.resource_barrier(
                        slice::from_ref(&ResourceBarrier::new_transition(
                            &ResourceTransitionBarrier::default()
                                .with_resource(
                                    &context.render_targets[frame_idx],
                                )
                                .with_state_before(ResourceStates::Common)
                                .with_state_after(ResourceStates::RenderTarget),
                        )),
                    );

                    let rtv_handle = heaps[0]
                        .get_cpu_descriptor_handle_for_heap_start()
                        .advance(frame_idx as u32, rtv_descriptor_handle_size);
                    context.direct_command_list.set_render_targets(
                        slice::from_ref(&rtv_handle),
                        false,
                        None,
                    );

                    context.direct_command_list.clear_render_target_view(
                        rtv_handle,
                        CLEAR_COLOR,
                        &[],
                    );

                    let srv_index = if frame_idx == 0 {
                        SRV_PARTICLE_POS_VEL_0
                    } else {
                        SRV_PARTICLE_POS_VEL_1
                    };

                    let srv_handle = heaps[1]
                        .get_gpu_descriptor_handle_for_heap_start()
                        .advance(srv_index, cbv_srv_descriptor_handle_size);

                    context
                        .direct_command_list
                        .set_graphics_root_descriptor_table(
                            GRAPHICS_ROOT_SRV_TABLE,
                            srv_handle,
                        );

                    context.direct_command_list.draw_instanced(
                        PARTICLE_COUNT,
                        1,
                        0,
                        0,
                    );

                    context.direct_command_list.resource_barrier(
                        slice::from_ref(&ResourceBarrier::new_transition(
                            &ResourceTransitionBarrier::default()
                                .with_resource(
                                    &context.render_targets[frame_idx],
                                )
                                .with_state_before(ResourceStates::RenderTarget)
                                .with_state_after(ResourceStates::Common),
                        )),
                    );

                    context
                        .direct_command_list
                        .close()
                        .expect("Cannot close graphics command list");

                    // gpu wait for compute to simulate particles

                    trace!(
                        "Graphics queue: waiting on producer fence value {}",
                        producer_fence_value
                    );

                    context
                        .direct_command_queue
                        .wait(&producer_fence, producer_fence_value)
                        .expect("Cannot wait on queue");
                    producer_fence_value += 1;

                    context.direct_command_queue.execute_command_lists(
                        slice::from_ref(&context.direct_command_list),
                    );

                    trace!(
                        "Graphics queue: signaling consumer fence value {}",
                        consumer_fence_value
                    );

                    context
                        .direct_command_queue
                        .signal(&consumer_fence, consumer_fence_value)
                        .expect("Cannot signal queue");
                    consumer_fence_value += 1;

                    context
                        .direct_command_queue
                        .signal(
                            &frame_fence,
                            frame_fence_value.fetch_add(1, Ordering::SeqCst),
                        )
                        .expect("Cannot signal queue");

                    trace!(
                            "Graphics thread finished rendering frame #{} (signaled frame fence with value {})",
                            frame_idx,
                            frame_fence_value.load(Ordering::SeqCst) // race condition
                        );

                    render_finish_tx.send(ContextType::Graphics).expect(
                        "Cannot send rendering finish from graphics thread",
                    );
                } else {
                    trace!("Shutting down graphics thread");
                    break;
                }
            }
        })
    }

    fn update(&mut self, frame_index: usize) {
        let camera = Camera::default();

        let world = Mat4::identity();
        let view = make_view_matrix(camera.position, camera.look_at);
        let proj = make_projection_matrix(&camera);

        let cb_data = ConstantBufferGs {
            wvp: proj * view * world,
            inverse_view: view.invert().expect("No inverse for view matrix"),
            _padding: unsafe { std::mem::zeroed() },
        };

        unsafe {
            copy_nonoverlapping(
                &cb_data,
                (self.constant_buffer_gs_mapped_data as *mut ConstantBufferGs)
                    .add(frame_index),
                1,
            );
        }
    }
}

struct ComputeContext {
    compute_command_queue: CommandQueue,
    compute_command_allocators: Vec<CommandAllocator>,
    compute_command_list: CommandList,
    compute_root_signature: RootSignature,
    compute_pso: PipelineState,
}

impl ComputeContext {
    fn create_thread(
        render_start_rx: Receiver<Option<usize>>,
        render_finish_tx: Sender<ContextType>,
        device: Device,
        initial_frame_index: usize,
        uavs: [Resource; 2],
        srv_uav_heap: DescriptorHeap,
        cs_cbuffer: Resource,
        producer_fence: Fence,
        consumer_fence: Fence,
        frame_fence: Fence,
        frame_fence_value: Arc<AtomicU64>,
        cbv_srv_descriptor_handle_size: ByteCount,
    ) -> JoinHandle<()> {
        thread::spawn(move || {
            let compute_command_queue = device
                .create_command_queue(
                    &CommandQueueDesc::default()
                        .with_queue_type(CommandListType::Compute),
                )
                .expect("Cannot create compute command queue");

            let compute_command_allocators = (0..FRAMES_IN_FLIGHT)
                .map(|idx| {
                    let alloc = device
                        .create_command_allocator(CommandListType::Compute)
                        .expect("Cannot create compute command allocator");
                    alloc
                        .set_name(&format!("compute command allocator {}", idx))
                        .expect("Cannot set name on command allocator");
                    alloc
                })
                .collect::<Vec<_>>();

            let compute_command_list = device
                .create_command_list(
                    CommandListType::Compute,
                    &compute_command_allocators[initial_frame_index],
                    None,
                )
                .expect("Cannot create compute command list");

            compute_command_list
                .close()
                .expect("Cannot close command list");
            let compute_root_signature = create_compute_root_signature(&device);
            let compute_pso =
                create_compute_pso(&device, &compute_root_signature);

            let context = Self {
                compute_command_queue,
                compute_command_allocators,
                compute_command_list,
                compute_root_signature,
                compute_pso,
            };

            let mut producer_fence_value = 1;
            let mut consumer_fence_value = 0;

            loop {
                let frame_idx = match render_start_rx.recv() {
                    Ok(idx) => idx,
                    Err(_) => panic!("main thread destroyed its channel why async thread was alive")
                };

                if let Some(frame_idx) = frame_idx {
                    trace!(
                        "Compute thread received message, frame #{}",
                        frame_idx
                    );

                    context.compute_command_allocators[initial_frame_index]
                        .reset()
                        .expect("Cannot reset command allocator");

                    context
                        .compute_command_list
                        .reset(
                            &context.compute_command_allocators[frame_idx],
                            Some(&context.compute_pso),
                        )
                        .expect("Cannot reset compute command list");

                    simulate(
                        frame_idx as u8,
                        &uavs,
                        &context.compute_command_list,
                        &context.compute_pso,
                        &context.compute_root_signature,
                        &srv_uav_heap,
                        &cs_cbuffer,
                        cbv_srv_descriptor_handle_size,
                    );

                    context
                        .compute_command_list
                        .close()
                        .expect("Cannot close compute command list");

                    // ToDo: pix marker

                    // gpu wait for graphics to finish rendering previous srv

                    // if consumer_fence_value > 1 {
                    trace!(
                        "Compute queue: waiting on consumer fence value {}",
                        consumer_fence_value
                    );
                    context
                        .compute_command_queue
                        .wait(&consumer_fence, consumer_fence_value)
                        .expect("Cannot wait on queue");
                    // }
                    consumer_fence_value += 1;

                    context.compute_command_queue.execute_command_lists(
                        slice::from_ref(&context.compute_command_list),
                    );

                    trace!(
                        "Compute queue: signaling producer fence value {}",
                        producer_fence_value
                    );
                    context
                        .compute_command_queue
                        .signal(&producer_fence, producer_fence_value)
                        .expect("Cannot signal queue");
                    producer_fence_value += 1;

                    context
                        .compute_command_queue
                        .signal(
                            &frame_fence,
                            frame_fence_value.fetch_add(1, Ordering::SeqCst),
                        )
                        .expect("Cannot signal queue");

                    trace!(
                        "Compute thread finished rendering frame #{} (signaled frame fence with value {})",
                        frame_idx,
                        frame_fence_value.load(Ordering::SeqCst) // race condition
                    );

                    render_finish_tx.send(ContextType::Compute).expect(
                        "Cannot send rendering finish from compute thread",
                    );
                } else {
                    trace!("Shutting down compute thread");
                    break;
                }
            }
        })
    }
}

fn create_compute_root_signature(device: &Device) -> RootSignature {
    let srv_range = DescriptorRange::default()
        .with_range_type(DescriptorRangeType::Srv)
        .with_num_descriptors(1)
        .with_base_shader_register(0)
        .with_flags(DescriptorRangeFlags::DescriptorsVolatile);

    let srv_table = RootDescriptorTable::default()
        .with_descriptor_ranges(slice::from_ref(&srv_range));

    let uav_range = DescriptorRange::default()
        .with_range_type(DescriptorRangeType::Uav)
        .with_num_descriptors(1)
        .with_base_shader_register(0)
        .with_flags(DescriptorRangeFlags::DataVolatile);

    let uav_table = RootDescriptorTable::default()
        .with_descriptor_ranges(slice::from_ref(&uav_range));
    let root_parameters = [
        RootParameter::default()
            .new_descriptor(
                &RootDescriptor::default()
                    .with_shader_register(0)
                    .with_flags(RootDescriptorFlags::DataStatic),
                RootParameterType::Cbv,
            )
            .with_shader_visibility(ShaderVisibility::All),
        RootParameter::default().new_descriptor_table(&srv_table),
        RootParameter::default().new_descriptor_table(&uav_table),
    ];
    let root_signature_desc = VersionedRootSignatureDesc::default()
        .with_desc_1_1(
            &RootSignatureDesc::default()
                .with_parameters(&root_parameters)
                .with_flags(RootSignatureFlags::AllowInputAssemblerInputLayout),
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
            &ShaderBytecode::new(serialized_signature.get_buffer()),
        )
        .expect("Cannot create root signature on device 0");
    root_signature
}

struct Pipeline {
    device: Device,
    debug_device: Option<DebugDevice>,
    info_queue: Option<Rc<InfoQueue>>,
    swapchain: Swapchain,
    swapchain_event: Win32Event,
    frame_index: usize,
    rtv_heap: DescriptorHeap,
    rtv_descriptor_handle_size: ByteCount,

    srv_uav_heap: DescriptorHeap,
    cbv_srv_descriptor_handle_size: ByteCount,

    graphics_thread: Option<JoinHandle<()>>,
    graphics_tx: Sender<Option<usize>>,

    compute_thread: Option<JoinHandle<()>>,
    compute_tx: Sender<Option<usize>>,

    render_finish_rx: Receiver<ContextType>,

    particle_buffer_0: Resource,
    particle_buffer_0_upload: Resource,
    particle_buffer_1: Resource,
    particle_buffer_1_upload: Resource,
    particle_buffer_index: u8,

    constant_buffer_cs: Resource,
    constant_buffer_cs_upload: Resource,

    producer_fence: Fence,
    producer_fence_value: u64,
    producer_fence_event: Win32Event,

    consumer_fence: Fence,
    consumer_fence_value: u64,
    consumer_fence_event: Win32Event,
    // frame_fence_values: [u64; FRAMES_IN_FLIGHT],
    frame_fence: Fence,
    frame_fence_value: Arc<AtomicU64>,
    last_frame_fence_value: u64,
    frame_fence_event: Win32Event,
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

            #[cfg(feature = "debug_callback")]
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
                    .with_queue_type(CommandListType::Direct),
            )
            .expect("Cannot create direct command queue");
        let swapchain = create_swapchain(factory, &direct_command_queue, hwnd);
        let frame_index = swapchain.get_current_back_buffer_index() as usize;
        trace!("Swapchain returned frame index {}", frame_index);

        let swapchain_event = swapchain.get_frame_latency_waitable_object();

        let (rtv_heap, srv_uav_heap) =
            create_descriptor_heaps(&device, &swapchain);

        let rtv_descriptor_handle_size = device
            .get_descriptor_handle_increment_size(DescriptorHeapType::Rtv);
        let cbv_srv_descriptor_handle_size = device
            .get_descriptor_handle_increment_size(
                DescriptorHeapType::CbvSrvUav,
            );

        let render_targets = create_render_targets(
            &device,
            &rtv_heap,
            &swapchain,
            rtv_descriptor_handle_size,
        );

        let (
            producer_fence,
            producer_fence_event,
            consumer_fence,
            consumer_fence_event,
            frame_fence,
            frame_fence_event,
        ) = create_fences(&device);

        let temp_command_allocator = device
            .create_command_allocator(CommandListType::Direct)
            .expect("Cannot create command allocator");

        let temp_command_list = device
            .create_command_list(
                CommandListType::Direct,
                &temp_command_allocator,
                None,
            )
            .expect("Cannot create direct command list");

        let (
            particle_buffer_0,
            particle_buffer_0_upload,
            particle_buffer_1,
            particle_buffer_1_upload,
        ) = create_particle_buffers(
            &device,
            &temp_command_list,
            &srv_uav_heap,
            rtv_descriptor_handle_size,
            cbv_srv_descriptor_handle_size,
        );

        trace!("Created partice buffers");

        let (vertex_buffer, vertex_buffer_upload, vertex_buffer_view) =
            create_vertex_buffer(&device, &temp_command_list);
        trace!("Created vertex buffer");

        let (constant_buffer_gs, constant_buffer_gs_mapped_data) =
            create_gs_constant_buffer(&device, &temp_command_list);
        trace!("Created gs constant buffer");

        let (constant_buffer_cs, constant_buffer_cs_upload) =
            create_cs_constant_buffer(&device, &temp_command_list);

        trace!("Created cs constant buffer");

        let frame_fence_value = 1;
        {
            temp_command_list
                .close()
                .expect("Cannot close command list");
            direct_command_queue
                .execute_command_lists(slice::from_ref(&temp_command_list));

            direct_command_queue
                .signal(&frame_fence, frame_fence_value)
                .expect("Cannot signal fence");

            frame_fence
                .set_event_on_completion(frame_fence_value, &frame_fence_event)
                .expect("Cannot set fence event");
            frame_fence_event.wait(None);

            frame_fence.signal(0).expect("Cannot reset frame fence");
        }

        trace!("Executed command lists");

        let frame_fence_value = Arc::new(AtomicU64::new(0));
        let (graphics_render_start_tx, graphics_render_start_rx) =
            mpsc::channel();
        let (render_finish_tx, render_finish_rx) =
            mpsc::channel::<ContextType>();
        let graphics_thread = GraphicsContext::create_thread(
            graphics_render_start_rx,
            render_finish_tx.clone(),
            device.clone(),
            direct_command_queue,
            frame_index,
            [rtv_heap.clone(), srv_uav_heap.clone()],
            render_targets,
            vertex_buffer,
            vertex_buffer_upload,
            vertex_buffer_view,
            constant_buffer_gs,
            constant_buffer_gs_mapped_data as usize,
            producer_fence.clone(),
            consumer_fence.clone(),
            frame_fence.clone(),
            frame_fence_value.clone(),
            rtv_descriptor_handle_size,
            cbv_srv_descriptor_handle_size,
        );

        info!("Created graphics context");

        let (compute_tx, compute_rx) = mpsc::channel();
        let compute_thread = ComputeContext::create_thread(
            compute_rx,
            render_finish_tx,
            device.clone(),
            frame_index,
            [particle_buffer_0.clone(), particle_buffer_1.clone()],
            srv_uav_heap.clone(),
            constant_buffer_cs.clone(),
            producer_fence.clone(),
            consumer_fence.clone(),
            frame_fence.clone(),
            frame_fence_value.clone(),
            cbv_srv_descriptor_handle_size,
        );

        info!("Created compute context");

        Self {
            device,
            debug_device,
            info_queue,
            swapchain,
            swapchain_event,
            frame_index,
            rtv_heap,
            srv_uav_heap,

            graphics_thread: Some(graphics_thread),
            graphics_tx: graphics_render_start_tx,

            compute_thread: Some(compute_thread),
            compute_tx,

            render_finish_rx,

            particle_buffer_0,
            particle_buffer_0_upload,
            particle_buffer_1,
            particle_buffer_1_upload,
            particle_buffer_index: 0,

            constant_buffer_cs,
            constant_buffer_cs_upload,

            producer_fence,
            producer_fence_value: 0,
            producer_fence_event,

            consumer_fence,
            consumer_fence_value: 0,
            consumer_fence_event,

            frame_fence,
            frame_fence_value,
            last_frame_fence_value: 0,
            frame_fence_event,
            rtv_descriptor_handle_size,
            cbv_srv_descriptor_handle_size,
        }
    }

    fn render(&mut self) {
        trace!("Rendering frame, idx {}", self.frame_index);

        self.graphics_tx
            .send(Some(self.frame_index))
            .expect("Cannot send message to graphics thread");

        self.compute_tx
            .send(Some(self.frame_index))
            .expect("Cannot send message to compute thread");

        let context_count = 2; // graphics and compute
        for _ in 0..context_count {
            match self.render_finish_rx.recv() {
                Ok(ctx_type) => {
                    trace!("Context {:?} finished rendering", ctx_type)
                }
                Err(_) => panic!("Failed to query rendering finish msg"),
            }
        }

        self.swapchain
            .present(0, PresentFlags::None)
            .unwrap_or_else(|err| {
                error!("{}", err);
                let real_error = self.device.get_device_removed_reason();
                error!("Device removed reason: {}", real_error);
            });

        // cpu wait for command allocators

        // let temp_ff_value = self.frame_fence_value.load(Ordering::SeqCst);
        // if temp_ff_value > 0 {
        if self.last_frame_fence_value > 1 {
            trace!(
                "render(): waiting for frame fence value {}",
                self.last_frame_fence_value
            );
            let completed_frame_fence_value =
                self.frame_fence.get_completed_value();

            trace!(
                "completed frame fence value {}",
                completed_frame_fence_value
            );

            if completed_frame_fence_value < self.last_frame_fence_value {
                self.frame_fence
                    .set_event_on_completion(
                        self.last_frame_fence_value,
                        &self.frame_fence_event,
                    )
                    .expect("Cannot set fence event");

                self.frame_fence_event.wait(None);
            }
        }
        self.last_frame_fence_value += 2;

        self.frame_index = (self.frame_index + 1) % FRAMES_IN_FLIGHT;
    }

    // fn move_to_next_frame(&mut self) {
    //     self.frame_index =
    //         self.swapchain.get_current_back_buffer_index() as usize;

    // }

    fn update(&mut self) {
        // self.swapchain_event.wait(Some(100));
    }
}

fn simulate(
    resource_selector: u8,
    uavs: &[Resource; 2],
    compute_command_list: &CommandList,
    pso: &PipelineState,
    root_sig: &RootSignature,
    srv_uav_heap: &DescriptorHeap,
    constant_buffer: &Resource,
    cbv_srv_descriptor_handle_size: ByteCount,
) {
    let curr_srv_index;
    let curr_uav_index;
    let curr_uav;
    if resource_selector == 0 {
        curr_srv_index = SRV_PARTICLE_POS_VEL_0;
        curr_uav_index = UAV_PARTICLE_POS_VEL_1;
        curr_uav = &uavs[1];
    } else {
        curr_srv_index = SRV_PARTICLE_POS_VEL_1;
        curr_uav_index = UAV_PARTICLE_POS_VEL_0;
        curr_uav = &uavs[0];
    }
    compute_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .with_resource(curr_uav)
                .with_state_before(ResourceStates::NonPixelShaderResource)
                .with_state_after(ResourceStates::UnorderedAccess),
        ),
    ));
    compute_command_list.set_pipeline_state(pso);
    compute_command_list.set_compute_root_signature(root_sig);
    compute_command_list.set_descriptor_heaps(slice::from_ref(srv_uav_heap));
    let srv_handle = srv_uav_heap
        .get_gpu_descriptor_handle_for_heap_start()
        .advance(curr_srv_index, cbv_srv_descriptor_handle_size);
    let uav_handle = srv_uav_heap
        .get_gpu_descriptor_handle_for_heap_start()
        .advance(curr_uav_index, cbv_srv_descriptor_handle_size);
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
                .with_resource(curr_uav)
                .with_state_before(ResourceStates::UnorderedAccess)
                .with_state_after(ResourceStates::NonPixelShaderResource),
        ),
    ));
}
fn create_fences(
    device: &Device,
) -> (Fence, Win32Event, Fence, Win32Event, Fence, Win32Event) {
    let producer_fence = device
        .create_fence(0, FenceFlags::None)
        .expect("Cannot create producer_fence");
    producer_fence
        .set_name("producer fence")
        .expect("Cannot set name on fence");
    let producer_fence_event = Win32Event::default();

    let consumer_fence = device
        .create_fence(0, FenceFlags::None)
        .expect("Cannot create consumer_fence");
    consumer_fence
        .set_name("consumer fence")
        .expect("Cannot set name on fence");
    let consumer_fence_event = Win32Event::default();

    let frame_fence = device
        .create_fence(0, FenceFlags::None)
        .expect("Cannot create frame_fence");
    frame_fence
        .set_name("frame fence")
        .expect("Cannot set name on fence");

    let frame_fence_event = Win32Event::default();

    (
        producer_fence,
        producer_fence_event,
        consumer_fence,
        consumer_fence_event,
        frame_fence,
        frame_fence_event,
    )
}

fn create_gs_constant_buffer(
    device: &Device,
    direct_command_list: &CommandList,
) -> (Resource, *mut u8) {
    let buffer_size = size_of!(ConstantBufferGs) * FRAMES_IN_FLIGHT;

    let constant_buffer_gs = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(buffer_size.0.into()),
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

    (constant_buffer_gs, mapped_data)
}

fn create_cs_constant_buffer(
    device: &Device,
    direct_command_list: &CommandList,
) -> (Resource, Resource) {
    let buffer_size = size_of!(ConstantBufferCs);

    let constant_buffer_cs = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(buffer_size.0.into()),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create constant_buffer_cs");
    constant_buffer_cs
        .set_name("constant_buffer_cs")
        .expect("Cannot set name on resource");

    let constant_buffer_cs_upload = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(buffer_size.0.into()),
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
        .with_data(slice::from_ref(&cb_data))
        .with_row_pitch(size_of!(ConstantBufferCs))
        .with_slice_pitch(size_of!(ConstantBufferCs));

    direct_command_list
        .update_subresources_heap_alloc(
            &constant_buffer_cs,
            &constant_buffer_cs_upload,
            ByteCount(0),
            0,
            1,
            slice::from_ref(&subresource_data),
        )
        .expect("Cannot upload vertex buffer");
    trace!("Uploaded vertex buffer");

    direct_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .with_resource(&constant_buffer_cs)
                .with_state_before(ResourceStates::CopyDest)
                .with_state_after(ResourceStates::VertexAndConstantBuffer),
        ),
    ));

    trace!("Created CS constant buffer");

    (constant_buffer_cs, constant_buffer_cs_upload)
}

fn create_particle_buffers(
    device: &Device,
    direct_command_list: &CommandList,
    srv_uav_heap: &DescriptorHeap,
    rtv_descriptor_handle_size: ByteCount,
    cbv_uav_descriptor_handle_size: ByteCount,
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
            &HeapProperties::default().with_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(data_size.0)
                .with_flags(ResourceFlags::AllowUnorderedAccess),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create particle_buffer_0");
    particle_buffer_0
        .set_name("particle_buffer_0")
        .expect("Cannot set name on resource");

    let particle_buffer_1 = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(data_size.0)
                .with_flags(ResourceFlags::AllowUnorderedAccess),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create vertex_buffer");
    particle_buffer_1
        .set_name("particle_buffer_1")
        .expect("Cannot set name on resource");

    let particle_buffer_0_upload = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(data_size.0),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create particle_buffer_0_upload");
    particle_buffer_0_upload
        .set_name("particle_buffer_0_upload")
        .expect("Cannot set name on resource");

    let particle_buffer_1_upload = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(data_size.0),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create particle_buffer_1_upload");
    particle_buffer_1_upload
        .set_name("particle_buffer_1_upload")
        .expect("Cannot set name on resource");

    let particle_data = SubresourceData::default()
        .with_data(&data)
        .with_row_pitch(data_size)
        .with_slice_pitch(data_size);

    direct_command_list
        .update_subresources_heap_alloc(
            &particle_buffer_0,
            &particle_buffer_0_upload,
            ByteCount(0),
            0,
            1,
            slice::from_ref(&particle_data),
        )
        .expect("Cannot upload particle buffer");

    direct_command_list
        .update_subresources_heap_alloc(
            &particle_buffer_1,
            &particle_buffer_1_upload,
            ByteCount(0),
            0,
            1,
            slice::from_ref(&particle_data),
        )
        .expect("Cannot upload particle buffer");
    trace!("Uploaded particle buffers");

    direct_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .with_resource(&particle_buffer_0)
                .with_state_before(ResourceStates::CopyDest)
                .with_state_after(ResourceStates::NonPixelShaderResource),
        ),
    ));

    direct_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .with_resource(&particle_buffer_1)
                .with_state_before(ResourceStates::CopyDest)
                .with_state_after(ResourceStates::NonPixelShaderResource),
        ),
    ));

    let srv_desc = ShaderResourceViewDesc::default()
        .new_buffer(
            &BufferSrv::default()
                .with_first_element(0)
                .with_num_elements(PARTICLE_COUNT)
                .with_structure_byte_stride(size_of!(Particle)),
        )
        .with_shader_4_component_mapping(ShaderComponentMapping::default());

    device.create_shader_resource_view(
        &particle_buffer_0,
        Some(&srv_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(SRV_PARTICLE_POS_VEL_0, cbv_uav_descriptor_handle_size),
    );
    device.create_shader_resource_view(
        &particle_buffer_1,
        Some(&srv_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(SRV_PARTICLE_POS_VEL_1, cbv_uav_descriptor_handle_size),
    );

    let uav_desc = UnorderedAccessViewDesc::default().new_buffer(
        &BufferUav::default()
            .with_first_element(0)
            .with_num_elements(PARTICLE_COUNT)
            .with_structure_byte_stride(size_of!(Particle))
            .with_counter_offset_in_bytes(ByteCount(0)),
    );

    device.create_unordered_access_view(
        &particle_buffer_0,
        None,
        Some(&uav_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(UAV_PARTICLE_POS_VEL_0, cbv_uav_descriptor_handle_size),
    );

    device.create_unordered_access_view(
        &particle_buffer_1,
        None,
        Some(&uav_desc),
        srv_uav_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(UAV_PARTICLE_POS_VEL_1, cbv_uav_descriptor_handle_size),
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

    let vertex_buffer_size = size_of!(Vertex) * particle_vertices.len();

    let vertex_buffer = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Default),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(vertex_buffer_size.0),
            ResourceStates::CopyDest,
            None,
        )
        .expect("Cannot create vertex_buffer");
    vertex_buffer
        .set_name("Vertex buffer")
        .expect("Cannot set name on resource");

    let vertex_buffer_upload = device
        .create_committed_resource(
            &HeapProperties::default().with_heap_type(HeapType::Upload),
            HeapFlags::None,
            &ResourceDesc::default()
                .with_dimension(ResourceDimension::Buffer)
                .with_layout(TextureLayout::RowMajor)
                .with_width(vertex_buffer_size.0.into()),
            ResourceStates::GenericRead,
            None,
        )
        .expect("Cannot create vertex_buffer");
    vertex_buffer_upload
        .set_name("Vertex buffer upload")
        .expect("Cannot set name on resource");

    let vertex_data = SubresourceData::default()
        .with_data(&particle_vertices)
        .with_row_pitch(vertex_buffer_size)
        .with_slice_pitch(vertex_buffer_size);

    direct_command_list
        .update_subresources_heap_alloc(
            &vertex_buffer,
            &vertex_buffer_upload,
            ByteCount(0),
            0,
            1,
            slice::from_ref(&vertex_data),
        )
        .expect("Cannot upload vertex buffer");
    trace!("Uploaded vertex buffer");

    direct_command_list.resource_barrier(slice::from_ref(
        &ResourceBarrier::new_transition(
            &ResourceTransitionBarrier::default()
                .with_resource(&vertex_buffer)
                .with_state_before(ResourceStates::CopyDest)
                .with_state_after(ResourceStates::VertexAndConstantBuffer),
        ),
    ));

    let vertex_buffer_view = VertexBufferView::default()
        .with_buffer_location(vertex_buffer.get_gpu_virtual_address())
        .with_stride_in_bytes(size_of!(Vertex))
        .with_size_in_bytes(vertex_buffer_size);
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

fn create_graphics_pso(
    device: &Device,
    graphics_root_signature: &RootSignature,
) -> PipelineState {
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

    let vs_bytecode = ShaderBytecode::new(&vertex_shader);
    let gs_bytecode = ShaderBytecode::new(&geometry_shader);
    let ps_bytecode = ShaderBytecode::new(&pixel_shader);

    let input_layout =
        InputLayoutDesc::default().with_input_elements(&input_layout);
    let graphics_pso_desc = GraphicsPipelineStateDesc::default()
        .with_input_layout(&input_layout)
        .with_root_signature(graphics_root_signature)
        .with_vs_bytecode(&vs_bytecode)
        .with_gs_bytecode(&gs_bytecode)
        .with_ps_bytecode(&ps_bytecode)
        .with_rasterizer_state(
            RasterizerDesc::default().with_depth_clip_enable(false),
        )
        .with_blend_state(
            BlendDesc::default().with_render_targets(slice::from_ref(
                &RenderTargetBlendDesc::default()
                    .with_blend_enable(true)
                    .with_src_blend(Blend::SrcAlpha)
                    .with_dest_blend(Blend::One)
                    .with_src_blend_alpha(Blend::Zero)
                    .with_dest_blend_alpha(Blend::Zero),
            )),
        )
        .with_depth_stencil_state(
            DepthStencilDesc::default().with_depth_enable(false),
        )
        .with_primitive_topology_type(PrimitiveTopologyType::Point)
        .with_rtv_formats(&[Format::R8G8B8A8Unorm])
        .with_dsv_format(Format::D32Float);

    let graphics_pso = device
        .create_graphics_pipeline_state(&graphics_pso_desc)
        .expect("Cannot create PSO");
    graphics_pso
        .set_name("graphics_pso")
        .expect("Cannot set name on pso");
    trace!("Created graphics_pso");

    graphics_pso
}

fn create_compute_pso(
    device: &Device,
    compute_root_signature: &RootSignature,
) -> PipelineState {
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
    let cs_bytecode = ShaderBytecode::new(&compute_shader);
    let compute_pso_desc = ComputePipelineStateDesc::default()
        .with_root_signature(compute_root_signature)
        .with_cs_bytecode(&cs_bytecode);
    let compute_pso = device
        .create_compute_pipeline_state(&compute_pso_desc)
        .expect("Cannot create compute PSO");
    trace!("Created compute_pso");
    compute_pso
}

fn create_graphics_root_signature(device: &Device) -> RootSignature {
    let graphics_root_signature = {
        let ranges = [DescriptorRange::default()
            .with_range_type(DescriptorRangeType::Srv)
            .with_num_descriptors(1)
            .with_base_shader_register(0)
            .with_flags(DescriptorRangeFlags::DataStatic)];

        let table =
            RootDescriptorTable::default().with_descriptor_ranges(&ranges);
        let root_parameters = [
            RootParameter::default()
                .new_descriptor(
                    &RootDescriptor::default()
                        .with_shader_register(0)
                        .with_flags(RootDescriptorFlags::DataStatic),
                    RootParameterType::Cbv,
                )
                .with_shader_visibility(ShaderVisibility::All),
            RootParameter::default().new_descriptor_table(&table),
        ];

        let root_signature_desc = VersionedRootSignatureDesc::default()
            .with_desc_1_1(
                &RootSignatureDesc::default()
                    .with_parameters(&root_parameters)
                    .with_flags(
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
                &ShaderBytecode::new(serialized_signature.get_buffer()),
            )
            .expect("Cannot create root signature on device 0");
        root_signature
    };

    info!("created graphics root signature");

    graphics_root_signature
}

fn create_render_targets(
    device: &Device,
    rtv_heap: &DescriptorHeap,
    swapchain: &Swapchain,
    rtv_uav_descriptor_handle_size: ByteCount,
) -> Vec<Resource> {
    let clear_value = ClearValue::default()
        .with_format(Format::R8G8B8A8Unorm)
        .with_color(CLEAR_COLOR);

    let render_target_desc = ResourceDesc::default()
        .with_dimension(ResourceDimension::Texture2D)
        .with_format(Format::R8G8B8A8Unorm)
        .with_width(WINDOW_WIDTH.into())
        .with_height(WINDOW_HEIGHT)
        .with_flags(ResourceFlags::AllowRenderTarget);

    let mut render_targets = vec![];

    for frame_idx in 0..FRAMES_IN_FLIGHT {
        render_targets.push(
            swapchain
                .get_buffer(frame_idx as u32)
                .expect("Cannot get buffer from swapchain"),
        );
    }

    let mut rtv_handle = rtv_heap.get_cpu_descriptor_handle_for_heap_start();
    for frame_idx in 0..FRAMES_IN_FLIGHT {
        device
            .create_render_target_view(&render_targets[frame_idx], rtv_handle);

        rtv_handle = rtv_handle.advance(1, rtv_uav_descriptor_handle_size);
    }

    trace!("created command allocators");

    render_targets
}

fn create_descriptor_heaps(
    device: &Device,
    swapchain: &Swapchain,
) -> (DescriptorHeap, DescriptorHeap) {
    let rtv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .with_heap_type(DescriptorHeapType::Rtv)
                .with_num_descriptors(FRAMES_IN_FLIGHT as u32),
        )
        .expect("Cannot create RTV heap");
    rtv_heap
        .set_name("RTV heap")
        .expect("Cannot set RTV heap name");

    let srv_uav_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .with_heap_type(DescriptorHeapType::CbvSrvUav)
                .with_num_descriptors(DESCRIPTOR_COUNT)
                .with_flags(DescriptorHeapFlags::ShaderVisible),
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
    let swapchain_desc = SwapChainDesc::default()
        .with_width(WINDOW_WIDTH)
        .with_height(WINDOW_HEIGHT)
        .with_buffer_count(FRAMES_IN_FLIGHT as u32);
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
