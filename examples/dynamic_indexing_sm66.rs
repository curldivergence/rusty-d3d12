use log::{error, trace};
use std::{
    borrow::Borrow,
    ffi::CString,
    intrinsics::copy_nonoverlapping,
    io::Read,
    mem::{size_of, zeroed},
    rc::Rc,
    slice,
};

use cgmath::{prelude::*, Vector4};
use cgmath::{Matrix4, Vector3};

use memoffset::offset_of;

use rusty_d3d12::*;

#[no_mangle]
pub static D3D12SDKVersion: u32 = 4;

#[no_mangle]
pub static D3D12SDKPath: &[u8; 9] = b".\\D3D12\\\0";

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

const USE_BUNDLES: bool = true;

mod sample_assets {
    use cgmath::{Vector2, Vector3};
    use memoffset::offset_of;
    use rusty_d3d12::*;
    use std::{ffi::CString, io::Read, rc::Rc, slice::Windows};

    pub const PATH_TO_CITY_BIN: &'static str = "assets/occcity.bin";

    #[repr(C)]
    pub struct Vertex {
        position: Vector3<f32>,
        normal: Vector3<f32>,
        uv: Vector2<f32>,
        tangent: Vector3<f32>,
    }

    impl Vertex {
        pub fn make_desc() -> Vec<InputElementDesc<'static>> {
            vec![
                InputElementDesc::default()
                    .set_name("POSITION")
                    .unwrap()
                    .set_format(Format::R32G32B32_Float)
                    .set_input_slot(0)
                    .set_offset(ByteCount::from(offset_of!(Self, position))),
                InputElementDesc::default()
                    .set_name("NORMAL")
                    .unwrap()
                    .set_format(Format::R32G32B32_Float)
                    .set_input_slot(0)
                    .set_offset(ByteCount::from(offset_of!(Self, normal))),
                InputElementDesc::default()
                    .set_name("TEXCOORD")
                    .unwrap()
                    .set_format(Format::R32G32_Float)
                    .set_input_slot(0)
                    .set_offset(ByteCount::from(offset_of!(Self, uv))),
                InputElementDesc::default()
                    .set_name("TANGENT")
                    .unwrap()
                    .set_format(Format::R32G32B32_Float)
                    .set_input_slot(0)
                    .set_offset(ByteCount::from(offset_of!(Self, tangent))),
            ]
        }
    }

    #[derive(Default)]
    #[repr(C)]
    pub struct DataProperties {
        pub offset: u32,
        pub size: u32,
        pub pitch: u32,
    }

    impl DataProperties {
        const fn zeroed() -> Self {
            Self {
                offset: 0,
                size: 0,
                pitch: 0,
            }
        }
    }

    #[repr(C)]
    pub struct TextureResource {
        pub width: u32,
        pub height: u32,
        pub mip_levels: u16,
        pub format: Format,
        pub data: [DataProperties; D3D12_REQ_MIP_LEVELS as usize],
    }

    pub const TEXTURES: [TextureResource; 1] = {
        [TextureResource {
            width: 1024,
            height: 1024,
            mip_levels: 1,
            format: Format::BC1_UNorm,
            data: [
                DataProperties {
                    offset: 0,
                    size: 524288,
                    pitch: 2048,
                },
                // No way to partially initialize an array in Rust
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
                DataProperties::zeroed(),
            ],
        }]
    };

    pub const CITY_ROW_COUNT: u32 = 15;
    pub const CITY_COLUMN_COUNT: u32 = 8;
    pub const CITY_MATERIAL_COUNT: u32 = CITY_ROW_COUNT * CITY_COLUMN_COUNT;

    pub const CITY_MATERIAL_TEXTURE_WIDTH: u32 = 64;
    pub const CITY_MATERIAL_TEXTURE_HEIGHT: u32 = 64;
    pub const CITY_MATERIAL_TEXTURE_CHANNEL_COUNT: u32 = 4;
    pub const CITY_SPACING_INTERVAL: f32 = 16.;

    pub const VERTEX_DATA_OFFSET: ByteCount = ByteCount(524288);
    pub const VERTEX_DATA_SIZE: ByteCount = ByteCount(820248);

    pub const INDEX_DATA_OFFSET: ByteCount = ByteCount(1344536);
    pub const INDEX_DATA_SIZE: ByteCount = ByteCount(74568);
}

use sample_assets::*;

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

macro_rules! make_debug_printer {
    ($info_queue:expr) => {
        ScopedDebugMessagePrinter::new(Rc::clone(&$info_queue));
    };
}

struct DynamicIndexingSample {
    device: Device,
    debug_device: DebugDevice,
    info_queue: Rc<InfoQueue>,
    command_queue: CommandQueue,
    fence: Fence,
    fence_value: u64,
    fence_event: Win32Event,
    swapchain: Swapchain,
    frame_index: u32,
    viewport_desc: Viewport,
    scissor_desc: Rect,
    render_targets: Vec<Resource>,
    rtv_heap: DescriptorHeap,
    dsv_heap: DescriptorHeap,
    cbv_srv_heap: DescriptorHeap,
    cbv_srv_descriptor_handle_size: ByteCount,
    rtv_descriptor_handle_size: ByteCount,
    sampler_heap: DescriptorHeap,
    command_allocator: CommandAllocator,
    command_list: CommandList,
    root_signature: RootSignature,
    pso: PipelineState,
    mesh_data: Vec<u8>,

    vertex_buffer: Option<Resource>,
    vertex_staging_buffer: Option<Resource>,
    vertex_buffer_view: Option<VertexBufferView>,

    index_buffer: Option<Resource>,
    index_staging_buffer: Option<Resource>,
    index_buffer_view: Option<IndexBufferView>,
    index_count: u32,

    texture_staging_buffer: Option<Resource>,

    city_material_textures: Vec<Resource>,
    materials_staging_buffer: Option<Resource>,

    city_diffuse_texture: Option<Resource>,

    depth_stencil: Option<Resource>,

    frame_number: u64,
    current_frame_resource_index: u32,
    frame_resources: Vec<FrameResource>,
}

