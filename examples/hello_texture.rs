use std::{ffi::CString, rc::Rc, slice::Windows};

use cgmath::{Vector2, Vector3};
use memoffset::offset_of;

use rusty_d3d12::*;

use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowExtWindows,
    window::WindowBuilder,
};

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;
const ASPECT_RATIO: f32 = WINDOW_WIDTH as f32 / WINDOW_HEIGHT as f32;

const TEXTURE_WIDTH: u32 = 256;
const TEXTURE_HEIGHT: u32 = 256;

const FRAMES_IN_FLIGHT: u32 = 3;

const USE_DEBUG: bool = true;
const USE_WARP_ADAPTER: bool = false;

#[no_mangle]
pub static D3D12SDKVersion: u32 = 4;

#[no_mangle]
pub static D3D12SDKPath: &[u8; 9] = b".\\D3D12\\\0";

struct ScopedDebugMessagePrinter {
    info_queue: Rc<InfoQueue>,
}

impl ScopedDebugMessagePrinter {
    fn new(info_queue: Rc<InfoQueue>) -> Self {
        ScopedDebugMessagePrinter { info_queue }
    }
}

impl Drop for ScopedDebugMessagePrinter {
    fn drop(&mut self) {
        self.info_queue
            .print_messages()
            .expect("Cannot print info queue messages");
    }
}

#[repr(C)]
struct Vertex {
    position: Vector3<f32>,
    uv: Vector2<f32>,
}

impl Vertex {
    fn make_desc() -> InputLayout {
        vec![
            InputElementDesc::default()
                .set_name(CString::new("POSITION").unwrap())
                .set_format(DxgiFormat::R32G32B32_Float)
                .set_input_slot(0)
                .set_offset(Bytes(offset_of!(Self, position) as u64)),
            InputElementDesc::default()
                .set_name(CString::new("TEXCOORD").unwrap())
                .set_format(DxgiFormat::R32G32_Float)
                .set_input_slot(0)
                .set_offset(Bytes(offset_of!(Self, uv) as u64)),
        ]
    }
}

struct HelloTextureSample {
    hwnd: *mut std::ffi::c_void,
    device: Device,
    debug_device: DebugDevice,
    info_queue: Rc<InfoQueue>,
    command_queue: CommandQueue,
    fence: Fence,
    fence_event: Win32Event,
    swapchain: DxgiSwapchain,
    frame_index: u32,
    viewport_desc: Viewport,
    scissor_desc: Rect,
    render_targets: Vec<Resource>,
    rtv_heap: DescriptorHeap,
    srv_heap: DescriptorHeap,
    command_allocator: CommandAllocator,
    command_list: CommandList,
    root_signature: RootSignature,
    pso: PipelineState,
    vertex_buffer: Option<Resource>,
    vertex_buffer_view: Option<VertexBufferView>,
    texture: Option<Resource>,
    texture_upload_heap: Option<Resource>,
}

