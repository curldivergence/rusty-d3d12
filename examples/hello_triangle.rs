// ToDo: remove these when finished
//#![allow(unused_variables)]
#![allow(dead_code)]

use log::{debug, error, trace, warn};
use memoffset::offset_of;

use rusty_d3d12::*;
#[no_mangle]
pub static D3D12SDKVersion: u32 = 4;

#[no_mangle]
pub static D3D12SDKPath: &[u8; 9] = b".\\D3D12\\\0";

use std::{ffi::CString, rc::Rc};
use widestring::WideCStr;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::windows::WindowExtWindows,
    window::WindowBuilder,
};

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

const WINDOW_WIDTH: u32 = 640;
const WINDOW_HEIGHT: u32 = 480;
const FRAMES_IN_FLIGHT: u32 = 3;

#[repr(C)]
struct Vertex {
    position: [f32; 3],
    color: [f32; 4],
}

impl Vertex {
    fn make_desc() -> InputLayout {
        vec![
            InputElementDesc::default()
                .set_name(CString::new("Position").unwrap())
                .set_format(DxgiFormat::R32G32B32_Float)
                .set_input_slot(0)
                .set_offset(Bytes(offset_of!(Self, position) as u64)),
            InputElementDesc::default()
                .set_name(CString::new("Color").unwrap())
                .set_format(DxgiFormat::R32G32B32A32_Float)
                .set_input_slot(0)
                .set_offset(Bytes(offset_of!(Self, color) as u64)),
        ]
    }
}

trait TypedBuffer {
    type ElementType;
    fn from_resource(
        resource: Resource,
        element_count: Elements,
        element_size: Bytes,
    ) -> Self;
}

struct VertexBuffer {
    buffer: Resource,
    view: VertexBufferView,
    size: Bytes,
}

impl TypedBuffer for VertexBuffer {
    type ElementType = Vertex;

    // note it consumes the resource; should it be revisited?
    fn from_resource(
        buffer: Resource,
        element_count: Elements,
        element_size: Bytes,
    ) -> Self {
        let size = element_size * element_count;
        let view = VertexBufferView::default()
            .set_buffer_location(buffer.get_gpu_virtual_address())
            .set_size_in_bytes(size)
            .set_stride_in_bytes(element_size);
        VertexBuffer {
            buffer: buffer,
            view: view,
            size: size,
        }
    }
}

struct IndexBuffer {
    buffer: Resource,
    view: IndexBufferView,
    size: Bytes,
}

impl TypedBuffer for IndexBuffer {
    type ElementType = u16;

    fn from_resource(
        buffer: Resource,
        element_count: Elements,
        element_size: Bytes,
    ) -> Self {
        let size = element_size * element_count;
        let view = IndexBufferView::new(&buffer, element_count, element_size);
        IndexBuffer {
            buffer: buffer,
            view: view,
            size: size,
        }
    }
}

struct HelloTriangleSample {
    root_signature: Option<RootSignature>,
    pipeline_state: Option<PipelineState>,
    index_buffers: Vec<IndexBuffer>,
    vertex_buffers: Vec<VertexBuffer>,
    current_frame: u64,
    current_fence_value: u64,
    rtv_descriptor_size: u32,
    rtv_heap: DescriptorHeap,
    swapchain: DxgiSwapchain,
    command_list: CommandList,
    command_allocator: CommandAllocator,
    command_queue: CommandQueue,
    fence: Fence,
    info_queue: Rc<InfoQueue>,
    device: Device,
    adapter: DxgiAdapter,
    factory: DxgiFactory,
    debug_layer: Debug,
}