impl DynamicIndexingSample {
    fn new(hwnd: *mut std::ffi::c_void) -> Self {
        // d3d_enable_experimental_shader_models()
        //     .expect("Cannot enable experimental shader models");

        let mut factory_flags = CreateFactoryFlags::None;
        if USE_DEBUG {
            let debug_controller =
                Debug::new().expect("Cannot create debug controller");
            debug_controller.enable_debug_layer();
            factory_flags = CreateFactoryFlags::Debug;
        }

        let factory =
            Factory::new(factory_flags).expect("Cannot create factory");

        let device = create_device(&factory);

        let debug_device =
            DebugDevice::new(&device).expect("Cannot create debug device");

        let info_queue = Rc::new(
            InfoQueue::new(
                &device,
                Some(&[
                    MessageSeverity::Corruption,
                    MessageSeverity::Error,
                    MessageSeverity::Warning,
                ]),
                // None,
            )
            .expect("Cannot create debug info queue"),
        );

        let _debug_printer = make_debug_printer!(&info_queue);

        let command_queue = device
            .create_command_queue(&CommandQueueDesc::default())
            .expect("Cannot create command queue");

        let fence = device
            .create_fence(0, FenceFlags::None)
            .expect("Cannot create fence");

        let fence_event = Win32Event::default();

        let swapchain = create_swapchain(factory, &command_queue, hwnd);
        let frame_index = swapchain.get_current_back_buffer_index() as u32;

        let viewport_desc = Viewport::default()
            .set_width(WINDOW_WIDTH as f32)
            .set_height(WINDOW_HEIGHT as f32);

        let scissor_desc = Rect::default()
            .set_right(WINDOW_WIDTH as i32)
            .set_bottom(WINDOW_HEIGHT as i32);

        let cbv_srv_descriptor_handle_size = device
            .get_descriptor_handle_increment_size(
                DescriptorHeapType::CbvSrvUav,
            );

        let rtv_descriptor_handle_size = device
            .get_descriptor_handle_increment_size(DescriptorHeapType::Rtv);

        let (render_targets, rtv_heap, dsv_heap, cbv_srv_heap, sampler_heap) =
            setup_heaps(&device, &swapchain, rtv_descriptor_handle_size);

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

        let mesh_data = {
            let mut bin_file = std::fs::File::open(PATH_TO_CITY_BIN)
                .expect("Cannot open asset file");
            let mut temp_buffer = Vec::new();
            bin_file
                .read_to_end(&mut temp_buffer)
                .expect("Cannot read asset file");
            temp_buffer
        };

        let mut dynamic_indexing_sample = Self {
            device,
            debug_device,
            info_queue,
            command_queue,
            fence,
            fence_value: 0,
            fence_event,
            swapchain,
            frame_index,
            viewport_desc,
            scissor_desc,
            render_targets,
            rtv_heap,
            dsv_heap,
            cbv_srv_heap,
            cbv_srv_descriptor_handle_size,
            rtv_descriptor_handle_size,
            sampler_heap,
            command_allocator,
            root_signature,
            pso,
            command_list,
            mesh_data,

            vertex_buffer: None,
            vertex_staging_buffer: None,
            vertex_buffer_view: None,

            index_buffer: None,
            index_staging_buffer: None,
            index_buffer_view: None,
            index_count: 0,

            texture_staging_buffer: None,

            city_material_textures: vec![],
            materials_staging_buffer: None,

            city_diffuse_texture: None,

            depth_stencil: None,

            frame_number: 0,
            current_frame_resource_index: 0,
            frame_resources: vec![],
        };

        dynamic_indexing_sample.setup_vertex_buffer();
        dynamic_indexing_sample.setup_index_buffer();
        dynamic_indexing_sample.setup_textures();
        dynamic_indexing_sample.setup_dsv();

        dynamic_indexing_sample
            .command_list
            .close()
            .expect("Cannot close command list for initial resource setup");

        dynamic_indexing_sample.command_queue.execute_command_lists(
            slice::from_ref(&dynamic_indexing_sample.command_list),
        );

        dynamic_indexing_sample.flush_gpu();

        dynamic_indexing_sample
            .command_list
            .reset(
                &dynamic_indexing_sample.command_allocator,
                Some(&dynamic_indexing_sample.pso),
            )
            .expect("Cannot reset command list");

        dynamic_indexing_sample.create_frame_resources();

        dynamic_indexing_sample
            .command_list
            .close()
            .expect("Cannot close command list for initial resource setup");

        dynamic_indexing_sample.command_queue.execute_command_lists(
            slice::from_ref(&dynamic_indexing_sample.command_list),
        );

        dynamic_indexing_sample.flush_gpu();

        dynamic_indexing_sample
    }

    fn create_frame_resources(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let mut cbv_srv_handle = self
            .cbv_srv_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(
                (CITY_MATERIAL_COUNT + 1).into(),
                self.cbv_srv_descriptor_handle_size,
            );

        for frame_idx in 0..FRAMES_IN_FLIGHT {
            let mut frame_resource = FrameResource::new(&self.device);

            let mut cb_offset = ByteCount(0);
            let cb_size = ByteCount::from(size_of::<SceneConstantBuffer>());

            for _ in 0..CITY_ROW_COUNT {
                for _ in 0..CITY_COLUMN_COUNT {
                    let cbv_desc = ConstantBufferViewDesc::default()
                        .set_buffer_location(GpuVirtualAddress(
                            frame_resource
                                .cbv_staging_buffer
                                .get_gpu_virtual_address()
                                .0
                                + cb_offset.0,
                        ))
                        .set_size_in_bytes(cb_size);

                    cb_offset += cb_size;

                    self.device
                        .create_constant_buffer_view(&cbv_desc, cbv_srv_handle);

                    cbv_srv_handle = cbv_srv_handle
                        .advance(1, self.cbv_srv_descriptor_handle_size);
                }
            }

            frame_resource.init_bundle(
                &self.device,
                &self.pso,
                frame_idx.into(),
                self.index_count,
                self.index_buffer_view
                    .as_ref()
                    .expect("No index buffer view"),
                self.vertex_buffer_view
                    .as_ref()
                    .expect("No vertex buffer view"),
                &self.cbv_srv_heap,
                &self.sampler_heap,
                &self.root_signature,
                self.cbv_srv_descriptor_handle_size,
            );

            self.frame_resources.push(frame_resource);
        }
    }