impl HelloTextureSample {
    fn new(hwnd: *mut std::ffi::c_void) -> Self {
        let mut factory_flags = DxgiCreateFactoryFlags::None;
        if USE_DEBUG {
            let debug_controller =
                Debug::new().expect("Cannot create debug controller");
            debug_controller.enable_debug_layer();
            factory_flags = DxgiCreateFactoryFlags::Debug;
        }

        let factory =
            DxgiFactory::new(factory_flags).expect("Cannot create factory");

        let device = create_device(&factory);

        let debug_device =
            DebugDevice::new(&device).expect("Cannot create debug device");

        let info_queue = Rc::new(
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

        let _debug_printer =
            ScopedDebugMessagePrinter::new(Rc::clone(&info_queue));

        let command_queue = device
            .create_command_queue(&CommandQueueDesc::default())
            .expect("Cannot create command queue");

        let fence = device
            .create_fence(0, FenceFlags::None)
            .expect("Cannot create fence");

        let fence_event = Win32Event::default();

        let swapchain = create_swapchain(factory, &command_queue, hwnd);
        let frame_index = swapchain.get_current_back_buffer_index().0 as u32;

        let viewport_desc = Viewport::default()
            .set_width(WINDOW_WIDTH as f32)
            .set_height(WINDOW_HEIGHT as f32);

        let scissor_desc = Rect::default()
            .set_right(WINDOW_WIDTH as i32)
            .set_bottom(WINDOW_HEIGHT as i32);

        let (render_targets, rtv_heap, srv_heap) =
            setup_heaps(&device, &swapchain);

        let command_allocator = device
            .create_command_allocator(CommandListType::Direct)
            .expect("Cannot create command allocator");

        let root_signature = setup_root_signature(&device);

        let (vertex_shader, pixel_shader) = create_shaders();

        let input_layout = Vertex::make_desc();

        let pso = create_pipeline_state(
            input_layout,
            &root_signature,
            vertex_shader,
            pixel_shader,
            &device,
        );

        let command_list = device
            .create_command_list(
                CommandListType::Direct,
                &command_allocator,
                Some(&pso),
                // None,
            )
            .expect("Cannot create command list");

        let mut hello_texture_sample = Self {
            hwnd,
            device,
            debug_device,
            info_queue,
            command_queue,
            fence,
            fence_event,
            swapchain,
            frame_index,
            viewport_desc,
            scissor_desc,
            render_targets,
            rtv_heap,
            srv_heap,
            command_allocator,
            root_signature,
            pso,
            command_list,
            vertex_buffer: None,
            vertex_buffer_view: None,
            texture: None,
            texture_upload_heap: None,
        };

        hello_texture_sample.setup_vertex_buffer();
        hello_texture_sample.setup_texture();
        hello_texture_sample.flush_gpu();

        hello_texture_sample
    }

    fn setup_vertex_buffer(&mut self) {
        let triangle_vertices = vec![
            Vertex {
                position: Vector3::new(0., 0.25 * ASPECT_RATIO, 0.),
                uv: Vector2::new(0.5, 0.),
            },
            Vertex {
                position: Vector3::new(0.25, -0.25 * ASPECT_RATIO, 0.),
                uv: Vector2::new(1., 1.),
            },
            Vertex {
                position: Vector3::new(-0.25, -0.25 * ASPECT_RATIO, 0.),
                uv: Vector2::new(0., 1.),
            },
        ];
        let vertex_buffer_size = Bytes::from(
            triangle_vertices.len() * std::mem::size_of::<Vertex>(),
        );
        let vertex_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(Elements(vertex_buffer_size.0))
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create vertex buffer");

        vertex_buffer
            .set_name("VertexBuffer")
            .expect("Cannot set name on vertex buffer");

        let data = vertex_buffer
            .map(Elements(0), None)
            .expect("Cannot map staging buffer");
        unsafe {
            std::ptr::copy_nonoverlapping(
                triangle_vertices.as_ptr() as *const u8,
                data,
                vertex_buffer_size.0 as usize,
            );
        }
        vertex_buffer.unmap(0, None);
        self.vertex_buffer_view = Some(
            VertexBufferView::default()
                .set_buffer_location(vertex_buffer.get_gpu_virtual_address())
                .set_size_in_bytes(Bytes::from(
                    3 * std::mem::size_of::<Vertex>(),
                ))
                .set_stride_in_bytes(
                    Bytes::from(std::mem::size_of::<Vertex>()),
                ),
        );

        self.vertex_buffer = Some(vertex_buffer);

        self.info_queue
            .print_messages()
            .expect("Cannot get messages from info queue");
    }

    fn setup_texture(&mut self) {
        let texture_width = Elements(TEXTURE_WIDTH as u64);
        let texture_height = Elements(TEXTURE_HEIGHT as u64);
        let pixel_size = Bytes(4);
        self.texture = Some(
            self.device
                .create_committed_resource(
                    &HeapProperties::default().set_type(HeapType::Default),
                    HeapFlags::None,
                    &ResourceDesc::default()
                        .set_format(DxgiFormat::R8G8B8A8_UNorm)
                        .set_width(Elements::from(texture_width.0))
                        .set_height(Elements::from(texture_height.0))
                        .set_dimension(ResourceDimension::Texture2D),
                    ResourceStates::CopyDest,
                    None,
                )
                .expect("Cannot create texture resource"),
        );

        self.texture
            .as_ref()
            .expect("No texture has been created")
            .set_name("Texture")
            .expect("Cannot set name on texture resource");

        self.upload_texture((texture_width, texture_height, pixel_size));

        self.command_list
            .resource_barrier(&vec![ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(
                        self.texture
                            .as_ref()
                            .expect("No texture has been created"),
                    )
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::PixelShaderResource),
            )]);