impl HelloTriangleSample {
    pub fn new(hwnd: *mut std::ffi::c_void) -> Result<Self, HRESULT> {
        let debug_layer = Debug::new().expect("Cannot create debug layer");
        debug_layer.enable_debug_layer();

        let mut factory = DxgiFactory::new(DxgiCreateFactoryFlags::Debug)
            .expect("Cannot create factory");
        let adapter = Self::choose_adapter(&mut factory);

        let device = Device::new(&adapter).expect("Cannot create device");

        let info_queue = Rc::new(
            InfoQueue::new(&device, None)
                .expect("Cannot create debug info queue"),
        );

        let _debug_printer =
            ScopedDebugMessagePrinter::new(Rc::clone(&info_queue));

        let fence = device
            .create_fence(0, FenceFlags::None)
            .expect("Cannot create fence");

        let rtv_descriptor_size = device
            .get_descriptor_handle_increment_size(DescriptorHeapType::RTV);

        let command_queue = device
            .create_command_queue(&CommandQueueDesc::default())
            .expect("Cannot create command queue");

        let command_allocator = device
            .create_command_allocator(CommandListType::Direct)
            .expect("Cannot create command allocator");

        let command_list = device
            .create_command_list(
                CommandListType::Direct,
                &command_allocator,
                None,
            )
            .expect("Cannot create command list");
        command_list.close().expect("Cannot close command list");

        let swapchain_desc = DxgiSwapchainDesc::default()
            .set_width(WINDOW_WIDTH)
            .set_height(WINDOW_HEIGHT)
            .set_buffer_count(Elements::from(FRAMES_IN_FLIGHT));

        let swapchain = factory
            .create_swapchain(
                &command_queue,
                hwnd as *mut HWND__,
                &swapchain_desc,
            )
            .expect("Cannot create swapchain");

        let rtv_heap = device
            .create_descriptor_heap(
                &DescriptorHeapDesc::default()
                    .set_type(DescriptorHeapType::RTV)
                    .set_num_descriptors(Elements::from(FRAMES_IN_FLIGHT)),
            )
            .expect("Cannot create RTV heap");

        let mut renderer = HelloTriangleSample {
            root_signature: None,
            pipeline_state: None,
            index_buffers: vec![],
            vertex_buffers: vec![],
            current_frame: 0,
            current_fence_value: 0,
            debug_layer: debug_layer,
            factory: factory,
            adapter: adapter,
            device: device,
            info_queue: info_queue,
            fence: fence,
            rtv_descriptor_size: rtv_descriptor_size,
            command_queue: command_queue,
            command_allocator: command_allocator,
            command_list: command_list,
            swapchain: swapchain,
            rtv_heap: rtv_heap,
        };

        renderer.create_render_target_views();

        let vertex_data = vec![
            Vertex {
                position: [-1., -1., 0.],
                color: [1., 0., 0., 1.],
            },
            Vertex {
                position: [0., 1., 0.],
                color: [0., 1., 0., 1.],
            },
            Vertex {
                position: [1., -1., 0.],
                color: [1., 0., 1., 1.],
            },
        ];

        let vertex_buffer = renderer
            .create_default_buffer(&vertex_data, Some("vertex_buffer"))
            .expect("Cannot create vertex buffer");

        renderer.vertex_buffers.push(vertex_buffer);

        let index_data: Vec<u16> = vec![0, 1, 2];
        let index_buffer = renderer
            .create_default_buffer(&index_data, Some("index_buffer"))
            .expect("Cannot create index buffer");
        renderer.index_buffers.push(index_buffer);

        let raw_vertex_shader_bytecode = HelloTriangleSample::compile_shader(
            "VertexShader",
            r#"
struct VertexIn
{
    float3 pos: Position;
    float4 color: Color;
};

struct VertexOut
{
    float4 pos: SV_POSITION;
    float4 color: Color;
};

[RootSignature("RootFlags(ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT)")]
VertexOut VS(VertexIn input)
{
    VertexOut result = (VertexOut)0;
    result.pos = float4(input.pos, 1.);
    result.color = input.color;

    return result;
}
"#,
            "VS",
            "vs_6_0",
        )
        .expect("Cannot compile vertex shader");
        let vertex_bytecode =
            ShaderBytecode::from_bytes(&raw_vertex_shader_bytecode);

        let raw_pixel_shader_bytecode = HelloTriangleSample::compile_shader(
            "PixelShader",
            r#"
struct VertexOut
{
    float4 pos: SV_Position;
    float4 color: Color;
};

[RootSignature("RootFlags(ALLOW_INPUT_ASSEMBLER_INPUT_LAYOUT)")]
float4 PS(VertexOut input) : SV_Target
{
    return input.color;
}
"#,
            "PS",
            "ps_6_0",
        )
        .expect("Cannot compile pixel shader");
        let pixel_bytecode =
            ShaderBytecode::from_bytes(&raw_pixel_shader_bytecode);

        let root_signature = renderer
            .device
            .create_root_signature(0, &pixel_bytecode)
            .expect("Cannot create root signature");

        debug!("Created root signature");

        renderer.root_signature = Some(root_signature);

        let vertex_desc = Vertex::make_desc();
        let input_layout =
            InputLayoutDesc::default().from_input_layout(&vertex_desc);

        debug!("Created input layout");

        let pso_desc = GraphicsPipelineStateDesc::default()
            .set_vertex_shader_bytecode(&vertex_bytecode)
            .set_pixel_shader_bytecode(&pixel_bytecode)
            .set_blend_state(&BlendDesc::default())
            .set_rasterizer_state(&RasterizerDesc::default())
            .set_depth_stencil_state(&DepthStencilDesc::default())
            .set_input_layout(&input_layout)
            .set_primitive_topology_type(PrimitiveTopologyType::Triangle)
            .set_num_render_targets(Elements(1))
            .set_rtv_formats(&[DxgiFormat::R8G8B8A8_UNorm])
            .set_dsv_format(DxgiFormat::D24_UNorm_S8_UInt);

        let pso = renderer
            .device
            .create_graphics_pipeline_state(&pso_desc)
            .expect("Cannot create PSO");

        debug!("Created PSO");

        renderer.pipeline_state = Some(pso);

        Ok(renderer)
    }