    fn setup_dsv(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let depth_stencil_desc = DepthStencilViewDesc::default()
            .set_format(Format::D32_Float)
            .set_view_dimension(DsvDimension::Texture2D)
            .set_flags(DsvFlags::None);

        let depth_stencil = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Default),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Texture2D)
                    .set_width(WINDOW_WIDTH.into())
                    .set_height(WINDOW_HEIGHT.into())
                    .set_format(Format::D32_Float)
                    .set_flags(
                        ResourceFlags::AllowDepthStencil
                            | ResourceFlags::DenyShaderResource,
                    ),
                ResourceStates::DepthWrite,
                Some(
                    &ClearValue::default()
                        .set_format(Format::D32_Float)
                        .set_depth_stencil(
                            &DepthStencilValue::default()
                                .set_depth(1.)
                                .set_stencil(0),
                        ),
                ),
            )
            .expect("Cannot create depth stencil resource");

        depth_stencil
            .set_name("DepthStencil")
            .expect("Cannot set name on depth stencil");

        self.device.create_depth_stencil_view(
            &depth_stencil,
            &depth_stencil_desc,
            self.dsv_heap.get_cpu_descriptor_handle_for_heap_start(),
        );

        self.depth_stencil = Some(depth_stencil);
    }

    fn setup_vertex_buffer(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let vertex_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Default),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(VERTEX_DATA_SIZE.0.into())
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::CopyDest,
                None,
            )
            .expect("Cannot create vertex buffer");

        vertex_buffer
            .set_name("VertexBuffer")
            .expect("Cannot set name on vertex buffer");

        let vertex_staging_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(VERTEX_DATA_SIZE.0.into())
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create vertex staging buffer");

        vertex_staging_buffer
            .set_name("VertexStagingBuffer")
            .expect("Cannot set name on vertex staging buffer");

        let vertex_subresource_data = SubresourceData::default()
            .set_data(
                &self.mesh_data[VERTEX_DATA_OFFSET.into()
                    ..(VERTEX_DATA_OFFSET + VERTEX_DATA_SIZE).into()],
            )
            .set_row_pitch(ByteCount(VERTEX_DATA_SIZE.0))
            .set_slice_pitch(ByteCount(VERTEX_DATA_SIZE.0));

        self.command_list
            .update_subresources_heap_alloc(
                &vertex_buffer,
                &vertex_staging_buffer,
                ByteCount(0),
                0,
                1,
                &[vertex_subresource_data],
            )
            .expect("Cannot update vertex buffer");

        self.command_list
            .resource_barrier(&[ResourceBarrier::new_transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&vertex_buffer)
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::VertexAndConstantBuffer),
            )]);

        let vertex_count = VERTEX_DATA_SIZE.0 / size_of::<Vertex>() as u64;

        assert_eq!(vertex_count, 18642);

        self.vertex_buffer_view = Some(
            VertexBufferView::default()
                .set_buffer_location(vertex_buffer.get_gpu_virtual_address())
                .set_size_in_bytes(ByteCount::from(
                    vertex_count * size_of::<Vertex>() as u64,
                ))
                .set_stride_in_bytes(ByteCount::from(size_of::<Vertex>())),
        );

        self.vertex_buffer = Some(vertex_buffer);
        self.vertex_staging_buffer = Some(vertex_staging_buffer);

        self.info_queue
            .print_messages()
            .expect("Cannot get messages from info queue");
    }

    fn setup_index_buffer(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let index_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Default),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(INDEX_DATA_SIZE.0.into())
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::CopyDest,
                None,
            )
            .expect("Cannot create index buffer");

        index_buffer
            .set_name("IndexBuffer")
            .expect("Cannot set name on index buffer");

        let index_staging_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(INDEX_DATA_SIZE.0.into())
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create index staging buffer");

        index_staging_buffer
            .set_name("IndexStagingBuffer")
            .expect("Cannot set name on idnex staging buffer");

        let index_subresource_data = SubresourceData::default()
            .set_data(
                &self.mesh_data[INDEX_DATA_OFFSET.into()
                    ..(INDEX_DATA_OFFSET + INDEX_DATA_SIZE).into()],
            )
            .set_row_pitch(ByteCount(INDEX_DATA_SIZE.0))
            .set_slice_pitch(ByteCount(INDEX_DATA_SIZE.0));

        self.command_list
            .update_subresources_heap_alloc(
                &index_buffer,
                &index_staging_buffer,
                ByteCount(0),
                0,
                1,
                &[index_subresource_data],
            )
            .expect("Cannot update index buffer");

        self.command_list
            .resource_barrier(&[ResourceBarrier::new_transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&index_buffer)
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::IndexBuffer),
            )]);

        self.index_count = (INDEX_DATA_SIZE / size_of::<u32>()).0 as u32;

        self.index_buffer_view = Some(
            IndexBufferView::default()
                .set_buffer_location(index_buffer.get_gpu_virtual_address())
                .set_size_in_bytes(self.index_count * ByteCount(4))
                .set_format(Format::R32_UInt),
        );

        self.index_buffer = Some(index_buffer);
        self.index_staging_buffer = Some(index_staging_buffer);

        self.info_queue
            .print_messages()
            .expect("Cannot get messages from info queue");
    }

    fn setup_textures(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let texture_desc = ResourceDesc::default()
            .set_format(Format::R8G8B8A8_UNorm)
            .set_width(CITY_MATERIAL_TEXTURE_WIDTH as u64)
            .set_height(CITY_MATERIAL_TEXTURE_HEIGHT)
            .set_dimension(ResourceDimension::Texture2D);

        let texture_data = self.generate_texture_data(&texture_desc);
        self.upload_textures(&texture_desc, &texture_data);
        self.load_diffuse_texture();
        self.setup_sampler();
    }

    fn setup_sampler(&mut self) {
        let sampler_desc = SamplerDesc::default()
            .set_filter(Filter::MinMagMipLinear)
            .set_address_u(TextureAddressMode::Wrap)
            .set_address_v(TextureAddressMode::Wrap)
            .set_address_w(TextureAddressMode::Wrap)
            .set_comparison_func(ComparisonFunc::Always)
            .set_min_lod(0.)
            .set_max_lod(std::f32::MAX)
            .set_mip_lod_bias(0.)
            .set_max_anisotropy(1);

        self.device.create_sampler(
            &sampler_desc,
            self.sampler_heap.get_cpu_descriptor_handle_for_heap_start(),
        );

        let mut srv_handle =
            self.cbv_srv_heap.get_cpu_descriptor_handle_for_heap_start();

        let srv_desc = ShaderResourceViewDesc::default()
            .set_shader4_component_mapping(ShaderComponentMapping::default())
            .set_format(TEXTURES[0].format)
            .new_texture_2d(&Tex2DSrv::default().set_mip_levels(1));
        self.device.create_shader_resource_view(
            &self
                .city_diffuse_texture
                .as_ref()
                .expect("No texture has been created"),
            Some(&srv_desc),
            self.cbv_srv_heap.get_cpu_descriptor_handle_for_heap_start(),
        );

        srv_handle = srv_handle.advance(1, self.cbv_srv_descriptor_handle_size);

        for mat_idx in 0..CITY_MATERIAL_COUNT as usize {
            let mat_srv_desc =
                ShaderResourceViewDesc::default()
                    .set_shader4_component_mapping(
                        ShaderComponentMapping::default(),
                    )
                    .set_format(Format::R8G8B8A8_UNorm)
                    .new_texture_2d(&Tex2DSrv::default().set_mip_levels(1));

            self.device.create_shader_resource_view(
                &self.city_material_textures[mat_idx],
                Some(&mat_srv_desc),
                srv_handle,
            );

            srv_handle =
                srv_handle.advance(1, self.cbv_srv_descriptor_handle_size);
        }
    }

    fn load_diffuse_texture(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let texture_desc = ResourceDesc::default()
            .set_dimension(ResourceDimension::Texture2D)
            .set_mip_levels(TEXTURES[0].mip_levels.into())
            .set_format(TEXTURES[0].format)
            .set_width(TEXTURES[0].width.into())
            .set_height(TEXTURES[0].height.into());

        let city_diffuse_texture = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Default),
                HeapFlags::None,
                &texture_desc,
                ResourceStates::CopyDest,
                None,
            )
            .expect("Cannot create city diffuse texture");

        city_diffuse_texture
            .set_name("CityDiffuseTexture")
            .expect("Cannot set texture name");

        let subresource_count = u32::from(
            texture_desc.0.DepthOrArraySize * texture_desc.0.MipLevels,
        );

        let upload_buffer_size = city_diffuse_texture
            .get_required_intermediate_size(0, subresource_count)
            .expect("Cannot request upload buffer size");

        let texture_staging_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(upload_buffer_size.0.into())
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create city diffuse texture staging buffer");

        texture_staging_buffer
            .set_name("CityDiffuseTextureStagingBuffer")
            .expect("Cannot set texture staging buffer name");

        let texture_subresource_data = SubresourceData::default()
            .set_data(
                &self.mesh_data[TEXTURES[0].data[0].offset as usize
                    ..TEXTURES[0].data[0].offset as usize
                        + TEXTURES[0].data[0].size as usize],
            )
            .set_row_pitch(TEXTURES[0].data[0].pitch.into())
            .set_slice_pitch(TEXTURES[0].data[0].size.into());

        self.command_list
            .update_subresources_heap_alloc(
                &city_diffuse_texture,
                &texture_staging_buffer,
                ByteCount(0),
                0,
                subresource_count,
                slice::from_ref(&texture_subresource_data),
            )
            .expect("Cannot upload diffuse texture");

        self.command_list
            .resource_barrier(&[ResourceBarrier::new_transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(&city_diffuse_texture)
                    .set_state_before(ResourceStates::CopyDest)
                    .set_state_after(ResourceStates::PixelShaderResource),
            )]);

        self.city_diffuse_texture = Some(city_diffuse_texture);
        self.texture_staging_buffer = Some(texture_staging_buffer);
    }

    fn upload_textures(
        &mut self,
        texture_desc: &ResourceDesc,
        texture_data: &Vec<Vec<u8>>,
    ) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        let subresource_count =
            (texture_desc.0.DepthOrArraySize * texture_desc.0.MipLevels) as u32;

        let upload_buffer_step = self.city_material_textures[0]
            .get_required_intermediate_size(0, subresource_count)
            .expect("Cannot get upload buffer step");

        let upload_buffer_size =
            ByteCount::from(upload_buffer_step * CITY_MATERIAL_COUNT);

        let materials_staging_buffer = self
            .device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(upload_buffer_size.0)
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create materials staging buffer");
        materials_staging_buffer
            .set_name("Materials staging buffer")
            .expect("Cannot set name on staging buffer");

        for mat_idx in 0..CITY_MATERIAL_COUNT as usize {
            let texture_subresource_data = SubresourceData::default()
                .set_data(&texture_data[mat_idx])
                .set_row_pitch(ByteCount(
                    CITY_MATERIAL_TEXTURE_CHANNEL_COUNT as u64
                        * texture_desc.0.Width,
                ))
                .set_slice_pitch(ByteCount(
                    CITY_MATERIAL_TEXTURE_CHANNEL_COUNT as u64
                        * texture_desc.0.Width
                        * texture_desc.0.Height as u64,
                ));

            self.command_list
                .update_subresources_heap_alloc(
                    &self.city_material_textures[mat_idx],
                    &materials_staging_buffer,
                    ByteCount(mat_idx as u64 * upload_buffer_step.0),
                    0,
                    subresource_count,
                    slice::from_ref(&texture_subresource_data),
                )
                .expect("Cannot update material staging buffer");

            self.command_list.resource_barrier(slice::from_ref(
                &ResourceBarrier::new_transition(
                    &ResourceTransitionBarrier::default()
                        .set_resource(&self.city_material_textures[mat_idx])
                        .set_state_before(ResourceStates::CopyDest)
                        .set_state_after(ResourceStates::PixelShaderResource),
                ),
            ));
        }

        self.materials_staging_buffer = Some(materials_staging_buffer);
    }

    fn generate_texture_data(
        &mut self,
        texture_desc: &ResourceDesc,
    ) -> Vec<Vec<u8>> {
        let material_grad_step = 1. / CITY_MATERIAL_COUNT as f32;

        let mut city_texture_data: Vec<Vec<u8>> =
            vec![vec![]; CITY_MATERIAL_COUNT as usize];

        for mat_idx in 0..CITY_MATERIAL_COUNT as usize {
            let material_texture = self
                .device
                .create_committed_resource(
                    &HeapProperties::default().set_heap_type(HeapType::Default),
                    HeapFlags::None,
                    texture_desc,
                    ResourceStates::CopyDest,
                    None,
                )
                .expect("Cannot create material texture");

            material_texture
                .set_name(&format!("CityMaterialTextures_{}", mat_idx))
                .expect("Cannot set name on material texture");

            self.city_material_textures.push(material_texture);

            let t = mat_idx as f32 * material_grad_step;
            city_texture_data[mat_idx].resize(
                (CITY_MATERIAL_TEXTURE_WIDTH
                    * CITY_MATERIAL_TEXTURE_HEIGHT
                    * CITY_MATERIAL_TEXTURE_CHANNEL_COUNT)
                    as usize,
                0,
            );

            for x in 0..CITY_MATERIAL_TEXTURE_WIDTH {
                for y in 0..CITY_MATERIAL_TEXTURE_HEIGHT {
                    let pixel_index = ((y
                        * CITY_MATERIAL_TEXTURE_CHANNEL_COUNT
                        * CITY_MATERIAL_TEXTURE_WIDTH)
                        + (x * CITY_MATERIAL_TEXTURE_CHANNEL_COUNT))
                        as usize;

                    let t_prime = t
                        + (y as f32 / CITY_MATERIAL_TEXTURE_HEIGHT as f32)
                            * material_grad_step;

                    let rgb: [f32; 3] =
                        colorsys::Rgb::from(&colorsys::Hsl::from([
                            t_prime as f64 * 255.,
                            50.,
                            50.,
                            1.,
                        ]))
                        .into();

                    city_texture_data[mat_idx][pixel_index + 0] = rgb[0] as u8;
                    city_texture_data[mat_idx][pixel_index + 1] = rgb[1] as u8;
                    city_texture_data[mat_idx][pixel_index + 2] = rgb[2] as u8;
                    city_texture_data[mat_idx][pixel_index + 3] = 255;
                }
            }
        }

        city_texture_data
    }

    fn populate_command_list(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        self.frame_resources[self.current_frame_resource_index as usize]
            .command_allocator
            .reset()
            .expect("Cannot reset command allocator");

        self.command_list
            .reset(
                &self.frame_resources
                    [self.current_frame_resource_index as usize]
                    .command_allocator,
                Some(&self.pso),
            )
            .expect("Cannot reset command list");

        let mut heaps = [self.cbv_srv_heap.clone(), self.sampler_heap.clone()];
        self.command_list.set_descriptor_heaps(&mut heaps);

        self.command_list
            .set_graphics_root_signature(&self.root_signature);

        self.command_list.set_viewports(&vec![self.viewport_desc]);
        self.command_list
            .set_scissor_rects(&vec![self.scissor_desc]);

        self.command_list
            .resource_barrier(&[ResourceBarrier::new_transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(
                        &self.render_targets[self.frame_index as usize],
                    )
                    .set_state_before(ResourceStates::Common)
                    .set_state_after(ResourceStates::RenderTarget),
            )]);

        let mut rtv_handle = self
            .rtv_heap
            .get_cpu_descriptor_handle_for_heap_start()
            .advance(
                self.frame_index.into(),
                self.cbv_srv_descriptor_handle_size,
            );

        let dsv_handle =
            self.dsv_heap.get_cpu_descriptor_handle_for_heap_start();

        self.command_list.set_render_targets(
            slice::from_mut(&mut rtv_handle),
            false,
            Some(dsv_handle),
        );

        let clear_color: [f32; 4] = [0.0, 0.2, 0.4, 1.0];
        self.command_list.clear_render_target_view(
            rtv_handle,
            clear_color,
            &[],
        );

        self.command_list.clear_depth_stencil_view(
            dsv_handle,
            ClearFlags::Depth,
            1.,
            0,
            &[],
        );

        if USE_BUNDLES {
            self.command_list.execute_bundle(
                &self.frame_resources
                    [self.current_frame_resource_index as usize]
                    .bundle
                    .as_ref()
                    .expect("No bundle in frame resource"),
            );
        } else {
            &self.frame_resources[self.current_frame_resource_index as usize]
                .populate_command_list(
                    &self.command_list,
                    self.current_frame_resource_index.into(),
                    self.index_count,
                    self.index_buffer_view
                        .as_ref()
                        .expect("No index buffer view"),
                    &self
                        .vertex_buffer_view
                        .as_ref()
                        .expect("No vertex buffer view"),
                    &self.cbv_srv_heap,
                    &self.sampler_heap,
                    &self.root_signature,
                    self.cbv_srv_descriptor_handle_size,
                );
        }

        self.command_list
            .resource_barrier(&[ResourceBarrier::new_transition(
                &ResourceTransitionBarrier::default()
                    .set_resource(
                        &self.render_targets[self.frame_index as usize],
                    )
                    .set_state_before(ResourceStates::RenderTarget)
                    .set_state_after(ResourceStates::Common),
            )]);

        self.command_list
            .close()
            .expect("Cannot close command list");
    }

    fn draw(&mut self) {
        // return;

        let _debug_printer = make_debug_printer!(&self.info_queue);

        // trace!("[draw] frame #{}", self.frame_number);

        // Merged OnUpdate and OnRender from the original sample

        let last_completed_fence_value = self.fence.get_completed_value();

        self.current_frame_resource_index =
            (self.current_frame_resource_index + 1) % FRAMES_IN_FLIGHT;

        let resource_fence_value = self.frame_resources
            [self.current_frame_resource_index as usize]
            .fence_value;

        if resource_fence_value != 0
            && resource_fence_value > last_completed_fence_value
        {
            self.fence
                .set_event_on_completion(
                    resource_fence_value,
                    &self.fence_event,
                )
                .expect("Cannot set fence event");

            self.fence_event.wait(None);
        }

        let view = Mat4 {
            x: Vec4::new(1., 0., 0., -56.),
            y: Vec4::new(0., 1., 0., -15.),
            z: Vec4::new(0., 0., 1., -50.),
            w: Vec4::new(0., 0., 0., 1.),
        };

        let proj = Mat4 {
            x: Vec4::new(1.33, 0., 0., 0.),
            y: Vec4::new(0., 2.36, 0., 0.),
            z: Vec4::new(0., 0., -1., -1.),
            w: Vec4::new(0., 0., -1., 0.),
        };

        self.frame_resources[self.current_frame_resource_index as usize]
            .update_constant_buffers(view, proj);

        self.populate_command_list();

        self.command_queue
            .execute_command_lists(slice::from_mut(&mut self.command_list));

        self.swapchain
            .present(1, PresentFlags::None)
            .expect("Cannot present");
        self.frame_index =
            self.swapchain.get_current_back_buffer_index() as u32;

        self.frame_resources[self.current_frame_resource_index as usize]
            .fence_value = self.fence_value;

        self.command_queue
            .signal(&self.fence, self.fence_value)
            .expect("Cannot signal fence");

        self.fence_value += 1;
        self.frame_number += 1;
    }

    fn flush_gpu(&mut self) {
        let _debug_printer = make_debug_printer!(&self.info_queue);

        self.fence_value += 1;

        self.command_queue
            .signal(&self.fence, self.fence_value)
            .expect("Cannot signal fence");

        if self.fence.get_completed_value() < self.fence_value {
            self.fence
                .set_event_on_completion(self.fence_value, &self.fence_event)
                .expect("Cannot set event on fence");
            self.fence_event.wait(None);
            // self.fence_event.close();
        }
    }
}