        self.command_list
            .close()
            .expect("Cannot close command list");

        self.command_queue
            .execute_command_lists(std::slice::from_ref(&self.command_list));
        self.flush_gpu();

        let srv_desc = ShaderResourceViewDesc::default()
            .set_shader4_component_mapping(ShaderComponentMapping::default())
            .set_format(DxgiFormat::R8G8B8A8_UNorm)
            .set_view_dimension(SrvDimension::Texture2D)
            .set_texture_2d(&Tex2DSrv::default().set_mip_levels(Elements(1)));
        self.device.create_shader_resource_view(
            &self.texture.as_ref().expect("No texture has been created"),
            Some(&srv_desc),
            self.srv_heap.get_cpu_descriptor_handle_for_heap_start(),
        );
    }

    fn upload_texture(
        &mut self,
        (texture_width, texture_height, pixel_size): (
            Elements,
            Elements,
            Bytes,
        ),
    ) {
        let upload_buffer_size = self
            .texture
            .as_ref()
            .expect("No texture has been created")
                .get_required_intermediate_size(Elements(0), Elements(1))
                .expect("Cannot get required intermediate size for texture staging buffer");

        self.texture_upload_heap = Some(
            self.device
                .create_committed_resource(
                    &HeapProperties::default().set_type(HeapType::Upload),
                    HeapFlags::None,
                    &ResourceDesc::default()
                        .set_dimension(ResourceDimension::Buffer)
                        .set_width(Elements::from(upload_buffer_size.0))
                        .set_layout(TextureLayout::RowMajor),
                    ResourceStates::GenericRead,
                    None,
                )
                .expect("Cannot create upload buffer"),
        );

        let texture_data =
            generate_texture_data(texture_width, texture_height, pixel_size);

        let texture_subresource_data = SubresourceData::default()
            .set_data(&texture_data)
            // ToDo: clean up these conversions
            .set_row_pitch(Bytes((pixel_size * texture_width).0 as u64))
            .set_slice_pitch(Bytes(
                (pixel_size * texture_width * texture_height).0 as u64,
            ));

        self.command_list
            .update_subresources_heap_alloc(
                &self.texture.as_ref().expect("No texture has been created"),
                &self
                    .texture_upload_heap
                    .as_ref()
                    .expect("No texture staging buffer has been created"),
                Bytes(0),
                Elements(0),
                Elements(1),
                &vec![texture_subresource_data],
            )
            .expect("Cannot update texture");
    }

    fn populate_command_list(&mut self) {
        self.command_allocator
            .reset()
            .expect("Cannot reset command allocator");

        self.command_list
            .reset(&self.command_allocator, Some(&self.pso))
            .expect("Cannot reset command list");

        self.command_list
            .set_graphics_root_signature(&self.root_signature);

        self.command_list
            .set_descriptor_heaps(std::slice::from_mut(&mut self.srv_heap));

        self.command_list.set_graphics_root_descriptor_table(
            Elements(0),
            self.srv_heap.get_gpu_descriptor_handle_for_heap_start(),
        );

        self.command_list.set_viewports(&vec![self.viewport_desc]);
        self.command_list
            .set_scissor_rects(&vec![self.scissor_desc]);

        self.command_list
            .resource_barrier(&vec![ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(
                        &self.render_targets[self.frame_index as usize],
                    )
                    .set_state_before(ResourceStates::CommonOrPresent)
                    .set_state_after(ResourceStates::RenderTarget),
            )]);

        let rtv_handle = self
            .rtv_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(Elements(self.frame_index as u64));

        self.command_list
            .set_render_targets(&mut [rtv_handle], false, None);

        let clear_color: [f32; 4] = [0.0, 0.2, 0.4, 1.0];
        self.command_list.clear_render_target_view(
            rtv_handle,
            clear_color,
            &[],
        );

        self.command_list
            .set_primitive_topology(PrimitiveTopology::TriangleList);
        self.command_list.set_vertex_buffers(
            Elements(0),
            &vec![self.vertex_buffer_view.expect("No vertex buffer created")],
        );

        self.command_list.draw_instanced(
            Elements(3),
            Elements(1),
            Elements(0),
            Elements(0),
        );

        self.command_list
            .resource_barrier(&[ResourceBarrier::transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(
                        &self.render_targets[self.frame_index as usize],
                    )
                    .set_state_before(ResourceStates::RenderTarget)
                    .set_state_after(ResourceStates::CommonOrPresent),
            )]);

        self.command_list
            .close()
            .expect("Cannot close command list");
    }

    fn draw(&mut self) {
        self.populate_command_list();

        self.command_queue
            .execute_command_lists(std::slice::from_ref(&self.command_list));

        self.swapchain.present(1, 0).expect("Cannot present");
        self.flush_gpu();

        self.frame_index = self.swapchain.get_current_back_buffer_index().0 as u32;
    }

    fn flush_gpu(&mut self) {
        let fence_value = self.fence.get_completed_value() + 1;

        self.command_queue
            .signal(&self.fence, fence_value)
            .expect("Cannot signal fence");

        if self.fence.get_completed_value() < fence_value {
            self.fence
                .set_event_on_completion(fence_value, &self.fence_event)
                .expect("Cannot set event on fence");
            self.fence_event.wait();
        }
    }
}