    pub fn draw(&mut self) {
        let _debug_printer =
            ScopedDebugMessagePrinter::new(Rc::clone(&self.info_queue));

        self.command_allocator
            .reset()
            .expect("Cannot reset command allocator");
        self.command_list
            .reset(&self.command_allocator, None)
            .expect("Cannot reset command list");

        let current_buffer_index =
            self.swapchain.get_current_back_buffer_index();
        let current_buffer = self
            .swapchain
            .get_buffer(Elements::from(current_buffer_index))
            .expect("Cannot get current swapchain buffer");

        let rtv_handle = self
            .rtv_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(Elements(current_buffer_index.0 as u64));

        HelloTriangleSample::add_transition(
            &self.command_list,
            &current_buffer,
            ResourceStates::CommonOrPresent,
            ResourceStates::RenderTarget,
        );

        let viewport_desc = Viewport::default()
            .set_width(WINDOW_WIDTH as f32)
            .set_height(WINDOW_HEIGHT as f32);
        self.command_list.set_pipeline_state(
            self.pipeline_state
                .as_ref()
                .expect("No pipeline state found"),
        );
        self.command_list.set_graphics_root_signature(
            self.root_signature
                .as_ref()
                .expect("No root signature to set"),
        );
        self.command_list.set_viewports(&[viewport_desc]);

        let scissor_desc = Rect::default()
            .set_right(WINDOW_WIDTH as i32)
            .set_bottom(WINDOW_HEIGHT as i32);

        self.command_list.set_scissor_rects(&[scissor_desc]);
        self.command_list.clear_render_target_view(
            rtv_handle,
            [0., 0.1, 0.8, 1.],
            &[],
        );
        self.command_list
            .set_render_targets(&mut [rtv_handle], false, None);

        self.command_list
            .set_vertex_buffers(Elements(0), &[self.vertex_buffers[0].view]);

        self.command_list
            .set_index_buffer(&self.index_buffers[0].view);
        self.command_list
            .set_primitive_topology(PrimitiveTopology::TriangleList);
        self.command_list.draw_indexed_instanced(
            Elements(3),
            Elements(1),
            Elements(0),
            0,
            Elements(0),
        );

        HelloTriangleSample::add_transition(
            &self.command_list,
            &current_buffer,
            ResourceStates::RenderTarget,
            ResourceStates::CommonOrPresent,
        );

        self.command_list
            .close()
            .expect("Cannot close command list");
        self.command_queue
            .execute_command_lists(std::slice::from_ref(&self.command_list));

        self.swapchain.present(0, 0).expect("Cannot present frame");

        self.flush_command_queue();

        self.current_frame += 1;
    }
}