impl Drop for DynamicIndexingSample {
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

    let input_layout =
        InputLayoutDesc::default().from_input_elements(&input_layout);
    let pso_desc = GraphicsPipelineStateDesc::default()
        .set_input_layout(&input_layout)
        .set_root_signature(root_signature)
        .set_vs_bytecode(&vs_bytecode)
        .set_ps_bytecode(&ps_bytecode)
        .set_rasterizer_state(
            &RasterizerDesc::default().set_cull_mode(CullMode::None),
        )
        .set_blend_state(&BlendDesc::default())
        .set_depth_stencil_state(&DepthStencilDesc::default())
        .set_primitive_topology_type(PrimitiveTopologyType::Triangle)
        .set_rtv_formats(&[Format::R8G8B8A8_UNorm])
        .set_dsv_format(Format::D32_Float);

    let pso = device
        .create_graphics_pipeline_state(&pso_desc)
        .expect("Cannot create PSO");
    pso
}

fn create_shaders() -> (Vec<u8>, Vec<u8>) {
    let vertex_shader = compile_shader(
        "VertexShader",
        r"#
struct VSInput
{
    float3 position    : POSITION;
    float3 normal    : NORMAL;
    float2 uv        : TEXCOORD0;
    float3 tangent    : TANGENT;
};

struct PSInput
{
    float4 position    : SV_POSITION;
    float2 uv        : TEXCOORD0;
};

cbuffer cb0 : register(b0)
{
    float4x4 g_mWorldViewProj;
};

PSInput VSMain(VSInput input)
{
    PSInput result;

    result.position = mul(float4(input.position, 1.0f), g_mWorldViewProj);
    result.uv = input.uv;

    return result;
}
#",
        "VSMain",
        "vs_6_6",
        &[],
        &[],
    )
    .expect("Cannot compile vertex shader");

    let pixel_shader = compile_shader(
        "PixelShader",
        r"#
struct PSInput
{
    float4 position    : SV_POSITION;
    float2 uv        : TEXCOORD0;
};

struct MaterialConstants
{
    uint matIndex;    // Dynamically set index for looking up from the descriptor heap.
};

ConstantBuffer<MaterialConstants> materialConstants : register(b0, space0);
Texture2D        g_txDiffuse    : register(t0);

float4 PSMain(PSInput input) : SV_TARGET
{
    SamplerState sampler = SamplerDescriptorHeap[0];
    float3 diffuse = g_txDiffuse.Sample(sampler, input.uv).rgb;
    Texture2D mat = ResourceDescriptorHeap[1 + materialConstants.matIndex];
    float3 matValue = mat.Sample(sampler, input.uv).rgb;
    return float4(diffuse * matValue, 1.0f);
}
#",
        "PSMain",
        "ps_6_6",
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
        unimplemented!(
            "To support v1.0 root signature serialization we'd need to bring \
d3dx12.h as a dependency to have DX12SerializeVersionedRootSignature"
        );
    }

    let ranges = vec![
        DescriptorRange::default()
            .set_range_type(DescriptorRangeType::Srv)
            .set_num_descriptors(1)
            .set_flags(DescriptorRangeFlags::DataStatic),
        DescriptorRange::default()
            .set_range_type(DescriptorRangeType::Cbv)
            .set_num_descriptors(1)
            .set_flags(DescriptorRangeFlags::DataStatic),
    ];

    let srv_table = RootDescriptorTable::default()
        .set_descriptor_ranges(slice::from_ref(&ranges[0]));

    let cbv_table = RootDescriptorTable::default()
        .set_descriptor_ranges(slice::from_ref(&ranges[1]));
    let root_parameters = vec![
        RootParameter::default()
            .new_descriptor_table(&srv_table)
            .set_shader_visibility(ShaderVisibility::Pixel),
        RootParameter::default()
            .new_descriptor_table(&cbv_table)
            .set_shader_visibility(ShaderVisibility::Vertex),
        RootParameter::default()
            .set_shader_visibility(ShaderVisibility::Pixel)
            .new_constants(&RootConstants::default().set_num_32_bit_values(1)),
    ];
    let root_signature_desc = VersionedRootSignatureDesc::default()
        .set_desc_1_1(
            &RootSignatureDesc::default()
                .set_parameters(&root_parameters)
                .set_flags(
                    RootSignatureFlags::AllowInputAssemblerInputLayout
                        | RootSignatureFlags::CbvSrvUavHeapDirectlyIndexed
                        | RootSignatureFlags::SamplerHeapDirectlyIndexed,
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
        .expect("Cannot create root signature");
    root_signature
}

fn setup_heaps(
    device: &Device,
    swapchain: &Swapchain,
    rtv_descriptor_handle_size: ByteCount,
) -> (
    Vec<Resource>,
    DescriptorHeap,
    DescriptorHeap,
    DescriptorHeap,
    DescriptorHeap,
) {
    let rtv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_heap_type(DescriptorHeapType::Rtv)
                .set_num_descriptors(FRAMES_IN_FLIGHT.into()),
        )
        .expect("Cannot create RTV heap");
    rtv_heap
        .set_name("RTV heap")
        .expect("Cannot set RTV heap name");

    let dsv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_heap_type(DescriptorHeapType::Dsv)
                .set_num_descriptors(1),
        )
        .expect("Cannot create RTV heap");
    dsv_heap
        .set_name("DSV heap")
        .expect("Cannot set DSV heap name");

    let cbv_srv_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_heap_type(DescriptorHeapType::CbvSrvUav)
                .set_flags(DescriptorHeapFlags::ShaderVisible)
                .set_num_descriptors(u32::from(
                    FRAMES_IN_FLIGHT * CITY_ROW_COUNT * CITY_COLUMN_COUNT
                        + CITY_MATERIAL_COUNT
                        + 1,
                )),
        )
        .expect("Cannot create CBV_SRV heap");
    cbv_srv_heap
        .set_name("CBV_SRV heap")
        .expect("Cannot set CBV_SRV heap name");

    let sampler_heap = device
        .create_descriptor_heap(
            &DescriptorHeapDesc::default()
                .set_heap_type(DescriptorHeapType::Sampler)
                .set_flags(DescriptorHeapFlags::ShaderVisible)
                .set_num_descriptors(1),
        )
        .expect("Cannot create sampler heap");
    sampler_heap
        .set_name("Sampler heap")
        .expect("Cannot set sampler heap name");

    let mut rtv_handle = rtv_heap.get_cpu_descriptor_handle_for_heap_start();

    let mut render_targets = vec![];
    for frame_idx in 0..FRAMES_IN_FLIGHT {
        let render_target = swapchain
            .get_buffer(u32::from(frame_idx))
            .expect("Cannot get buffer from swapchain");

        device.create_render_target_view(&render_target, rtv_handle);
        render_targets.push(render_target);

        rtv_handle = rtv_handle.advance(1, rtv_descriptor_handle_size);
    }

    (
        render_targets,
        rtv_heap,
        dsv_heap,
        cbv_srv_heap,
        sampler_heap,
    )
}