impl Drop for HelloTextureSample {
    fn drop(&mut self) {
        self.debug_device
            .report_live_device_objects()
            .expect("Device cannot report live objects");
    }
}

fn create_pipeline_state(
    input_layout: Vec<InputElementDesc>,
    root_signature: &RootSignature,
    vertex_shader: Vec<u8>,
    pixel_shader: Vec<u8>,
    device: &Device,
) -> PipelineState {
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
        .set_rtv_formats(&[DxgiFormat::R8G8B8A8_UNorm]);

    let pso = device
        .create_graphics_pipeline_state(&pso_desc)
        .expect("Cannot create PSO");
    pso
}

fn create_shaders() -> (Vec<u8>, Vec<u8>) {
    let vertex_shader = compile_shader(
        "VertexShader",
        r"#
struct PSInput
{
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

Texture2D g_texture : register(t0);
SamplerState g_sampler : register(s0);

PSInput VSMain(float4 position : POSITION, float4 uv : TEXCOORD)
{
    PSInput result;

    result.position = position;
    result.uv = uv;

    return result;
}
                #",
        "VSMain",
        "vs_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile vertex shader");

    let pixel_shader = compile_shader(
        "PixelShader",
        r"#
struct PSInput
{
    float4 position : SV_POSITION;
    float2 uv : TEXCOORD;
};

Texture2D g_texture : register(t0);
SamplerState g_sampler : register(s0);

float4 PSMain(PSInput input) : SV_TARGET
{
    return g_texture.Sample(g_sampler, input.uv);
}
                #",
        "PSMain",
        "ps_6_0",
        &[],
        &[],
    )
    .expect("Cannot compile pixel shader");
    (vertex_shader, pixel_shader)
}

fn setup_root_signature(device: &Device) -> RootSignature {
    let mut feature_data =
        FeatureDataRootSignature::new(RootSignatureVersion::V1_1);
    if device
        .check_feature_support(Feature::RootSignature, &mut feature_data)
        .is_err()
    {
        feature_data.set_highest_version(RootSignatureVersion::V1_0);
        unimplemented!(
            "To support v1.0 root signature serialization we'd need to bring \
d3dx12.h as a dependency to have X12SerializeVersionedRootSignature"
        );
    }

    let ranges = vec![DescriptorRange::default()
        .set_range_type(DescriptorRangeType::Srv)
        .set_num_descriptors(Elements(1))
        .set_flags(DescriptorRangeFlags::DataStatic)];

    let root_parameters = vec![RootParameter::default()
        .set_parameter_type(RootParameterType::DescriptorTable)
        .set_descriptor_table(
            &RootDescriptorTable::default().set_descriptor_ranges(&ranges),
        )
        .set_shader_visibility(ShaderVisibility::Pixel)];

    let sampler_desc = StaticSamplerDesc::default()
        .set_filter(Filter::MinMagMipPoint)
        .set_address_u(TextureAddressMode::Border)
        .set_address_v(TextureAddressMode::Border)
        .set_address_w(TextureAddressMode::Border)
        .set_comparison_func(ComparisonFunc::Never)
        .set_border_color(StaticBorderColor::TransparentBlack)
        .set_shader_visibility(ShaderVisibility::Pixel);

    let root_signature_desc = VersionedRootSignatureDesc::default()
        .set_version(RootSignatureVersion::V1_1)
        .set_desc_1_1(
            &RootSignatureDesc::default()
                .set_parameters(&root_parameters)
                .set_static_samplers(&vec![sampler_desc])
                .set_flags(RootSignatureFlags::AllowInputAssemblerInputLayout),
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
        .expect("Cannot create root signature");
    root_signature
}

fn setup_heaps(
    device: &Device,
    swapchain: &DxgiSwapchain,
) -> (Vec<Resource>, DescriptorHeap, DescriptorHeap) {
    let rtv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_type(DescriptorHeapType::RTV)
                .set_num_descriptors(Elements::from(FRAMES_IN_FLIGHT)),
        )
        .expect("Cannot create RTV heap");
    rtv_heap
        .set_name("RTV heap")
        .expect("Cannot set RTV heap name");

    let srv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_type(DescriptorHeapType::CBV_SRV_UAV)
                .set_flags(DescriptorHeapFlags::ShaderVisible)
                .set_num_descriptors(Elements(1)),
        )
        .expect("Cannot create SRV heap");
    srv_heap
        .set_name("SRV heap")
        .expect("Cannot set SRV heap name");

    let mut rtv_handle = rtv_heap.get_cpu_descriptor_handle_for_heap_start();

    let mut render_targets = vec![];
    for frame_idx in 0..FRAMES_IN_FLIGHT {
        let render_target = swapchain
            .get_buffer(Elements::from(frame_idx))
            .expect("Cannot get buffer from swapchain");

        device.create_render_target_view(&render_target, rtv_handle);
        render_targets.push(render_target);

        rtv_handle = rtv_handle.advance(Elements(1));
    }
    (render_targets, rtv_heap, srv_heap)
}