// Private methods

impl HelloTriangleSample {
    fn choose_adapter(factory: &mut DxgiFactory) -> DxgiAdapter {
        let mut adapters =
            factory.enum_adapters().expect("Cannot enumerate adapters");
        debug!("Found adapters:");
        for adapter in &adapters {
            let desc_struct =
                adapter.get_desc().expect("Cannot get adapter desc");
            // ToDo: move this inside DxgiAdapterDesc?
            let desc_string =
                WideCStr::from_slice_with_nul(&desc_struct.0.Description)
                    .expect("Cannot parse UTF16 adapter description");
            debug!("\t{}", desc_string.to_string_lossy());
        }

        adapters.remove(0)
    }

    fn create_render_target_views(&self) {
        for buffer_index in 0..(FRAMES_IN_FLIGHT as u32) {
            let rtv_handle = self
                .rtv_heap
                .get_cpu_descriptor_handle_for_heap_start()
                .advance(Elements(buffer_index as u64));
            let mut buffer = self
                .swapchain
                .get_buffer(Elements::from(buffer_index))
                .expect("Cannot obtain swapchain buffer");
            self.device.create_render_target_view(&buffer, rtv_handle);
        }
    }

    fn create_buffer(
        &self,
        device: &Device,
        size: Bytes,
        heap_type: HeapType,
        initial_state: ResourceStates,
    ) -> DxResult<Resource> {
        let heap_props = HeapProperties::default().set_type(heap_type);
        let resource_desc = ResourceDesc::default()
            .set_dimension(ResourceDimension::Buffer)
            .set_width(Elements::from(size.0))
            .set_layout(TextureLayout::RowMajor);

        device.create_committed_resource(
            &heap_props,
            HeapFlags::None,
            &resource_desc,
            initial_state,
            None,
        )
    }

    fn add_transition(
        command_list: &CommandList,
        resource: &Resource,
        from: ResourceStates,
        to: ResourceStates,
    ) {
        command_list.resource_barrier(&[ResourceBarrier::transition(
            &ResourceTransitionBarrier::default()
                .set_resource(resource)
                .set_state_before(from)
                .set_state_after(to),
        )]);
    }

    fn create_default_buffer<T: TypedBuffer>(
        &mut self,
        init_data: &Vec<T::ElementType>,
        debug_name: Option<&str>,
    ) -> Result<T, HRESULT> {
        let _debug_printer =
            ScopedDebugMessagePrinter::new(Rc::clone(&self.info_queue));

        self.command_list
            .reset(&self.command_allocator, None)
            .expect("Cannot reset command lsit");

        let size = Bytes::from(
            init_data.len() * std::mem::size_of::<T::ElementType>(),
        );
        let staging_buffer = self
            .create_buffer(
                &self.device,
                size,
                HeapType::Upload,
                ResourceStates::GenericRead,
            )
            .expect("Cannot create staging buffer");

        if let Some(debug_name) = debug_name {
            staging_buffer
                .set_name(&format!("staging_{}", debug_name))
                .expect("Cannot set name on staging buffer");
        }

        let data = staging_buffer
            .map(Elements(0), None)
            .expect("Cannot map staging buffer");

        unsafe {
            std::ptr::copy_nonoverlapping(
                init_data.as_ptr() as *const u8,
                data,
                size.0 as usize,
            );
        }
        staging_buffer.unmap(0, None);

        let default_buffer = self
            .create_buffer(
                &self.device,
                size,
                HeapType::Default,
                ResourceStates::CommonOrPresent,
            )
            .expect("Cannot create default buffer");

        if let Some(debug_name) = debug_name {
            default_buffer
                .set_name(&format!("default_{}", debug_name))
                .expect("Cannot set name on default buffer");
        }

        HelloTriangleSample::add_transition(
            &self.command_list,
            &default_buffer,
            ResourceStates::CommonOrPresent,
            ResourceStates::CopyDest,
        );

        self.command_list.copy_buffer_region(
            &default_buffer,
            Bytes(0),
            &staging_buffer,
            Bytes(0),
            size,
        );

        HelloTriangleSample::add_transition(
            &self.command_list,
            &default_buffer,
            ResourceStates::CopyDest,
            ResourceStates::GenericRead,
        );

        self.command_list
            .close()
            .expect("Cannot close command list");

        self.command_queue
            .execute_command_lists(std::slice::from_ref(&self.command_list));
        self.flush_command_queue();

        Ok(T::from_resource(
            default_buffer,
            Elements::from(init_data.len()),
            Bytes::from(std::mem::size_of::<T::ElementType>()),
        ))
    }