fn create_swapchain(
    factory: Factory,
    command_queue: &CommandQueue,
    hwnd: *mut std::ffi::c_void,
) -> Swapchain {
    let swapchain_desc = SwapchainDesc::default()
        .set_width(WINDOW_WIDTH)
        .set_height(WINDOW_HEIGHT)
        .set_buffer_count(u32::from(FRAMES_IN_FLIGHT));
    let swapchain = factory
        .create_swapchain(&command_queue, hwnd as *mut HWND__, &swapchain_desc)
        .expect("Cannot create swapchain");
    factory
        .make_window_association(hwnd, MakeWindowAssociationFlags::NoAltEnter)
        .expect("Cannot make window association");
    swapchain
}

fn create_device(factory: &Factory) -> Device {
    let device;
    if USE_WARP_ADAPTER {
        let warp_adapter = factory
            .enum_warp_adapter()
            .expect("Cannot enum warp adapter");
        device = Device::new(&warp_adapter)
            .expect("Cannot create device on WARP adapter");
    } else {
        let hw_adapter = factory
            .enum_adapters_by_gpu_preference(GpuPreference::HighPerformance)
            .expect("Cannot enumerate adapters")
            .remove(0);
        device = Device::new(&hw_adapter).expect("Cannot create device");
    }
    device
}