fn create_swapchain(
    factory: DxgiFactory,
    command_queue: &CommandQueue,
    hwnd: *mut std::ffi::c_void,
) -> DxgiSwapchain {
    let swapchain_desc = DxgiSwapchainDesc::default()
        .set_width(WINDOW_WIDTH)
        .set_height(WINDOW_HEIGHT)
        .set_buffer_count(Elements::from(FRAMES_IN_FLIGHT));
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

fn create_device(factory: &DxgiFactory) -> Device {
    let device;
    if USE_WARP_ADAPTER {
        let warp_adapter = factory
            .enum_warp_adapter()
            .expect("Cannot enum warp adapter");
        device = Device::new(&warp_adapter)
            .expect("Cannot create device on WARP adapter");
    } else {
        let hw_adapter = factory
            .enum_adapters()
            .expect("Cannot enumerate adapters")
            .remove(0);
        device = Device::new(&hw_adapter).expect("Cannot create device");
    }
    device
}

fn generate_texture_data(
    width: Elements,
    height: Elements,
    pixel_size: Bytes,
) -> Vec<u8> {
    let row_pitch = width.0 as u32 * pixel_size.0 as u32;
    let cell_pitch = row_pitch >> 3;
    let cell_height = width.0 >> 3;
    let texture_size = row_pitch * height.0 as u32;

    let mut data: Vec<u8> = Vec::with_capacity(texture_size as usize);
    unsafe {
        data.set_len(texture_size as usize);
    }

    for n in (0..texture_size as usize).step_by(pixel_size.0 as usize) {
        let x = n % row_pitch as usize;
        let y = n / row_pitch as usize;
        let i = x / cell_pitch as usize;
        let j = y / cell_height as usize;

        if i % 2 == j % 2 {
            data[n] = 0x00;
            data[n + 1] = 0x00;
            data[n + 2] = 0x00;
            data[n + 3] = 0xff;
        } else {
            data[n] = 0xff;
            data[n + 1] = 0xff;
            data[n + 2] = 0xff;
            data[n + 3] = 0xff;
        }
    }

    data
}

fn main() {
    simple_logger::init_with_level(log::Level::Trace).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Cannot create window");
    window.set_inner_size(winit::dpi::LogicalSize::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
    ));
    let mut sample = HelloTextureSample::new(window.hwnd());

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
                // Redraw the application.
                //
                // It's preferrable to render in this event rather than in MainEventsCleared, since
                // rendering in here allows the program to gracefully handle redraws requested
                // by the OS.

                sample.draw();
            }
            _ => (),
        }
    });
}