    fn flush_command_queue(&mut self) {
        self.current_fence_value += 1;
        self.command_queue
            .signal(&self.fence, self.current_fence_value)
            .expect("Cannot signal fence from command queue");
        if self.fence.get_completed_value() < self.current_fence_value {
            let event_handle = Win32Event::default();
            self.fence
                .set_event_on_completion(
                    self.current_fence_value,
                    &event_handle,
                )
                .expect("Cannot set fence completion event");
            event_handle.wait();
            event_handle.close();
        }
    }

    fn compile_shader(
        name: &str,
        source: &str,
        entry_point: &str,
        shader_model: &str,
    ) -> Result<Vec<u8>, String> {
        let result = hassle_rs::utils::compile_hlsl(
            name,
            source,
            entry_point,
            shader_model,
            &[],
            &[],
        );
        match result {
            Ok(bytecode) => {
                debug!("Shader {} compiled successfully", name);
                Ok(bytecode)
            }
            Err(error) => {
                error!("Cannot compile shader: {}", &error);
                Err(error)
            }
        }
    }
}

impl Drop for HelloTriangleSample {
    fn drop(&mut self) {
        self.info_queue
            .print_messages()
            .expect("Cannot print info queue messages");
        debug!("Renderer destroyed");
    }
}

fn main() {
    let command_args = clap::App::new("Hobbiton")
        .arg(
            clap::Arg::with_name("frame_count")
                .short("f")
                .takes_value(true)
                .value_name("NUMBER")
                .help("Run <frame_count> frames and exit"),
        )
        .arg(
            clap::Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Verbosity level"),
        )
        .get_matches();

    let frame_count = command_args
        .value_of("frame_count")
        .unwrap_or(&std::u64::MAX.to_string())
        .parse::<u64>()
        .expect("Cannot parse frame count");
    let log_level: log::Level;
    match command_args.occurrences_of("v") {
        0 => log_level = log::Level::Info,
        1 => log_level = log::Level::Debug,
        2 | _ => log_level = log::Level::Trace,
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
    let mut sample = HelloTriangleSample::new(window.hwnd())
        .expect("Cannot create renderer");

    let mut current_frame: u64 = 0;
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                println!("The close button was pressed; stopping");
                *control_flow = ControlFlow::Exit
            }
            Event::MainEventsCleared => {
                // Application update code.
                if current_frame > frame_count {
                    *control_flow = ControlFlow::Exit;
                }
                // Queue a RedrawRequested event.
                window.request_redraw();
            }
            Event::RedrawRequested(_) => {
                // Redraw the application.
                //
                // It's preferrable to render in this event rather than in MainEventsCleared, since
                // rendering in here allows the program to gracefully handle redraws requested
                // by the OS.

                sample.draw();
                current_frame += 1;
            }
            _ => (),
        }
    });
}