type Mat4 = Matrix4<f32>;
type Vec3 = Vector3<f32>;
type Vec4 = Vector4<f32>;

#[derive(Clone, Copy)]
struct SceneConstantBuffer {
    mvp: Mat4,
    padding: [f32; 48],
}

struct FrameResource {
    command_allocator: CommandAllocator,
    bundle_allocator: CommandAllocator,
    bundle: Option<CommandList>,
    cbv_staging_buffer: Resource,
    constant_buffer_ptr: *mut u8,
    constant_buffers: Vec<SceneConstantBuffer>,
    fence_value: u64,
    model_matrices: Vec<Mat4>,
}

impl FrameResource {
    fn new(device: &Device) -> Self {
        let command_allocator = device
            .create_command_allocator(CommandListType::Direct)
            .expect("Cannot create direct command list allocator");

        let bundle_allocator = device
            .create_command_allocator(CommandListType::Bundle)
            .expect("Cannot create bundle allocator");

        let cbv_staging_buffer = device
            .create_committed_resource(
                &HeapProperties::default().set_heap_type(HeapType::Upload),
                HeapFlags::None,
                &ResourceDesc::default()
                    .set_dimension(ResourceDimension::Buffer)
                    .set_width(
                        (size_of::<SceneConstantBuffer>() as u32
                            * (CITY_ROW_COUNT * CITY_COLUMN_COUNT))
                            as u64,
                    )
                    .set_layout(TextureLayout::RowMajor),
                ResourceStates::GenericRead,
                None,
            )
            .expect("Cannot create cbuffer staging buffer");

        let constant_buffer_ptr = cbv_staging_buffer
            .map(0, None)
            .expect("Cannot map cbv staging buffer");

        let mut model_matrices = vec![];
        for i in 0..CITY_ROW_COUNT {
            let city_offset_z = i as f32 * -CITY_SPACING_INTERVAL;
            for j in 0..CITY_COLUMN_COUNT {
                let city_offset_x = j as f32 * CITY_SPACING_INTERVAL;

                model_matrices.push(
                    Mat4::from_translation(Vec3::new(
                        city_offset_x,
                        0.02 * (i as f32 * CITY_COLUMN_COUNT as f32 + j as f32),
                        city_offset_z,
                    ))
                    .transpose(),
                );
            }
        }

        Self {
            command_allocator,
            bundle_allocator,
            bundle: None,
            cbv_staging_buffer,
            constant_buffer_ptr,
            constant_buffers: unsafe {
                vec![
                    std::mem::zeroed();
                    (CITY_ROW_COUNT * CITY_COLUMN_COUNT) as usize
                ]
            },
            fence_value: 0,
            model_matrices,
        }
    }

    fn init_bundle(
        &mut self,
        device: &Device,
        pso: &PipelineState,
        frame_resource_index: u32,
        num_indices: u32,
        index_buffer_view_desc: &IndexBufferView,
        vertex_buffer_view_desc: &VertexBufferView,
        cbv_srv_descriptor_heap: &DescriptorHeap,
        sampler_descriptor_heap: &DescriptorHeap,
        root_signature: &RootSignature,
        cbv_srv_descriptor_handle_size: ByteCount,
    ) {
        let bundle = device
            .create_command_list(
                CommandListType::Bundle,
                &self.bundle_allocator,
                Some(pso),
            )
            .expect("Cannot create frame resource bundle");

        bundle
            .set_name("FrameResourceBundle")
            .expect("Cannot set name on bundle");

        self.populate_command_list(
            &bundle,
            frame_resource_index,
            num_indices,
            index_buffer_view_desc,
            vertex_buffer_view_desc,
            cbv_srv_descriptor_heap,
            sampler_descriptor_heap,
            root_signature,
            cbv_srv_descriptor_handle_size,
        );

        bundle.close().expect("Cannot close bundle");

        self.bundle = Some(bundle);
    }

    fn populate_command_list(
        &mut self,
        command_list: &CommandList,
        frame_resource_index: u32,
        num_indices: u32,
        index_buffer_view_desc: &IndexBufferView,
        vertex_buffer_view_desc: &VertexBufferView,
        cbv_srv_descriptor_heap: &DescriptorHeap,
        sampler_descriptor_heap: &DescriptorHeap,
        root_signature: &RootSignature,
        cbv_srv_descriptor_handle_size: ByteCount,
    ) {
        let mut heaps = [
            cbv_srv_descriptor_heap.clone(),
            sampler_descriptor_heap.clone(),
        ];
        command_list.set_descriptor_heaps(&mut heaps);
        command_list.set_graphics_root_signature(root_signature);

        command_list.set_primitive_topology(PrimitiveTopology::TriangleList);
        command_list.set_index_buffer(index_buffer_view_desc);
        command_list
            .set_vertex_buffers(0, slice::from_ref(vertex_buffer_view_desc));

        command_list.set_graphics_root_descriptor_table(
            0,
            cbv_srv_descriptor_heap.get_gpu_descriptor_handle_for_heap_start(),
        );

        let frame_resource_descriptor_offset = (CITY_MATERIAL_COUNT + 1)
            + (frame_resource_index as u32
                * CITY_ROW_COUNT
                * CITY_COLUMN_COUNT);

        let mut cbv_srv_handle = cbv_srv_descriptor_heap
            .get_gpu_descriptor_handle_for_heap_start()
            .advance(
                u32::from(frame_resource_descriptor_offset),
                cbv_srv_descriptor_handle_size,
            );

        for i in 0..CITY_ROW_COUNT {
            for j in 0..CITY_COLUMN_COUNT {
                command_list.set_graphics_root_32bit_constant(
                    2,
                    i * CITY_COLUMN_COUNT + j,
                    0,
                );

                command_list
                    .set_graphics_root_descriptor_table(1, cbv_srv_handle);

                cbv_srv_handle =
                    cbv_srv_handle.advance(1, cbv_srv_descriptor_handle_size);

                command_list.draw_indexed_instanced(num_indices, 1, 0, 0, 0);
            }
        }
    }

    fn update_constant_buffers(&mut self, view: Mat4, proj: Mat4) {
        for i in 0..CITY_ROW_COUNT {
            for j in 0..CITY_COLUMN_COUNT {
                let model =
                    self.model_matrices[(i * CITY_COLUMN_COUNT + j) as usize];

                let mvp = model * view * proj;
                self.constant_buffers[(i * CITY_COLUMN_COUNT + j) as usize]
                    .mvp = mvp;
            }
        }

        unsafe {
            copy_nonoverlapping(
                self.constant_buffers.as_ptr(),
                self.constant_buffer_ptr as *mut SceneConstantBuffer,
                self.constant_buffers.len(),
            );
        }
    }
}

fn main() {
    simple_logger::init_with_level(log::Level::Warn).unwrap();

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .build(&event_loop)
        .expect("Cannot create window");
    window.set_inner_size(winit::dpi::LogicalSize::new(
        WINDOW_WIDTH,
        WINDOW_HEIGHT,
    ));
    let mut sample = DynamicIndexingSample::new(window.hwnd());

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
