#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::slice;
use std::{convert::TryFrom, marker::PhantomData, mem::size_of};

use widestring::WideCStr;

use crate::raw_bindings::*;
use crate::utils::*;
use crate::{const_wrappers::*, PipelineState};
use crate::{enum_wrappers::*, RootSignature};

use crate::Resource;

// Only newtypes for data structs etc. live here;
// if a struct is not identical to the raw one,
// it should be placed directly in lib.rs

// ToDo: make internal members private after adding all the type-safe getters?

pub struct GpuVirtualAddress(pub D3D12_GPU_VIRTUAL_ADDRESS);

// ToDo: such fields should not be public?
#[repr(transparent)]
pub struct DxgiSwapchainDesc(pub DXGI_SWAP_CHAIN_DESC1);

impl Default for DxgiSwapchainDesc {
    fn default() -> Self {
        DxgiSwapchainDesc(DXGI_SWAP_CHAIN_DESC1 {
            Width: 0,
            Height: 0,
            Format: DXGI_FORMAT_DXGI_FORMAT_R8G8B8A8_UNORM,
            Stereo: 0,
            SampleDesc: DxgiSampleDesc::default().0,
            BufferUsage: DXGI_USAGE_RENDER_TARGET_OUTPUT,
            BufferCount: 2,
            Scaling: DXGI_SCALING_DXGI_SCALING_STRETCH,
            SwapEffect: DXGI_SWAP_EFFECT_DXGI_SWAP_EFFECT_FLIP_DISCARD,
            AlphaMode: DXGI_ALPHA_MODE_DXGI_ALPHA_MODE_UNSPECIFIED,
            Flags: DXGI_SWAP_CHAIN_FLAG_DXGI_SWAP_CHAIN_FLAG_ALLOW_TEARING
                as u32,
        })
    }
}

impl DxgiSwapchainDesc {
    pub fn set_width(mut self, width: u32) -> Self {
        self.0.Width = width;
        self
    }

    pub fn width(&self) -> u32 {
        self.0.Width
    }

    pub fn set_height(mut self, height: u32) -> Self {
        self.0.Height = height;
        self
    }

    pub fn height(&self) -> u32 {
        self.0.Height
    }

    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn format(&self) -> DxgiFormat {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_stereo(mut self, stereo: bool) -> Self {
        self.0.Stereo = stereo as i32;
        self
    }

    pub fn stereo(&self) -> bool {
        self.0.Stereo != 0
    }

    pub fn set_sample_desc(mut self, sample_desc: &DxgiSampleDesc) -> Self {
        self.0.SampleDesc = sample_desc.0;
        self
    }

    pub fn sample_desc(&self) -> DxgiSampleDesc {
        DxgiSampleDesc(self.0.SampleDesc)
    }

    pub fn set_buffer_usage(mut self, buffer_usage: DxgiUsage) -> Self {
        self.0.BufferUsage = buffer_usage.bits();
        self
    }

    pub fn buffer_usage(&self) -> DxgiUsage {
        unsafe { DxgiUsage::from_bits_unchecked(self.0.BufferUsage) }
    }

    pub fn set_buffer_count(mut self, buffer_count: Elements) -> Self {
        self.0.BufferCount = buffer_count.0 as u32;
        self
    }

    pub fn buffer_count(&self) -> Elements {
        Elements::from(self.0.BufferCount)
    }

    pub fn set_scaling(mut self, scaling: DxgiScaling) -> Self {
        self.0.Scaling = scaling as i32;
        self
    }

    pub fn scaling(&self) -> DxgiScaling {
        unsafe { std::mem::transmute(self.0.Scaling) }
    }

    pub fn set_swap_effect(mut self, swap_effect: DxgiSwapEffect) -> Self {
        self.0.SwapEffect = swap_effect as i32;
        self
    }

    pub fn swap_effect(&self) -> DxgiSwapEffect {
        unsafe { std::mem::transmute(self.0.SwapEffect) }
    }

    pub fn set_alpha_mode(mut self, alpha_mode: DxgiAlphaMode) -> Self {
        self.0.AlphaMode = alpha_mode as i32;
        self
    }

    pub fn alpha_mode(&self) -> DxgiAlphaMode {
        unsafe { std::mem::transmute(self.0.AlphaMode) }
    }

    pub fn set_flags(mut self, flags: DxgiSwapChainFlag) -> Self {
        self.0.Flags = flags.bits() as u32;
        self
    }

    pub fn flags(&self) -> DxgiSwapChainFlag {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

#[repr(transparent)]
pub struct DxgiAdapterDesc(pub DXGI_ADAPTER_DESC1);

impl DxgiAdapterDesc {
    pub fn is_software(&self) -> bool {
        self.0.Flags & DXGI_ADAPTER_FLAG_DXGI_ADAPTER_FLAG_SOFTWARE as u32 != 0
    }
}

impl Default for DxgiAdapterDesc {
    fn default() -> Self {
        DxgiAdapterDesc(DXGI_ADAPTER_DESC1 {
            Description: [0; 128],
            VendorId: 0,
            DeviceId: 0,
            SubSysId: 0,
            Revision: 0,
            DedicatedVideoMemory: 0,
            DedicatedSystemMemory: 0,
            SharedSystemMemory: 0,
            AdapterLuid: LUID {
                LowPart: 0,
                HighPart: 0,
            },
            Flags: 0,
        })
    }
}

impl std::fmt::Display for DxgiAdapterDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            concat!(
                "Description: {}, VendorId: {:x}, DeviceId: {:x}, ",
                "SubSysId: {:x}, Revision: {:x}, DedicatedVideoMemory: {}, ",
                "DedicatedSystemMemory: {}, SharedSystemMemory: {}, ",
                "AdapterLuid.LowPart: {:x}, AdapterLuid.HighPart: {:x}, Flags: {:x}"
            ),
            WideCStr::from_slice_with_nul(&self.0.Description)
                .expect("Adapter desc is not valid utf-16")
                .to_string_lossy(),
            self.0.VendorId,
            self.0.DeviceId,
            self.0.SubSysId,
            self.0.Revision,
            self.0.DedicatedVideoMemory,
            self.0.DedicatedSystemMemory,
            self.0.SharedSystemMemory,
            self.0.AdapterLuid.LowPart,
            self.0.AdapterLuid.HighPart,
            self.0.Flags
        )
    }
}

impl std::fmt::Debug for DxgiAdapterDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Default)]
#[repr(transparent)]
pub struct DxgiSampleDesc(pub DXGI_SAMPLE_DESC);

impl DxgiSampleDesc {
    pub fn set_count(mut self, count: u32) -> Self {
        self.0.Count = count;
        self
    }

    pub fn count(&self) -> u32 {
        self.0.Count
    }

    pub fn set_quality(mut self, quality: u32) -> Self {
        self.0.Quality = quality;
        self
    }

    pub fn quality(&self) -> u32 {
        self.0.Quality
    }
}

#[derive(Default)]
#[repr(transparent)]
pub struct ResourceDesc(pub D3D12_RESOURCE_DESC);

impl ResourceDesc {
    pub fn set_dimension(mut self, dimension: ResourceDimension) -> Self {
        self.0.Dimension = dimension as i32;
        self
    }

    pub fn dimension(&self) -> ResourceDimension {
        unsafe { std::mem::transmute(self.0.Dimension) }
    }

    pub fn set_alignment(mut self, alignment: Bytes) -> Self {
        self.0.Alignment = alignment.0;
        self
    }

    pub fn alignment(&self) -> Bytes {
        Bytes(self.0.Alignment)
    }

    pub fn set_width(mut self, width: Elements) -> Self {
        self.0.Width = width.0;
        self
    }

    pub fn width(&self) -> Elements {
        Elements(self.0.Width)
    }

    pub fn set_height(mut self, height: Elements) -> Self {
        self.0.Height = height.0 as u32;
        self
    }

    pub fn height(&self) -> Elements {
        Elements::from(self.0.Height)
    }

    pub fn set_depth_or_array_size(
        mut self,
        depth_or_array_size: Elements,
    ) -> Self {
        self.0.DepthOrArraySize = depth_or_array_size.0 as u16;
        self
    }

    pub fn depth_or_array_size(&self) -> Elements {
        Elements::from(self.0.DepthOrArraySize)
    }

    pub fn set_mip_levels(mut self, mip_levels: u16) -> Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn mip_levels(&self) -> u16 {
        self.0.MipLevels
    }

    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn format(&self) -> DxgiFormat {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_sample_desc(mut self, sample_desc: DxgiSampleDesc) -> Self {
        self.0.SampleDesc = sample_desc.0;
        self
    }

    pub fn sample_desc(&self) -> DxgiSampleDesc {
        DxgiSampleDesc(self.0.SampleDesc)
    }

    pub fn set_layout(mut self, layout: D3D12_TEXTURE_LAYOUT) -> Self {
        self.0.Layout = layout as i32;
        self
    }

    pub fn layout(&self) -> TextureLayout {
        unsafe { std::mem::transmute(self.0.Layout) }
    }

    pub fn set_flags(mut self, flags: ResourceFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn flags(&self) -> D3D12_RESOURCE_FLAGS {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

#[repr(transparent)]
pub struct Message(pub D3D12_MESSAGE);

impl Default for Message {
    fn default() -> Self {
        Message(D3D12_MESSAGE {
            Category:
                D3D12_MESSAGE_CATEGORY_D3D12_MESSAGE_CATEGORY_MISCELLANEOUS,
            Severity: D3D12_MESSAGE_SEVERITY_D3D12_MESSAGE_SEVERITY_MESSAGE,
            ID: 0,
            pDescription: std::ptr::null(),
            DescriptionByteLength: 0,
        })
    }
}

#[repr(transparent)]
pub struct HeapProperties(pub D3D12_HEAP_PROPERTIES);

impl Default for HeapProperties {
    fn default() -> Self {
        HeapProperties(D3D12_HEAP_PROPERTIES {
            Type: HeapType::Default as i32,
            CPUPageProperty: CPUPageProperty::Unknown as i32,
            MemoryPoolPreference: MemoryPool::Unknown as i32,
            CreationNodeMask: 0,
            VisibleNodeMask: 0,
        })
    }
}

impl HeapProperties {
    pub fn set_heap_type(mut self, heap_type: HeapType) -> Self {
        self.0.Type = heap_type as i32;
        self
    }

    pub fn heap_type(&self) -> HeapType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_cpu_page_property(
        mut self,
        cpu_page_property: CPUPageProperty,
    ) -> Self {
        self.0.CPUPageProperty = cpu_page_property as i32;
        self
    }

    pub fn cpu_page_property(&self) -> CPUPageProperty {
        unsafe { std::mem::transmute(self.0.CPUPageProperty) }
    }

    pub fn set_memory_pool_preference(
        mut self,
        memory_pool_preference: MemoryPool,
    ) -> Self {
        self.0.MemoryPoolPreference = memory_pool_preference as i32;
        self
    }

    pub fn memory_pool_preference(&self) -> MemoryPool {
        unsafe { std::mem::transmute(self.0.MemoryPoolPreference) }
    }

    pub fn set_creation_node_mask(mut self, node_mask: u32) -> Self {
        self.0.CreationNodeMask = node_mask;
        self
    }

    pub fn creation_node_mask(&self) -> u32 {
        self.0.CreationNodeMask
    }

    pub fn set_visibility_node_mask(mut self, node_mask: u32) -> Self {
        self.0.VisibleNodeMask = node_mask;
        self
    }

    pub fn visible_node_mask(&self) -> u32 {
        self.0.VisibleNodeMask
    }
}

#[derive(Default, Copy, Clone)]
#[repr(transparent)]
pub struct Range(pub D3D12_RANGE);

impl Range {
    pub fn set_begin(mut self, begin: Bytes) -> Self {
        self.0.Begin = begin.0;
        self
    }

    pub fn begin(&self) -> Bytes {
        Bytes(self.0.Begin)
    }

    pub fn set_end(mut self, end: Bytes) -> Self {
        self.0.End = end.0;
        self
    }

    pub fn end(&self) -> Bytes {
        Bytes(self.0.End)
    }
}

#[repr(transparent)]
pub struct ResourceBarrier(pub D3D12_RESOURCE_BARRIER);

impl ResourceBarrier {
    pub fn set_barrier_type(
        mut self,
        barrier_type: ResourceBarrierType,
    ) -> Self {
        self.0.Type = barrier_type as i32;
        self
    }

    pub fn barrier_type(&self) -> ResourceBarrierType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_flags(mut self, flags: ResourceBarrierFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn flags(&self) -> ResourceBarrierFlags {
        unsafe { std::mem::transmute(self.0.Flags) }
    }

    pub fn set_transition(
        mut self,
        barrier_desc: &ResourceTransitionBarrier,
    ) -> Self {
        self.0.__bindgen_anon_1.Transition = barrier_desc.0;
        self
    }

    pub fn transition(&self) -> ResourceTransitionBarrier {
        unsafe { ResourceTransitionBarrier(self.0.__bindgen_anon_1.Transition) }
    }

    pub fn set_aliasing(
        mut self,
        barrier_desc: &ResourceAliasingBarrier,
    ) -> Self {
        self.0.__bindgen_anon_1.Aliasing = barrier_desc.0;
        self
    }

    pub fn aliasing(&self) -> ResourceAliasingBarrier {
        unsafe { ResourceAliasingBarrier(self.0.__bindgen_anon_1.Aliasing) }
    }

    pub fn set_uav(mut self, barrier_desc: &ResourceUavBarrier) -> Self {
        self.0.__bindgen_anon_1.UAV = barrier_desc.0;
        self
    }

    pub fn uav(&self) -> ResourceUavBarrier {
        unsafe { ResourceUavBarrier(self.0.__bindgen_anon_1.UAV) }
    }

    // Convenience methods
    pub fn make_transition_barrier(desc: &ResourceTransitionBarrier) -> Self {
        Self(D3D12_RESOURCE_BARRIER {
            Type: ResourceBarrierType::Transition as i32,
            Flags: ResourceBarrierFlags::None.bits(),
            __bindgen_anon_1: D3D12_RESOURCE_BARRIER__bindgen_ty_1 {
                Transition: desc.0,
            },
        })
    }

    pub fn make_aliasing_barrier(desc: &ResourceAliasingBarrier) -> Self {
        Self(D3D12_RESOURCE_BARRIER {
            Type: ResourceBarrierType::Aliasing as i32,
            Flags: ResourceBarrierFlags::None.bits(),
            __bindgen_anon_1: D3D12_RESOURCE_BARRIER__bindgen_ty_1 {
                Aliasing: desc.0,
            },
        })
    }

    pub fn make_uav_barrier(desc: &ResourceUavBarrier) -> Self {
        Self(D3D12_RESOURCE_BARRIER {
            Type: ResourceBarrierType::Uav as i32,
            Flags: ResourceBarrierFlags::None.bits(),
            __bindgen_anon_1: D3D12_RESOURCE_BARRIER__bindgen_ty_1 {
                UAV: desc.0,
            },
        })
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ResourceTransitionBarrier(pub D3D12_RESOURCE_TRANSITION_BARRIER);

impl ResourceTransitionBarrier {
    pub fn set_resource(mut self, resource: &Resource) -> Self {
        self.0.pResource = resource.this;
        self
    }

    pub fn resource(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResource,
        };
        resource.add_ref();
        resource
    }

    // None value means "all subresources"
    pub fn set_subresource(mut self, subresource: Option<Elements>) -> Self {
        match subresource {
            Some(index) => self.0.Subresource = index.0 as u32,
            None => {
                self.0.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES
            }
        }
        self
    }

    pub fn subresource(&self) -> Option<Elements> {
        match self.0.Subresource {
            D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES => None,
            _ => Some(Elements::from(self.0.Subresource)),
        }
    }

    pub fn set_state_before(mut self, state_before: ResourceStates) -> Self {
        self.0.StateBefore = state_before as i32;
        self
    }

    pub fn state_before(&self) -> ResourceStates {
        unsafe { std::mem::transmute(self.0.StateBefore) }
    }

    pub fn set_state_after(mut self, state_after: ResourceStates) -> Self {
        self.0.StateAfter = state_after as i32;
        self
    }

    pub fn state_after(&self) -> ResourceStates {
        unsafe { std::mem::transmute(self.0.StateAfter) }
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ResourceAliasingBarrier(pub D3D12_RESOURCE_ALIASING_BARRIER);

impl ResourceAliasingBarrier {
    pub fn set_resource_before(mut self, resource_before: &Resource) -> Self {
        self.0.pResourceBefore = resource_before.this;
        self
    }

    pub fn resource_before(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResourceBefore,
        };
        resource.add_ref();
        resource
    }

    pub fn set_resource_after(mut self, resource_after: &Resource) -> Self {
        self.0.pResourceAfter = resource_after.this;
        self
    }

    pub fn resource_after(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResourceAfter,
        };
        resource.add_ref();
        resource
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ResourceUavBarrier(pub D3D12_RESOURCE_UAV_BARRIER);

impl ResourceUavBarrier {
    pub fn set_resource(mut self, resource: &Resource) -> Self {
        self.0.pResource = resource.this;
        self
    }

    pub fn resource(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResource,
        };
        resource.add_ref();
        resource
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Viewport(pub D3D12_VIEWPORT);

impl Default for Viewport {
    fn default() -> Self {
        Viewport(D3D12_VIEWPORT {
            TopLeftX: 0.,
            TopLeftY: 0.,
            Width: 0.,
            Height: 0.,
            MinDepth: 0.,
            MaxDepth: 1.,
        })
    }
}

impl Viewport {
    pub fn set_top_left_x(mut self, top_left_x: f32) -> Self {
        self.0.TopLeftX = top_left_x;
        self
    }

    pub fn top_left_x(&self) -> f32 {
        self.0.TopLeftX
    }

    pub fn set_top_left_y(mut self, top_left_y: f32) -> Self {
        self.0.TopLeftY = top_left_y;
        self
    }

    pub fn top_left_y(&self) -> f32 {
        self.0.TopLeftY
    }

    pub fn set_width(mut self, width: f32) -> Self {
        self.0.Width = width;
        self
    }

    pub fn width(&self) -> f32 {
        self.0.Width
    }

    pub fn set_height(mut self, height: f32) -> Self {
        self.0.Height = height;
        self
    }

    pub fn height(&self) -> f32 {
        self.0.Height
    }

    pub fn set_min_depth(mut self, min_depth: f32) -> Self {
        self.0.MinDepth = min_depth;
        self
    }

    pub fn min_depth(&self) -> f32 {
        self.0.MinDepth
    }

    pub fn set_max_depth(mut self, max_depth: f32) -> Self {
        self.0.MaxDepth = max_depth;
        self
    }

    pub fn max_depth(&self) -> f32 {
        self.0.MaxDepth
    }
}

#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct Rect(pub D3D12_RECT);

impl Default for Rect {
    fn default() -> Self {
        Rect(D3D12_RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        })
    }
}

impl Rect {
    pub fn set_left(mut self, left: i32) -> Self {
        self.0.left = left;
        self
    }

    pub fn left(&self) -> i32 {
        self.0.left
    }

    pub fn set_top(mut self, top: i32) -> Self {
        self.0.top = top;
        self
    }

    pub fn top(&self) -> i32 {
        self.0.top
    }

    pub fn set_right(mut self, right: i32) -> Self {
        self.0.right = right;
        self
    }

    pub fn right(&self) -> i32 {
        self.0.right
    }

    pub fn set_bottom(mut self, bottom: i32) -> Self {
        self.0.bottom = bottom;
        self
    }

    pub fn bottom(&self) -> i32 {
        self.0.bottom
    }
}

#[repr(transparent)]
pub struct TextureCopyLocation(pub D3D12_TEXTURE_COPY_LOCATION);

impl TextureCopyLocation {
    pub fn new(resource: &Resource, location: &TextureLocationType) -> Self {
        match location {
            TextureLocationType::PlacedFootprint(footprint) => {
                Self(D3D12_TEXTURE_COPY_LOCATION {
                    pResource: resource.this,
                    Type: TextureCopyType::PlacedFootprint as i32,
                    __bindgen_anon_1:
                        D3D12_TEXTURE_COPY_LOCATION__bindgen_ty_1 {
                            PlacedFootprint: footprint.0,
                        },
                })
            }
            TextureLocationType::SubresourceIndex(index) => {
                Self(D3D12_TEXTURE_COPY_LOCATION {
                    pResource: resource.this,
                    Type: TextureCopyType::SubresourceIndex as i32,
                    __bindgen_anon_1:
                        D3D12_TEXTURE_COPY_LOCATION__bindgen_ty_1 {
                            SubresourceIndex: index.0 as u32,
                        },
                })
            }
        }
    }

    pub fn resource(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResource,
        };
        resource.add_ref();
        resource
    }

    pub fn copy_type(&self) -> TextureCopyType {
        match unsafe { std::mem::transmute(self.0.Type) } {
            TextureCopyType::PlacedFootprint => {
                TextureCopyType::PlacedFootprint
            }
            TextureCopyType::SubresourceIndex => {
                TextureCopyType::SubresourceIndex
            }
        }
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct Box(pub D3D12_BOX);

impl Default for Box {
    fn default() -> Self {
        Self(D3D12_BOX {
            left: 0,
            top: 0,
            front: 0,
            right: 0,
            bottom: 1,
            back: 1,
        })
    }
}

impl Box {
    pub fn set_left(mut self, left: Elements) -> Self {
        self.0.left = left.0 as u32;
        self
    }

    pub fn left(&self) -> Elements {
        Elements::from(self.0.left)
    }

    pub fn set_top(mut self, top: Elements) -> Self {
        self.0.top = top.0 as u32;
        self
    }

    pub fn top(&self) -> Elements {
        Elements::from(self.0.top)
    }

    pub fn set_front(mut self, front: Elements) -> Self {
        self.0.front = front.0 as u32;
        self
    }

    pub fn front(&self) -> Elements {
        Elements::from(self.0.front)
    }

    pub fn set_right(mut self, right: Elements) -> Self {
        self.0.right = right.0 as u32;
        self
    }

    pub fn right(&self) -> Elements {
        Elements::from(self.0.right)
    }

    pub fn set_bottom(mut self, bottom: Elements) -> Self {
        self.0.bottom = bottom.0 as u32;
        self
    }

    pub fn bottom(&self) -> Elements {
        Elements::from(self.0.bottom)
    }

    pub fn set_back(mut self, back: Elements) -> Self {
        self.0.back = back.0 as u32;
        self
    }

    pub fn back(&self) -> Elements {
        Elements::from(self.0.back)
    }
}

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct VertexBufferView(pub D3D12_VERTEX_BUFFER_VIEW);

impl VertexBufferView {
    pub fn set_buffer_location(
        mut self,
        buffer_location: GpuVirtualAddress,
    ) -> Self {
        self.0.BufferLocation = buffer_location.0;
        self
    }

    pub fn buffer_location(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(self.0.BufferLocation)
    }

    pub fn set_size_in_bytes(mut self, size_in_bytes: Bytes) -> Self {
        self.0.SizeInBytes = size_in_bytes.0 as u32;
        self
    }

    pub fn size_in_bytes(&self) -> Bytes {
        Bytes::from(self.0.SizeInBytes)
    }

    pub fn set_stride_in_bytes(mut self, stride_in_bytes: Bytes) -> Self {
        self.0.StrideInBytes = stride_in_bytes.0 as u32;
        self
    }

    pub fn stride_in_bytes(&self) -> Bytes {
        Bytes::from(self.0.StrideInBytes)
    }
}

#[repr(transparent)]
pub struct InputElementDesc<'a>(
    pub D3D12_INPUT_ELEMENT_DESC,
    PhantomData<&'a std::ffi::CStr>,
);

impl<'a> Default for InputElementDesc<'a> {
    fn default() -> Self {
        InputElementDesc(D3D12_INPUT_ELEMENT_DESC {
            SemanticName: std::ptr::null(),
            SemanticIndex: 0,
            Format: DxgiFormat::Unknown as i32,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass:
        D3D12_INPUT_CLASSIFICATION_D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        PhantomData)
    }
}

// ToDo: macro for generating input element desc from vertex struct type?

impl<'a> InputElementDesc<'a> {
    pub fn set_name(mut self, name: std::ffi::CString) -> Self {
        self.0.SemanticName = name.into_raw() as *const i8;
        self
    }

    pub fn name(&self) -> &'a std::ffi::CStr {
        unsafe { std::ffi::CStr::from_ptr(self.0.SemanticName) }
    }

    pub fn set_index(mut self, index: Elements) -> Self {
        self.0.SemanticIndex = index.0 as u32;
        self
    }

    pub fn index(&self) -> Elements {
        Elements::from(self.0.SemanticIndex)
    }

    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn format(&self) -> DxgiFormat {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_input_slot(mut self, slot: Elements) -> Self {
        self.0.InputSlot = slot.0 as u32;
        self
    }

    pub fn input_slot(&self) -> Elements {
        Elements::from(self.0.InputSlot)
    }

    pub fn set_offset(mut self, offset: Bytes) -> Self {
        self.0.AlignedByteOffset = offset.0 as u32;
        self
    }

    pub fn offset(&self) -> Bytes {
        Bytes::from(self.0.AlignedByteOffset)
    }

    pub fn set_input_slot_class(mut self, class: InputClassification) -> Self {
        self.0.InputSlotClass = class as i32;
        self
    }

    pub fn input_slot_class(&self) -> InputClassification {
        unsafe { std::mem::transmute(self.0.InputSlotClass) }
    }

    pub fn set_instance_data_steprate(mut self, step_rate: Elements) -> Self {
        self.0.InstanceDataStepRate = step_rate.0 as u32;
        self
    }

    pub fn instance_data_steprate(&self) -> Elements {
        Elements::from(self.0.InstanceDataStepRate)
    }
}

// We need this because we transfer ownership of the CString "name" into
// the raw C string (const char*) "SemanticName". Since this memory has to be
// valid until the destruction of this struct, we need to regain that memory
// back so it can be destroyed correctly
impl<'a> Drop for InputElementDesc<'a> {
    fn drop(&mut self) {
        unsafe {
            let _regained_name = std::ffi::CString::from_raw(
                self.0.SemanticName as *mut std::os::raw::c_char,
            );
        }
    }
}

#[derive(Copy, Clone, Default)]
#[repr(transparent)]
pub struct IndexBufferView(pub D3D12_INDEX_BUFFER_VIEW);

impl IndexBufferView {
    pub fn new(
        resource: &Resource,
        element_count: Elements,
        element_size: Bytes,
    ) -> Self {
        let format: DxgiFormat = match element_size {
            Bytes(2) => DxgiFormat::R16_UInt,
            Bytes(4) => DxgiFormat::R32_UInt,
            _ => panic!("Wrong format for index buffer"), // ToDo: DONT PANIC
        };
        IndexBufferView(D3D12_INDEX_BUFFER_VIEW {
            BufferLocation: resource.get_gpu_virtual_address().0,
            SizeInBytes: (element_size * element_count).0 as u32,
            Format: format as i32,
        })
    }

    pub fn set_buffer_location(
        mut self,
        buffer_location: GpuVirtualAddress,
    ) -> Self {
        self.0.BufferLocation = buffer_location.0;
        self
    }

    pub fn buffer_location(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(self.0.BufferLocation)
    }

    pub fn set_size_in_bytes(mut self, size_in_bytes: Bytes) -> Self {
        self.0.SizeInBytes = size_in_bytes.0 as u32;
        self
    }

    pub fn size_in_bytes(&self) -> Bytes {
        Bytes::from(self.0.SizeInBytes)
    }

    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn format(&self) -> DxgiFormat {
        unsafe { std::mem::transmute(self.0.Format) }
    }
}

#[repr(transparent)]
pub struct ShaderBytecode<'a>(pub D3D12_SHADER_BYTECODE, PhantomData<&'a [u8]>);

impl<'a> Default for ShaderBytecode<'a> {
    fn default() -> Self {
        ShaderBytecode(
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: std::ptr::null(),
                BytecodeLength: 0,
            },
            PhantomData,
        )
    }
}

impl<'a> ShaderBytecode<'a> {
    pub fn from_bytes(data: &'a [u8]) -> Self {
        Self(
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: data.as_ptr() as *const std::ffi::c_void,
                BytecodeLength: data.len() as u64,
            },
            PhantomData,
        )
    }
}

#[repr(transparent)]
pub struct StreamOutputDesc(pub D3D12_STREAM_OUTPUT_DESC);

impl Default for StreamOutputDesc {
    fn default() -> Self {
        Self(D3D12_STREAM_OUTPUT_DESC {
            pSODeclaration: std::ptr::null(),
            NumEntries: 0,
            pBufferStrides: std::ptr::null(),
            NumStrides: 0,
            RasterizedStream: 0,
        })
    }
}
#[derive(Default)]
#[repr(transparent)]
pub struct RenderTargetBlendDesc(pub D3D12_RENDER_TARGET_BLEND_DESC);

impl RenderTargetBlendDesc {
    pub fn set_blend_enable(mut self, blend_enable: bool) -> Self {
        self.0.BlendEnable = blend_enable as i32;
        self
    }

    pub fn blend_enable(&self) -> bool {
        self.0.BlendEnable != 0
    }

    pub fn set_logic_op_enable(mut self, logic_op_enable: bool) -> Self {
        self.0.LogicOpEnable = logic_op_enable as i32;
        self
    }

    pub fn logic_op_enable(&self) -> bool {
        self.0.LogicOpEnable != 0
    }

    pub fn set_src_blend(mut self, src_blend: Blend) -> Self {
        self.0.SrcBlend = src_blend as i32;
        self
    }

    pub fn src_blend(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.SrcBlend) }
    }

    pub fn set_dest_blend(mut self, dest_blend: Blend) -> Self {
        self.0.DestBlend = dest_blend as i32;
        self
    }

    pub fn dest_blend(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.DestBlend) }
    }

    pub fn set_blend_op(mut self, blend_op: BlendOp) -> Self {
        self.0.BlendOp = blend_op as i32;
        self
    }

    pub fn blend_op(&self) -> BlendOp {
        unsafe { std::mem::transmute(self.0.BlendOp) }
    }

    pub fn set_src_blend_alpha(mut self, src_blend_alpha: Blend) -> Self {
        self.0.SrcBlendAlpha = src_blend_alpha as i32;
        self
    }

    pub fn src_blend_alpha(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.SrcBlendAlpha) }
    }

    pub fn set_dest_blend_alpha(mut self, dest_blend_alpha: Blend) -> Self {
        self.0.DestBlendAlpha = dest_blend_alpha as i32;
        self
    }

    pub fn dest_blend_alpha(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.DestBlendAlpha) }
    }

    pub fn set_blend_op_alpha(mut self, blend_op_alpha: Blend) -> Self {
        self.0.BlendOpAlpha = blend_op_alpha as i32;
        self
    }

    pub fn blend_op_alpha(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.BlendOpAlpha) }
    }

    pub fn set_logic_op(mut self, logic_op: LogicOp) -> Self {
        self.0.LogicOp = logic_op as i32;
        self
    }

    pub fn logic_op(&self) -> LogicOp {
        unsafe { std::mem::transmute(self.0.LogicOp) }
    }

    pub fn set_render_target_write_mask(
        mut self,
        render_target_write_mask: u8,
    ) -> Self {
        self.0.RenderTargetWriteMask = render_target_write_mask;
        self
    }

    pub fn render_target_write_mask(&self) -> u8 {
        self.0.RenderTargetWriteMask
    }
}

#[derive(Default)]
#[repr(transparent)]
pub struct BlendDesc(pub D3D12_BLEND_DESC);

impl BlendDesc {
    pub fn set_alpha_to_coverage_enable(
        mut self,
        alpha_to_coverage_enable: bool,
    ) -> Self {
        self.0.AlphaToCoverageEnable = alpha_to_coverage_enable as i32;
        self
    }

    pub fn alpha_to_coverage_enable(&self) -> bool {
        self.0.AlphaToCoverageEnable != 0
    }

    pub fn set_independent_blend_enable(
        mut self,
        independent_blend_enable: bool,
    ) -> Self {
        self.0.IndependentBlendEnable = independent_blend_enable as i32;
        self
    }

    pub fn independent_blend_enable(&self) -> bool {
        self.0.IndependentBlendEnable != 0
    }

    pub fn set_render_target(
        mut self,
        render_targets: [RenderTargetBlendDesc; 8usize],
    ) -> Self {
        // transmute is okay due to repr::transparent
        self.0.RenderTarget = unsafe { std::mem::transmute(render_targets) };
        self
    }

    pub fn render_targets(&self) -> [RenderTargetBlendDesc; 8usize] {
        // transmute is okay due to repr::transparent
        unsafe { std::mem::transmute(self.0.RenderTarget) }
    }
}
#[derive(Default)]
#[repr(transparent)]
pub struct RasterizerDesc(pub D3D12_RASTERIZER_DESC);

impl RasterizerDesc {
    pub fn set_fill_mode(mut self, fill_mode: FillMode) -> Self {
        self.0.FillMode = fill_mode as i32;
        self
    }

    pub fn fill_mode(&self) -> FillMode {
        unsafe { std::mem::transmute(self.0.FillMode) }
    }

    pub fn set_cull_mode(mut self, cull_mode: CullMode) -> Self {
        self.0.CullMode = cull_mode as i32;
        self
    }

    pub fn cull_mode(&self) -> CullMode {
        unsafe { std::mem::transmute(self.0.CullMode) }
    }

    pub fn set_front_counter_clockwise(
        mut self,
        front_counter_clockwise: bool,
    ) -> Self {
        self.0.FrontCounterClockwise = front_counter_clockwise as i32;
        self
    }

    pub fn front_counter_clockwise(&self) -> bool {
        self.0.FrontCounterClockwise != 0
    }

    pub fn set_depth_bias(mut self, depth_bias: i32) -> Self {
        self.0.DepthBias = depth_bias;
        self
    }

    pub fn depth_bias(&self) -> i32 {
        self.0.DepthBias
    }

    pub fn set_depth_bias_clamp(mut self, depth_bias_clamp: f32) -> Self {
        self.0.DepthBiasClamp = depth_bias_clamp;
        self
    }

    pub fn depth_bias_clamp(&self) -> f32 {
        self.0.DepthBiasClamp
    }

    pub fn set_slope_scaled_depth_bias(
        mut self,
        slope_scaled_depth_bias: f32,
    ) -> Self {
        self.0.SlopeScaledDepthBias = slope_scaled_depth_bias;
        self
    }

    pub fn slope_scaled_depth_bias(&self) -> f32 {
        self.0.SlopeScaledDepthBias
    }

    pub fn set_depth_clip_enable(mut self, depth_clip_enable: bool) -> Self {
        self.0.DepthClipEnable = depth_clip_enable as i32;
        self
    }

    pub fn depth_clip_enable(&self) -> bool {
        self.0.DepthClipEnable != 0
    }

    pub fn set_multisample_enable(mut self, multisample_enable: bool) -> Self {
        self.0.MultisampleEnable = multisample_enable as i32;
        self
    }

    pub fn multisample_enable(&self) -> bool {
        self.0.MultisampleEnable != 0
    }

    pub fn set_antialiased_line_enable(
        mut self,
        antialiased_line_enable: bool,
    ) -> Self {
        self.0.AntialiasedLineEnable = antialiased_line_enable as i32;
        self
    }

    pub fn antialiased_line_enable(&self) -> bool {
        self.0.AntialiasedLineEnable != 0
    }

    pub fn set_forced_sample_count(mut self, forced_sample_count: u32) -> Self {
        self.0.ForcedSampleCount = forced_sample_count;
        self
    }

    pub fn forced_sample_count(&self) -> u32 {
        self.0.ForcedSampleCount
    }

    pub fn set_conservative_raster(
        mut self,
        conservative_raster: ConservativeRasterizationMode,
    ) -> Self {
        self.0.ConservativeRaster = conservative_raster as i32;
        self
    }

    pub fn conservative_raster(&self) -> ConservativeRasterizationMode {
        unsafe { std::mem::transmute(self.0.ConservativeRaster) }
    }
}

#[repr(transparent)]
pub struct DepthStencilOpDesc(pub D3D12_DEPTH_STENCILOP_DESC);

impl Default for DepthStencilOpDesc {
    fn default() -> Self {
        Self(D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: StencilOp::Zero as i32,
            StencilDepthFailOp: StencilOp::Zero as i32,
            StencilPassOp: StencilOp::Zero as i32,
            StencilFunc: ComparisonFunc::Never as i32,
        })
    }
}

impl DepthStencilOpDesc {
    pub fn set_stencil_fail_op(mut self, stencil_fail_op: StencilOp) -> Self {
        self.0.StencilFailOp = stencil_fail_op as i32;
        self
    }

    pub fn stencil_fail_op(&self) -> StencilOp {
        unsafe { std::mem::transmute(self.0.StencilFailOp) }
    }

    pub fn set_stencil_depth_fail_op(
        mut self,
        stencil_depth_fail_op: StencilOp,
    ) -> Self {
        self.0.StencilDepthFailOp = stencil_depth_fail_op as i32;
        self
    }

    pub fn stencil_depth_fail_op(&self) -> StencilOp {
        unsafe { std::mem::transmute(self.0.StencilDepthFailOp) }
    }

    pub fn set_stencil_pass_op(mut self, stencil_pass_op: StencilOp) -> Self {
        self.0.StencilPassOp = stencil_pass_op as i32;
        self
    }

    pub fn stencil_pass_op(&self) -> StencilOp {
        unsafe { std::mem::transmute(self.0.StencilPassOp) }
    }

    pub fn set_stencil_func(mut self, stencil_func: ComparisonFunc) -> Self {
        self.0.StencilFunc = stencil_func as i32;
        self
    }

    pub fn stencil_func(&self) -> ComparisonFunc {
        unsafe { std::mem::transmute(self.0.StencilFunc) }
    }
}

#[repr(transparent)]
pub struct DepthStencilDesc(pub D3D12_DEPTH_STENCIL_DESC);

impl Default for DepthStencilDesc {
    fn default() -> Self {
        Self(D3D12_DEPTH_STENCIL_DESC {
            DepthEnable: 0,
            DepthWriteMask: DepthWriteMask::Zero as i32,
            DepthFunc: ComparisonFunc::Never as i32,
            StencilEnable: 0,
            StencilReadMask: 0,
            StencilWriteMask: 0,
            FrontFace: DepthStencilOpDesc::default().0,
            BackFace: DepthStencilOpDesc::default().0,
        })
    }
}

impl DepthStencilDesc {
    pub fn set_depth_enable(mut self, depth_enable: bool) -> Self {
        self.0.DepthEnable = depth_enable as i32;
        self
    }

    pub fn depth_enable(&self) -> bool {
        self.0.DepthEnable != 0
    }

    pub fn set_depth_write_mask(
        mut self,
        depth_write_mask: DepthWriteMask,
    ) -> Self {
        self.0.DepthWriteMask = depth_write_mask as i32;
        self
    }

    pub fn depth_write_mask(&self) -> DepthWriteMask {
        unsafe { std::mem::transmute(self.0.DepthWriteMask) }
    }

    pub fn set_depth_func(mut self, depth_func: ComparisonFunc) -> Self {
        self.0.DepthFunc = depth_func as i32;
        self
    }

    pub fn depth_func(&self) -> ComparisonFunc {
        unsafe { std::mem::transmute(self.0.DepthFunc) }
    }

    pub fn set_stencil_enable(mut self, stencil_enable: bool) -> Self {
        self.0.StencilEnable = stencil_enable as i32;
        self
    }

    pub fn stencil_enable(&self) -> bool {
        self.0.StencilEnable != 0
    }

    pub fn set_stencil_read_mask(mut self, stencil_read_mask: u8) -> Self {
        self.0.StencilReadMask = stencil_read_mask;
        self
    }

    pub fn stencil_read_mask(&self) -> u8 {
        self.0.StencilReadMask
    }

    pub fn set_stencil_write_mask(mut self, stencil_write_mask: u8) -> Self {
        self.0.StencilWriteMask = stencil_write_mask;
        self
    }

    pub fn stencil_write_mask(&self) -> u8 {
        self.0.StencilWriteMask
    }

    pub fn set_front_face(mut self, front_face: DepthStencilOpDesc) -> Self {
        self.0.FrontFace = front_face.0;
        self
    }

    pub fn front_face(&self) -> DepthStencilOpDesc {
        DepthStencilOpDesc(self.0.FrontFace)
    }

    pub fn set_back_face(mut self, back_face: DepthStencilOpDesc) -> Self {
        self.0.BackFace = back_face.0;
        self
    }

    pub fn back_face(&self) -> DepthStencilOpDesc {
        DepthStencilOpDesc(self.0.BackFace)
    }
}

#[repr(transparent)]
pub struct InputLayoutDesc<'a>(
    pub D3D12_INPUT_LAYOUT_DESC,
    PhantomData<&'a [InputElementDesc<'a>]>,
);

impl Default for InputLayoutDesc<'_> {
    fn default() -> Self {
        Self(
            D3D12_INPUT_LAYOUT_DESC {
                pInputElementDescs: std::ptr::null(),
                NumElements: 0,
            },
            PhantomData,
        )
    }
}

impl<'a> InputLayoutDesc<'a> {
    pub fn from_input_elements(
        mut self,
        layout: &'a [InputElementDesc<'a>],
    ) -> Self {
        self.0.pInputElementDescs =
            layout.as_ptr() as *const D3D12_INPUT_ELEMENT_DESC;
        self.0.NumElements = layout.len() as u32;
        self.1 = PhantomData;
        self
    }
}

#[repr(transparent)]
pub struct CachedPipelineState<'a>(
    pub D3D12_CACHED_PIPELINE_STATE,
    PhantomData<&'a [u8]>,
);

impl<'a> Default for CachedPipelineState<'a> {
    fn default() -> Self {
        Self(
            D3D12_CACHED_PIPELINE_STATE {
                pCachedBlob: std::ptr::null_mut(),
                CachedBlobSizeInBytes: 0,
            },
            PhantomData,
        )
    }
}

impl<'a> CachedPipelineState<'a> {
    pub fn set_cached_blob(mut self, cached_blob: &'a [u8]) -> Self {
        self.0.pCachedBlob = cached_blob.as_ptr() as *const std::ffi::c_void;
        self.1 = PhantomData;
        self
    }

    pub fn cached_blob(&self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(
                self.0.pCachedBlob as *const u8,
                self.0.CachedBlobSizeInBytes as usize,
            )
        }
    }

    pub fn set_cached_blob_size_in_bytes(
        mut self,
        cached_blob_size_in_bytes: Bytes,
    ) -> Self {
        self.0.CachedBlobSizeInBytes = cached_blob_size_in_bytes.0;
        self
    }

    pub fn cached_blob_size_in_bytes(&self) -> Bytes {
        Bytes::from(self.0.CachedBlobSizeInBytes)
    }
}

#[repr(transparent)]
pub struct GraphicsPipelineStateDesc<'rs, 'vs, 'ps>(
    pub D3D12_GRAPHICS_PIPELINE_STATE_DESC,
    PhantomData<&'rs RootSignature>,
    PhantomData<&'vs ShaderBytecode<'vs>>,
    PhantomData<&'ps ShaderBytecode<'ps>>,
);

impl<'rs, 'vs, 'ps> Default for GraphicsPipelineStateDesc<'rs, 'vs, 'ps> {
    fn default() -> Self {
        Self(
            D3D12_GRAPHICS_PIPELINE_STATE_DESC {
                pRootSignature: std::ptr::null_mut(),
                VS: ShaderBytecode::default().0,
                PS: ShaderBytecode::default().0,
                DS: ShaderBytecode::default().0,
                HS: ShaderBytecode::default().0,
                GS: ShaderBytecode::default().0,
                StreamOutput: StreamOutputDesc::default().0,
                BlendState: BlendDesc::default().0,
                SampleMask: std::u32::MAX,
                RasterizerState: RasterizerDesc::default().0,
                DepthStencilState: DepthStencilDesc::default().0,
                InputLayout: InputLayoutDesc::default().0,
                IBStripCutValue: IndexBufferStripCut::Disabled as i32,
                PrimitiveTopologyType: PrimitiveTopologyType::Undefined as i32,
                NumRenderTargets: 0,
                RTVFormats: [DxgiFormat::Unknown as i32; 8usize],
                DSVFormat: DxgiFormat::Unknown as i32,
                SampleDesc: DxgiSampleDesc::default().0,
                NodeMask: 0,
                CachedPSO: CachedPipelineState::default().0,
                Flags: PipelineStateFlags::None.bits(),
            },
            PhantomData,
            PhantomData,
            PhantomData,
        )
    }
}

impl<'rs, 'vs, 'ps> GraphicsPipelineStateDesc<'rs, 'vs, 'ps> {
    pub fn set_root_signature(
        mut self,
        root_signature: &'rs RootSignature,
    ) -> Self {
        self.0.pRootSignature = root_signature.this;
        self.1 = PhantomData;
        self
    }

    pub fn set_vertex_shader_bytecode(
        mut self,
        bytecode: &'vs ShaderBytecode,
    ) -> Self {
        self.0.VS = bytecode.0;
        self.1 = PhantomData;
        self
    }

    pub fn set_pixel_shader_bytecode(
        mut self,
        bytecode: &'ps ShaderBytecode,
    ) -> Self {
        self.0.PS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn set_blend_state(mut self, blend_state: &BlendDesc) -> Self {
        self.0.BlendState = blend_state.0;
        self
    }

    pub fn set_rasterizer_state(
        mut self,
        rasterizer_state: &RasterizerDesc,
    ) -> Self {
        self.0.RasterizerState = rasterizer_state.0;
        self
    }

    pub fn set_depth_stencil_state(
        mut self,
        depth_stencil_state: &DepthStencilDesc,
    ) -> Self {
        self.0.DepthStencilState = depth_stencil_state.0;
        self
    }

    pub fn set_input_layout(mut self, input_layout: &InputLayoutDesc) -> Self {
        self.0.InputLayout = input_layout.0;
        self
    }

    pub fn set_primitive_topology_type(
        mut self,
        primitive_topology_type: PrimitiveTopologyType,
    ) -> Self {
        self.0.PrimitiveTopologyType = primitive_topology_type as i32;
        self
    }

    pub fn set_num_render_targets(
        mut self,
        num_render_targets: Elements,
    ) -> Self {
        self.0.NumRenderTargets = num_render_targets.0 as u32;
        self
    }

    // ToDo: eliminate loop here and in other similar places
    pub fn set_rtv_formats(mut self, rtv_formats: &[DxgiFormat]) -> Self {
        let mut hw_formats = [DxgiFormat::Unknown as i32; 8usize];
        for format_index in 0..rtv_formats.len() {
            hw_formats[format_index] = rtv_formats[format_index] as i32;
        }
        self.0.RTVFormats = hw_formats;
        self
    }

    pub fn set_dsv_format(mut self, dsv_format: DxgiFormat) -> Self {
        self.0.DSVFormat = dsv_format as i32;
        self
    }

    pub fn set_flags(
        mut self,
        pipeline_state_flags: PipelineStateFlags,
    ) -> Self {
        self.0.Flags = pipeline_state_flags.bits();
        self
    }
}

#[repr(transparent)]
pub struct SubresourceFootprint(pub D3D12_SUBRESOURCE_FOOTPRINT);

impl Default for SubresourceFootprint {
    fn default() -> Self {
        Self(D3D12_SUBRESOURCE_FOOTPRINT {
            Format: DxgiFormat::R8G8B8A8_UNorm as i32,
            Width: 0,
            Height: 1,
            Depth: 1,
            RowPitch: 0,
        })
    }
}

impl SubresourceFootprint {
    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn set_width(mut self, width: u32) -> Self {
        self.0.Width = width;
        self
    }

    pub fn set_height(mut self, height: u32) -> Self {
        self.0.Height = height;
        self
    }

    pub fn set_depth(mut self, depth: u32) -> Self {
        self.0.Depth = depth;
        self
    }

    pub fn set_row_pitch(mut self, row_pitch: u32) -> Self {
        self.0.RowPitch = row_pitch;
        self
    }
}

#[derive(Copy, Clone)]
#[repr(transparent)]
pub struct PlacedSubresourceFootprint(pub D3D12_PLACED_SUBRESOURCE_FOOTPRINT);

impl Default for PlacedSubresourceFootprint {
    fn default() -> Self {
        Self(D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
            Offset: 0,
            Footprint: SubresourceFootprint::default().0,
        })
    }
}

impl PlacedSubresourceFootprint {
    pub fn set_offset(mut self, offset: u64) -> Self {
        self.0.Offset = offset;
        self
    }

    pub fn set_footprint(mut self, footprint: SubresourceFootprint) -> Self {
        self.0.Footprint = footprint.0;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ConstantBufferViewDesc(pub D3D12_CONSTANT_BUFFER_VIEW_DESC);

impl ConstantBufferViewDesc {
    pub fn new(resource: &Resource, size: Bytes) -> Self {
        Self(D3D12_CONSTANT_BUFFER_VIEW_DESC {
            BufferLocation: resource.get_gpu_virtual_address().0,
            SizeInBytes: size.0 as u32,
        })
    }

    pub fn set_buffer_location(
        mut self,
        buffer_location: GpuVirtualAddress,
    ) -> Self {
        self.0.BufferLocation = buffer_location.0;
        self
    }

    pub fn set_size_in_bytes(mut self, size_in_bytes: Bytes) -> Self {
        self.0.SizeInBytes = size_in_bytes.0 as u32;
        self
    }
}

// ToDo: rethink the 'pub's in such wrappers
#[repr(transparent)]
pub struct DescriptorHeapDesc(pub D3D12_DESCRIPTOR_HEAP_DESC);

impl Default for DescriptorHeapDesc {
    fn default() -> Self {
        Self(D3D12_DESCRIPTOR_HEAP_DESC {
            Type: DescriptorHeapType::CBV_SRV_UAV as i32,
            NumDescriptors: 0,
            Flags: DescriptorHeapFlags::None.bits(),
            NodeMask: 0,
        })
    }
}

impl DescriptorHeapDesc {
    pub fn set_type(mut self, heap_type: DescriptorHeapType) -> Self {
        self.0.Type = heap_type as i32;
        self
    }

    pub fn set_num_descriptors(mut self, count: Elements) -> Self {
        self.0.NumDescriptors = count.0 as u32;
        self
    }

    pub fn set_flags(mut self, flags: DescriptorHeapFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }
}

#[repr(transparent)]
pub struct CommandQueueDesc(pub D3D12_COMMAND_QUEUE_DESC);

impl Default for CommandQueueDesc {
    fn default() -> Self {
        Self(D3D12_COMMAND_QUEUE_DESC {
            Type: CommandListType::Direct as i32,
            Priority: CommandQueuePriority::Normal as i32,
            Flags: DescriptorHeapFlags::None.bits(),
            NodeMask: 0,
        })
    }
}

impl CommandQueueDesc {
    pub fn set_type(mut self, command_list_type: CommandListType) -> Self {
        self.0.Type = command_list_type as i32;
        self
    }

    pub fn set_priority(mut self, priority: CommandQueuePriority) -> Self {
        self.0.Priority = priority as i32;
        self
    }

    pub fn set_flags(mut self, flags: DescriptorHeapFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }
}

#[repr(transparent)]
pub struct FeatureDataRootSignature(pub D3D12_FEATURE_DATA_ROOT_SIGNATURE);

impl FeatureDataRootSignature {
    pub fn new(version: RootSignatureVersion) -> Self {
        Self(D3D12_FEATURE_DATA_ROOT_SIGNATURE {
            HighestVersion: version as i32,
        })
    }

    pub fn set_highest_version(
        mut self,
        version: RootSignatureVersion,
    ) -> Self {
        self.0.HighestVersion = version as i32;
        self
    }
}

pub struct DescriptorRangeOffset(u32);

impl From<Elements> for DescriptorRangeOffset {
    fn from(count: Elements) -> Self {
        Self(count.0 as u32)
    }
}

impl DescriptorRangeOffset {
    pub fn append() -> Self {
        Self(D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND)
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct DescriptorRange(pub D3D12_DESCRIPTOR_RANGE1);

impl DescriptorRange {
    pub fn set_range_type(mut self, range_type: DescriptorRangeType) -> Self {
        self.0.RangeType = range_type as i32;
        self
    }

    pub fn set_num_descriptors(mut self, num_descriptors: Elements) -> Self {
        self.0.NumDescriptors = num_descriptors.0 as u32;
        self
    }

    pub fn set_base_shader_register(
        mut self,
        base_shader_register: u32,
    ) -> Self {
        self.0.BaseShaderRegister = base_shader_register;
        self
    }

    pub fn set_register_space(mut self, register_space: u32) -> Self {
        self.0.RegisterSpace = register_space;
        self
    }

    pub fn set_flags(mut self, flags: DescriptorRangeFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn set_offset_in_descriptors_from_table_start(
        mut self,
        offset_in_descriptors_from_table_start: DescriptorRangeOffset,
    ) -> Self {
        self.0.OffsetInDescriptorsFromTableStart =
            offset_in_descriptors_from_table_start.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RootParameter(pub D3D12_ROOT_PARAMETER1);

impl RootParameter {
    pub fn set_parameter_type(
        mut self,
        parameter_type: RootParameterType,
    ) -> Self {
        self.0.ParameterType = parameter_type as i32;
        self
    }

    pub fn set_descriptor_table(
        mut self,
        descriptor_table: &RootDescriptorTable,
    ) -> Self {
        self.0.__bindgen_anon_1.DescriptorTable = descriptor_table.0;
        self
    }

    pub fn set_constants(mut self, constants: &RootConstants) -> Self {
        self.0.__bindgen_anon_1.Constants = constants.0;
        self
    }

    pub fn set_descriptor(mut self, descriptor: &RootDescriptor) -> Self {
        self.0.__bindgen_anon_1.Descriptor = descriptor.0;
        self
    }

    pub fn set_shader_visibility(
        mut self,
        shader_visibility: ShaderVisibility,
    ) -> Self {
        self.0.ShaderVisibility = shader_visibility as i32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RootDescriptorTable<'a>(
    pub D3D12_ROOT_DESCRIPTOR_TABLE1,
    PhantomData<&'a DescriptorRange>,
);

impl<'a> RootDescriptorTable<'a> {
    pub fn set_descriptor_ranges(
        mut self,
        ranges: &'a [DescriptorRange],
    ) -> Self {
        self.0.NumDescriptorRanges = ranges.len() as u32;
        self.0.pDescriptorRanges =
            ranges.as_ptr() as *const D3D12_DESCRIPTOR_RANGE1;
        self.1 = PhantomData;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RootConstants(pub D3D12_ROOT_CONSTANTS);

impl RootConstants {
    pub fn set_shader_register(mut self, shader_register: u32) -> Self {
        self.0.ShaderRegister = shader_register;
        self
    }

    pub fn set_register_space(mut self, register_space: u32) -> Self {
        self.0.RegisterSpace = register_space;
        self
    }

    pub fn set_num_32_bit_values(mut self, num32_bit_values: Elements) -> Self {
        self.0.Num32BitValues = num32_bit_values.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RootDescriptor(pub D3D12_ROOT_DESCRIPTOR1);

impl RootDescriptor {
    pub fn set_shader_register(mut self, shader_register: Elements) -> Self {
        self.0.ShaderRegister = shader_register.0 as u32;
        self
    }

    pub fn set_register_space(mut self, register_space: Elements) -> Self {
        self.0.RegisterSpace = register_space.0 as u32;
        self
    }

    pub fn set_flags(mut self, flags: RootDescriptorFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct SamplerDesc(pub D3D12_SAMPLER_DESC);

impl SamplerDesc {
    pub fn set_filter(mut self, filter: Filter) -> Self {
        self.0.Filter = filter as i32;
        self
    }

    pub fn set_address_u(mut self, address_u: TextureAddressMode) -> Self {
        self.0.AddressU = address_u as i32;
        self
    }

    pub fn set_address_v(mut self, address_v: TextureAddressMode) -> Self {
        self.0.AddressV = address_v as i32;
        self
    }

    pub fn set_address_w(mut self, address_w: TextureAddressMode) -> Self {
        self.0.AddressW = address_w as i32;
        self
    }

    pub fn set_mip_lod_bias(mut self, mip_lod_bias: f32) -> Self {
        self.0.MipLODBias = mip_lod_bias;
        self
    }

    pub fn set_max_anisotropy(mut self, max_anisotropy: u32) -> Self {
        self.0.MaxAnisotropy = max_anisotropy;
        self
    }

    pub fn set_comparison_func(
        mut self,
        comparison_func: ComparisonFunc,
    ) -> Self {
        self.0.ComparisonFunc = comparison_func as i32;
        self
    }

    // ToDo: newtype for vec4 etc.?
    pub fn set_border_color(mut self, border_color: [f32; 4usize]) -> Self {
        self.0.BorderColor = border_color;
        self
    }

    pub fn set_min_lod(mut self, min_lod: f32) -> Self {
        self.0.MinLOD = min_lod;
        self
    }

    pub fn set_max_lod(mut self, max_lod: f32) -> Self {
        self.0.MaxLOD = max_lod;
        self
    }
}

#[derive(Debug)]
#[repr(transparent)]
pub struct StaticSamplerDesc(pub D3D12_STATIC_SAMPLER_DESC);

// based on the first constructor of CD3DX12_STATIC_SAMPLER_DESC
impl Default for StaticSamplerDesc {
    fn default() -> Self {
        Self(D3D12_STATIC_SAMPLER_DESC {
            Filter: D3D12_FILTER_D3D12_FILTER_ANISOTROPIC,
            AddressU:
                D3D12_TEXTURE_ADDRESS_MODE_D3D12_TEXTURE_ADDRESS_MODE_WRAP,
            AddressV:
                D3D12_TEXTURE_ADDRESS_MODE_D3D12_TEXTURE_ADDRESS_MODE_WRAP,
            AddressW:
                D3D12_TEXTURE_ADDRESS_MODE_D3D12_TEXTURE_ADDRESS_MODE_WRAP,
            MipLODBias: 0.,
            MaxAnisotropy: 16,
            ComparisonFunc:
                D3D12_COMPARISON_FUNC_D3D12_COMPARISON_FUNC_LESS_EQUAL,
            BorderColor:
                D3D12_STATIC_BORDER_COLOR_D3D12_STATIC_BORDER_COLOR_OPAQUE_WHITE,
            MinLOD: 0.,
            // ToDo: D3D12_FLOAT32_MAX - for some reason bindgen did not include this constant
            MaxLOD: 3.402823466e+38,
            ShaderRegister: 0,
            RegisterSpace: 0,
            ShaderVisibility:
                D3D12_SHADER_VISIBILITY_D3D12_SHADER_VISIBILITY_ALL,
        })
    }
}

impl StaticSamplerDesc {
    pub fn set_filter(mut self, filter: Filter) -> Self {
        self.0.Filter = filter as i32;
        self
    }

    pub fn set_address_u(mut self, address_u: TextureAddressMode) -> Self {
        self.0.AddressU = address_u as i32;
        self
    }

    pub fn set_address_v(mut self, address_v: TextureAddressMode) -> Self {
        self.0.AddressV = address_v as i32;
        self
    }

    pub fn set_address_w(mut self, address_w: TextureAddressMode) -> Self {
        self.0.AddressW = address_w as i32;
        self
    }

    pub fn set_mip_lod_bias(mut self, mip_lod_bias: f32) -> Self {
        self.0.MipLODBias = mip_lod_bias;
        self
    }

    pub fn set_max_anisotropy(mut self, max_anisotropy: u32) -> Self {
        self.0.MaxAnisotropy = max_anisotropy;
        self
    }

    pub fn set_comparison_func(
        mut self,
        comparison_func: ComparisonFunc,
    ) -> Self {
        self.0.ComparisonFunc = comparison_func as i32;
        self
    }

    pub fn set_border_color(mut self, border_color: StaticBorderColor) -> Self {
        self.0.BorderColor = border_color as i32;
        self
    }

    pub fn set_min_lod(mut self, min_lod: f32) -> Self {
        self.0.MinLOD = min_lod;
        self
    }

    pub fn set_max_lod(mut self, max_lod: f32) -> Self {
        self.0.MaxLOD = max_lod;
        self
    }

    pub fn set_shader_register(mut self, shader_register: Elements) -> Self {
        self.0.ShaderRegister = shader_register.0 as u32;
        self
    }

    pub fn set_register_space(mut self, register_space: Elements) -> Self {
        self.0.RegisterSpace = register_space.0 as u32;
        self
    }

    pub fn set_shader_visibility(
        mut self,
        shader_visibility: ShaderVisibility,
    ) -> Self {
        self.0.ShaderVisibility = shader_visibility as i32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct VersionedRootSignatureDesc(pub D3D12_VERSIONED_ROOT_SIGNATURE_DESC);

impl VersionedRootSignatureDesc {
    pub fn set_version(mut self, version: RootSignatureVersion) -> Self {
        self.0.Version = version as i32;
        self
    }

    // RS v1.0 is not supported
    pub fn set_desc_1_0(self, _desc_1_0: &RootSignatureDesc) -> Self {
        unimplemented!();
        // self.0.__bindgen_anon_1.Desc_1_0 = desc_1_0;
        // self
    }

    pub fn set_desc_1_1(mut self, desc_1_1: &RootSignatureDesc) -> Self {
        self.0.__bindgen_anon_1.Desc_1_1 = desc_1_1.0;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RootSignatureDesc<'a, 'b>(
    pub D3D12_ROOT_SIGNATURE_DESC1,
    PhantomData<&'a RootParameter>,
    PhantomData<&'b StaticSamplerDesc>,
);

impl<'a, 'b> RootSignatureDesc<'a, 'b> {
    pub fn set_parameters(mut self, parameters: &'a [RootParameter]) -> Self {
        self.0.NumParameters = parameters.len() as u32;
        self.0.pParameters =
            parameters.as_ptr() as *const D3D12_ROOT_PARAMETER1;
        self.1 = PhantomData;
        self
    }

    pub fn set_static_samplers(
        mut self,
        static_samplers: &'b [StaticSamplerDesc],
    ) -> Self {
        self.0.NumStaticSamplers = static_samplers.len() as u32;
        self.0.pStaticSamplers =
            static_samplers.as_ptr() as *const D3D12_STATIC_SAMPLER_DESC;
        self.2 = PhantomData;
        self
    }

    pub fn set_flags(mut self, flags: RootSignatureFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct SubresourceData<'a>(
    pub D3D12_SUBRESOURCE_DATA,
    PhantomData<&'a [u8]>,
);

impl<'a> SubresourceData<'a> {
    pub fn set_data<T>(mut self, data: &'a [T]) -> Self {
        self.0.pData = data.as_ptr() as *const std::ffi::c_void;
        self.1 = PhantomData;
        self
    }

    pub fn set_row_pitch(mut self, row_pitch: Bytes) -> Self {
        self.0.RowPitch = row_pitch.0 as i64;
        self
    }

    pub fn set_slice_pitch(mut self, slice_pitch: Bytes) -> Self {
        self.0.SlicePitch = slice_pitch.0 as i64;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ShaderResourceViewDesc(pub D3D12_SHADER_RESOURCE_VIEW_DESC);

impl ShaderResourceViewDesc {
    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn set_view_dimension(mut self, view_dimension: SrvDimension) -> Self {
        self.0.ViewDimension = view_dimension as i32;
        self
    }

    pub fn set_shader4_component_mapping(
        mut self,
        shader4_component_mapping: ShaderComponentMapping,
    ) -> Self {
        self.0.Shader4ComponentMapping = shader4_component_mapping.into();
        self
    }

    pub fn set_buffer(mut self, buffer: &BufferSrv) -> Self {
        self.0.__bindgen_anon_1.Buffer = buffer.0;
        self
    }

    pub fn set_texture_1d(mut self, texture_1d: &Tex1DSrv) -> Self {
        self.0.__bindgen_anon_1.Texture1D = texture_1d.0;
        self
    }

    pub fn set_texture_1d_array(
        mut self,
        texture_1d_array: &Tex1DArraySrv,
    ) -> Self {
        self.0.__bindgen_anon_1.Texture1DArray = texture_1d_array.0;
        self
    }

    pub fn set_texture_2d(mut self, texture_2d: &Tex2DSrv) -> Self {
        self.0.__bindgen_anon_1.Texture2D = texture_2d.0;
        self
    }

    pub fn set_texture_2d_array(
        mut self,
        texture_2d_array: &Tex2DArraySrv,
    ) -> Self {
        self.0.__bindgen_anon_1.Texture2DArray = texture_2d_array.0;
        self
    }

    pub fn set_texture_2d_ms(mut self, texture_2d_ms: &Tex2DMsSrv) -> Self {
        self.0.__bindgen_anon_1.Texture2DMS = texture_2d_ms.0;
        self
    }

    pub fn set_texture_2d_ms_array(
        mut self,
        texture_2d_ms_array: &Tex2DMsArraySrv,
    ) -> Self {
        self.0.__bindgen_anon_1.Texture2DMSArray = texture_2d_ms_array.0;
        self
    }

    pub fn set_texture_3d(mut self, texture_3d: &Tex3DSrv) -> Self {
        self.0.__bindgen_anon_1.Texture3D = texture_3d.0;
        self
    }

    pub fn set_texture_cube(mut self, texture_cube: &TexcubeSrv) -> Self {
        self.0.__bindgen_anon_1.TextureCube = texture_cube.0;
        self
    }

    pub fn set_texture_cube_array(
        mut self,
        texture_cube_array: &TexcubeArraySrv,
    ) -> Self {
        self.0.__bindgen_anon_1.TextureCubeArray = texture_cube_array.0;
        self
    }

    pub fn set_raytracing_acceleration_structure(
        mut self,
        raytracing_acceleration_structure: &RaytracingAccelerationStructureSrv,
    ) -> Self {
        self.0.__bindgen_anon_1.RaytracingAccelerationStructure =
            raytracing_acceleration_structure.0;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct BufferSrv(pub D3D12_BUFFER_SRV);

impl BufferSrv {
    pub fn set_first_element(mut self, first_element: Elements) -> Self {
        self.0.FirstElement = first_element.0 as u64;
        self
    }

    pub fn set_num_elements(mut self, num_elements: Elements) -> Self {
        self.0.NumElements = num_elements.0 as u32;
        self
    }

    pub fn set_structure_byte_stride(
        mut self,
        structure_byte_stride: Bytes,
    ) -> Self {
        self.0.StructureByteStride = structure_byte_stride.0 as u32;
        self
    }

    pub fn set_flags(mut self, flags: BufferSrvFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex1DSrv(pub D3D12_TEX1D_SRV);

impl Tex1DSrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex1DArraySrv(pub D3D12_TEX1D_ARRAY_SRV);

impl Tex1DArraySrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_first_array_slice(
        mut self,
        first_array_slice: Elements,
    ) -> Self {
        self.0.FirstArraySlice = first_array_slice.0 as u32;
        self
    }

    pub fn set_array_size(mut self, array_size: Elements) -> Self {
        self.0.ArraySize = array_size.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DSrv(pub D3D12_TEX2D_SRV);

impl Tex2DSrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_plane_slice(mut self, plane_slice: Elements) -> Self {
        self.0.PlaneSlice = plane_slice.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DArraySrv(pub D3D12_TEX2D_ARRAY_SRV);

impl Tex2DArraySrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_first_array_slice(
        mut self,
        first_array_slice: Elements,
    ) -> Self {
        self.0.FirstArraySlice = first_array_slice.0 as u32;
        self
    }

    pub fn set_array_size(mut self, array_size: Elements) -> Self {
        self.0.ArraySize = array_size.0 as u32;
        self
    }

    pub fn set_plane_slice(mut self, plane_slice: Elements) -> Self {
        self.0.PlaneSlice = plane_slice.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DMsSrv(pub D3D12_TEX2DMS_SRV);

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DMsArraySrv(pub D3D12_TEX2DMS_ARRAY_SRV);

impl Tex2DMsArraySrv {
    pub fn set_first_array_slice(
        mut self,
        first_array_slice: Elements,
    ) -> Self {
        self.0.FirstArraySlice = first_array_slice.0 as u32;
        self
    }

    pub fn set_array_size(mut self, array_size: Elements) -> Self {
        self.0.ArraySize = array_size.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex3DSrv(pub D3D12_TEX3D_SRV);

impl Tex3DSrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct TexcubeSrv(pub D3D12_TEXCUBE_SRV);

impl TexcubeSrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct TexcubeArraySrv(pub D3D12_TEXCUBE_ARRAY_SRV);

impl TexcubeArraySrv {
    pub fn set_most_detailed_mip(
        mut self,
        most_detailed_mip: Elements,
    ) -> Self {
        self.0.MostDetailedMip = most_detailed_mip.0 as u32;
        self
    }

    pub fn set_mip_levels(mut self, mip_levels: Elements) -> Self {
        self.0.MipLevels = mip_levels.0 as u32;
        self
    }

    pub fn set_first_2d_array_face(
        mut self,
        first_2d_array_face: Elements,
    ) -> Self {
        self.0.First2DArrayFace = first_2d_array_face.0 as u32;
        self
    }

    pub fn set_num_cubes(mut self, num_cubes: Elements) -> Self {
        self.0.NumCubes = num_cubes.0 as u32;
        self
    }

    pub fn set_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RaytracingAccelerationStructureSrv(
    pub D3D12_RAYTRACING_ACCELERATION_STRUCTURE_SRV,
);

impl RaytracingAccelerationStructureSrv {
    pub fn set_location(mut self, location: GpuVirtualAddress) -> Self {
        self.0.Location = location.0;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ClearValue(pub D3D12_CLEAR_VALUE);

impl ClearValue {
    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn set_color(mut self, color: [f32; 4usize]) -> Self {
        self.0.__bindgen_anon_1.Color = color;
        self
    }

    pub fn set_depth_stencil(
        mut self,
        depth_stencil: &DepthStencilValue,
    ) -> Self {
        self.0.__bindgen_anon_1.DepthStencil = depth_stencil.0;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct DepthStencilValue(pub D3D12_DEPTH_STENCIL_VALUE);

impl DepthStencilValue {
    pub fn set_depth(mut self, depth: f32) -> Self {
        self.0.Depth = depth;
        self
    }

    pub fn set_stencil(mut self, stencil: u8) -> Self {
        self.0.Stencil = stencil;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct DepthStencilViewDesc(pub D3D12_DEPTH_STENCIL_VIEW_DESC);

impl DepthStencilViewDesc {
    pub fn set_format(mut self, format: DxgiFormat) -> Self {
        self.0.Format = format as i32;
        self
    }

    pub fn set_view_dimension(mut self, view_dimension: DsvDimension) -> Self {
        self.0.ViewDimension = view_dimension as i32;
        self
    }

    pub fn set_flags(mut self, flags: DsvFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn set_texture_1d(mut self, texture_1d: Tex1DDsv) -> Self {
        self.0.__bindgen_anon_1.Texture1D = texture_1d.0;
        self
    }

    pub fn set_texture_1d_array(
        mut self,
        texture_1d_array: Tex1DArrayDsv,
    ) -> Self {
        self.0.__bindgen_anon_1.Texture1DArray = texture_1d_array.0;
        self
    }

    pub fn set_texture_2d(mut self, texture_2d: Tex2DDsv) -> Self {
        self.0.__bindgen_anon_1.Texture2D = texture_2d.0;
        self
    }

    pub fn set_texture_2d_array(
        mut self,
        texture_2d_array: Tex2DArrayDsv,
    ) -> Self {
        self.0.__bindgen_anon_1.Texture2DArray = texture_2d_array.0;
        self
    }

    pub fn set_texture_2d_ms(mut self, texture_2d_ms: Tex2DmsDsv) -> Self {
        self.0.__bindgen_anon_1.Texture2DMS = texture_2d_ms.0;
        self
    }

    pub fn set_texture_2d_ms_array(
        mut self,
        texture_2d_ms_array: D3D12_TEX2DMS_ARRAY_DSV,
    ) -> Self {
        self.0.__bindgen_anon_1.Texture2DMSArray = texture_2d_ms_array;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex1DDsv(pub D3D12_TEX1D_DSV);

impl Tex1DDsv {
    pub fn set_mip_slice(mut self, mip_slice: Elements) -> Self {
        self.0.MipSlice = mip_slice.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex1DArrayDsv(pub D3D12_TEX1D_ARRAY_DSV);

impl Tex1DArrayDsv {
    pub fn set_mip_slice(mut self, mip_slice: Elements) -> Self {
        self.0.MipSlice = mip_slice.0 as u32;
        self
    }

    pub fn set_first_array_slice(
        mut self,
        first_array_slice: Elements,
    ) -> Self {
        self.0.FirstArraySlice = first_array_slice.0 as u32;
        self
    }

    pub fn set_array_size(mut self, array_size: Elements) -> Self {
        self.0.ArraySize = array_size.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DDsv(pub D3D12_TEX2D_DSV);

impl Tex2DDsv {
    pub fn set_mip_slice(mut self, mip_slice: Elements) -> Self {
        self.0.MipSlice = mip_slice.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DArrayDsv(pub D3D12_TEX2D_ARRAY_DSV);

impl Tex2DArrayDsv {
    pub fn set_mip_slice(mut self, mip_slice: Elements) -> Self {
        self.0.MipSlice = mip_slice.0 as u32;
        self
    }

    pub fn set_first_array_slice(
        mut self,
        first_array_slice: Elements,
    ) -> Self {
        self.0.FirstArraySlice = first_array_slice.0 as u32;
        self
    }

    pub fn set_array_size(mut self, array_size: Elements) -> Self {
        self.0.ArraySize = array_size.0 as u32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DmsDsv(pub D3D12_TEX2DMS_DSV);

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct Tex2DmsArrayDsv(pub D3D12_TEX2DMS_ARRAY_DSV);

impl Tex2DmsArrayDsv {
    pub fn set_first_array_slice(
        mut self,
        first_array_slice: Elements,
    ) -> Self {
        self.0.FirstArraySlice = first_array_slice.0 as u32;
        self
    }

    pub fn set_array_size(mut self, array_size: Elements) -> Self {
        self.0.ArraySize = array_size.0 as u32;
        self
    }
}

// ToDo: more ::new() constructors for one-field structs?
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct FeatureDataShaderModel(pub D3D12_FEATURE_DATA_SHADER_MODEL);

impl FeatureDataShaderModel {
    pub fn new(highest_shader_model: ShaderModel) -> Self {
        Self(D3D12_FEATURE_DATA_SHADER_MODEL {
            HighestShaderModel: highest_shader_model as i32,
        })
    }

    pub fn set_highest_shader_model(
        mut self,
        highest_shader_model: ShaderModel,
    ) -> Self {
        self.0.HighestShaderModel = highest_shader_model as i32;
        self
    }
}

// ToDo: Default derives in the structs where they don't make sense
// should be cleaned up (in favor of Builder pattern?)
#[derive(Default, Debug)]
#[repr(transparent)]
pub struct PipelineStateStreamDesc<'a>(
    pub D3D12_PIPELINE_STATE_STREAM_DESC,
    PhantomData<&'a [u8]>,
);

impl<'a> PipelineStateStreamDesc<'a> {
    pub fn set_pipeline_state_subobject_stream(
        mut self,
        subobject_stream: &'a [u8],
    ) -> Self {
        self.0.SizeInBytes = subobject_stream.len() as u64;
        self.0.pPipelineStateSubobjectStream =
            subobject_stream.as_ptr() as *mut std::ffi::c_void;
        self.1 = PhantomData;

        self
    }
}

#[derive(Default, Debug)]
#[repr(C, align(8))]
pub struct PipelineStateSubobject<T> {
    ty: PipelineStateSubobjectType,
    subobject: T,
}

impl<T> PipelineStateSubobject<T> {
    pub fn new(ty: PipelineStateSubobjectType, subobject: T) -> Self {
        let mut subobject_wrapper: PipelineStateSubobject<T> =
            unsafe { std::mem::zeroed() };
        subobject_wrapper.ty = ty;
        subobject_wrapper.subobject = subobject;
        subobject_wrapper
    }
}

// Note it's not a struct from core API
// ToDo: a similar adapter for GraphicsPipelineState? In d3dx12.h
// they have one, and also one more for compute PSO's
#[repr(C)]
pub struct MeshShaderPipelineStateDesc<'rs, 'ams, 'ms, 'ps, 'cp> {
    // We don't use wrapper types here since i) these members are private
    // and don't leak into the public API, and ii) if we want to implement
    // Default trait, we need to either wrap our objects like ShaderBytecode
    // into Options or store raw pointers
    // Fun fact: it turns out Option's are FFI-safe, but anyway, see i)
    root_signature: PipelineStateSubobject<*mut ID3D12RootSignature>,
    amplification_shader: PipelineStateSubobject<D3D12_SHADER_BYTECODE>,
    mesh_shader: PipelineStateSubobject<D3D12_SHADER_BYTECODE>,
    pixel_shader: PipelineStateSubobject<D3D12_SHADER_BYTECODE>,
    blend_state: PipelineStateSubobject<D3D12_BLEND_DESC>,
    sample_mask: PipelineStateSubobject<UINT>,
    rasterizer_state: PipelineStateSubobject<D3D12_RASTERIZER_DESC>,
    depth_stencil_state: PipelineStateSubobject<D3D12_DEPTH_STENCIL_DESC>,
    primitive_topology_type:
        PipelineStateSubobject<D3D12_PRIMITIVE_TOPOLOGY_TYPE>,
    rtv_formats: PipelineStateSubobject<D3D12_RT_FORMAT_ARRAY>,
    dsv_format: PipelineStateSubobject<DXGI_FORMAT>,
    sample_desc: PipelineStateSubobject<DXGI_SAMPLE_DESC>,
    node_mask: PipelineStateSubobject<UINT>,
    cached_pso: PipelineStateSubobject<D3D12_CACHED_PIPELINE_STATE>,
    flags: PipelineStateSubobject<i32>,
    // ToDo: probably we need lifetimes on *mut IDXGI... wrappers, too?..
    rs_phantom_data: PhantomData<&'rs RootSignature>,
    ams_phantom_data: PhantomData<ShaderBytecode<'ams>>,
    ms_phantom_data: PhantomData<ShaderBytecode<'ms>>,
    ps_phantom_data: PhantomData<ShaderBytecode<'ps>>,
    cached_pso_phantom_data: PhantomData<CachedPipelineState<'cp>>,
}

impl<'rs, 'ams, 'ms, 'ps, 'cp> Default
    for MeshShaderPipelineStateDesc<'rs, 'ams, 'ms, 'ps, 'cp>
{
    fn default() -> Self {
        let mut pso_desc: MeshShaderPipelineStateDesc =
            unsafe { std::mem::zeroed() };
        pso_desc.root_signature = PipelineStateSubobject::new(
            PipelineStateSubobjectType::RootSignature,
            std::ptr::null_mut(),
        );
        pso_desc.amplification_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::AS,
            D3D12_SHADER_BYTECODE::default(),
        );
        pso_desc.mesh_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::MS,
            D3D12_SHADER_BYTECODE::default(),
        );
        pso_desc.pixel_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::PS,
            D3D12_SHADER_BYTECODE::default(),
        );
        pso_desc.blend_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Blend,
            BlendDesc::default().0,
        );
        pso_desc.sample_mask = PipelineStateSubobject::new(
            PipelineStateSubobjectType::SampleMask,
            u32::MAX,
        );
        pso_desc.rasterizer_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Rasterizer,
            RasterizerDesc::default().0,
        );
        pso_desc.depth_stencil_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::DepthStencil,
            DepthStencilDesc::default().0,
        );
        pso_desc.primitive_topology_type = PipelineStateSubobject::new(
            PipelineStateSubobjectType::PrimitiveTopology,
            PrimitiveTopologyType::Triangle as i32,
        );
        pso_desc.rtv_formats = PipelineStateSubobject::new(
            PipelineStateSubobjectType::RenderTargetFormats,
            RtFormatArray::default().0,
        );
        pso_desc.dsv_format = PipelineStateSubobject::new(
            PipelineStateSubobjectType::DepthStencilFormat,
            DxgiFormat::Unknown as i32,
        );
        pso_desc.sample_desc = PipelineStateSubobject::new(
            PipelineStateSubobjectType::SampleDesc,
            DxgiSampleDesc::default().0,
            // unsafe {
            //     std::mem::transmute([42u8; size_of::<DXGI_SAMPLE_DESC>()])
            // },
        );
        pso_desc.node_mask = PipelineStateSubobject::new(
            PipelineStateSubobjectType::NodeMask,
            0,
        );
        pso_desc.cached_pso = PipelineStateSubobject::new(
            PipelineStateSubobjectType::CachedPso,
            CachedPipelineState::default().0,
        );
        pso_desc.flags = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Flags,
            PipelineStateFlags::None.bits(),
        );
        pso_desc.rs_phantom_data = PhantomData;
        pso_desc.ams_phantom_data = PhantomData;
        pso_desc.ms_phantom_data = PhantomData;
        pso_desc.ps_phantom_data = PhantomData;
        pso_desc.cached_pso_phantom_data = PhantomData;
        pso_desc
    }
}

impl<'rs, 'ams, 'ms, 'ps, 'cp>
    MeshShaderPipelineStateDesc<'rs, 'ams, 'ms, 'ps, 'cp>
{
    pub fn set_root_signature(
        mut self,
        root_signature: &'rs RootSignature,
    ) -> Self {
        self.root_signature = PipelineStateSubobject::new(
            PipelineStateSubobjectType::RootSignature,
            root_signature.this,
        );
        self.rs_phantom_data = PhantomData;
        self
    }

    pub fn set_amplification_shader_bytecode(
        mut self,
        bytecode: &'ams ShaderBytecode,
    ) -> Self {
        self.amplification_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::AS,
            bytecode.0,
        );
        self.ams_phantom_data = PhantomData;
        self
    }

    pub fn set_mesh_shader_bytecode(
        mut self,
        bytecode: &'ms ShaderBytecode,
    ) -> Self {
        self.mesh_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::MS,
            bytecode.0,
        );
        self.ms_phantom_data = PhantomData;
        self
    }

    pub fn set_pixel_shader_bytecode(
        mut self,
        bytecode: &'ps ShaderBytecode,
    ) -> Self {
        self.pixel_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::PS,
            bytecode.0,
            // unsafe {
            //     std::mem::transmute([0x45u8; size_of::<ShaderBytecode>()])
            // },
        );

        self.ps_phantom_data = PhantomData;
        self
    }

    pub fn set_blend_state(mut self, blend_state: &BlendDesc) -> Self {
        self.blend_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Blend,
            blend_state.0,
        );
        self
    }

    pub fn set_rasterizer_state(
        mut self,
        rasterizer_state: &RasterizerDesc,
    ) -> Self {
        self.rasterizer_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Rasterizer,
            rasterizer_state.0,
        );
        self
    }

    pub fn set_depth_stencil_state(
        mut self,
        depth_stencil_state: &DepthStencilDesc,
    ) -> Self {
        self.depth_stencil_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::DepthStencil,
            depth_stencil_state.0,
        );
        self
    }

    pub fn set_primitive_topology_type(
        mut self,
        primitive_topology_type: PrimitiveTopologyType,
    ) -> Self {
        self.primitive_topology_type = PipelineStateSubobject::new(
            PipelineStateSubobjectType::PrimitiveTopology,
            primitive_topology_type as i32,
        );
        self
    }

    pub fn set_rtv_formats(mut self, rtv_formats: &[DxgiFormat]) -> Self {
        let rt_format_struct =
            RtFormatArray::default().set_rt_formats(rtv_formats);
        self.rtv_formats = PipelineStateSubobject::new(
            PipelineStateSubobjectType::RenderTargetFormats,
            rt_format_struct.0,
        );
        self
    }

    pub fn set_dsv_format(mut self, dsv_format: DxgiFormat) -> Self {
        self.dsv_format = PipelineStateSubobject::new(
            PipelineStateSubobjectType::DepthStencilFormat,
            dsv_format as i32,
        );
        self
    }

    pub fn set_flags(
        mut self,
        pipeline_state_flags: PipelineStateFlags,
    ) -> Self {
        self.flags = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Flags,
            pipeline_state_flags.bits(),
        );
        self
    }

    pub fn make_byte_stream(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct RtFormatArray(pub D3D12_RT_FORMAT_ARRAY);

impl RtFormatArray {
    pub fn set_rt_formats(mut self, rt_formats: &[DxgiFormat]) -> Self {
        let mut hw_formats = [DxgiFormat::Unknown as i32; 8usize];
        for format_index in 0..rt_formats.len() {
            hw_formats[format_index] = rt_formats[format_index] as i32;
        }

        self.0.RTFormats = hw_formats;
        self.0.NumRenderTargets = rt_formats.len() as u32;
        self
    }
}

#[repr(transparent)]
pub struct QueryHeapDesc(pub D3D12_QUERY_HEAP_DESC);

impl Default for QueryHeapDesc {
    fn default() -> Self {
        Self(D3D12_QUERY_HEAP_DESC {
            Type: D3D12_QUERY_HEAP_TYPE_D3D12_QUERY_HEAP_TYPE_TIMESTAMP,
            Count: 0,
            NodeMask: 0,
        })
    }
}

impl QueryHeapDesc {
    pub fn set_type(mut self, ty: QueryHeapType) -> Self {
        self.0.Type = ty as i32;
        self
    }

    pub fn set_count(mut self, count: Elements) -> Self {
        self.0.Count = count.0 as u32;
        self
    }

    pub fn set_node_mask(mut self, node_mask: u32) -> Self {
        self.0.NodeMask = node_mask;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct FeatureDataD3DOptions(pub D3D12_FEATURE_DATA_D3D12_OPTIONS);

impl FeatureDataD3DOptions {
    pub fn set_double_precision_float_shader_ops(
        mut self,
        double_precision_float_shader_ops: bool,
    ) -> Self {
        self.0.DoublePrecisionFloatShaderOps =
            double_precision_float_shader_ops as i32;
        self
    }

    pub fn set_output_merger_logic_op(
        mut self,
        output_merger_logic_op: bool,
    ) -> Self {
        self.0.OutputMergerLogicOp = output_merger_logic_op as i32;
        self
    }

    pub fn set_min_precision_support(
        mut self,
        min_precision_support: ShaderMinPrecisionSupport,
    ) -> Self {
        self.0.MinPrecisionSupport = min_precision_support as i32;
        self
    }

    pub fn set_tiled_resources_tier(
        mut self,
        tiled_resources_tier: D3D12_TILED_RESOURCES_TIER,
    ) -> Self {
        self.0.TiledResourcesTier = tiled_resources_tier;
        self
    }

    pub fn set_resource_binding_tier(
        mut self,
        resource_binding_tier: TiledResourcesTier,
    ) -> Self {
        self.0.ResourceBindingTier = resource_binding_tier as i32;
        self
    }

    pub fn set_ps_specified_stencil_ref_supported(
        mut self,
        ps_specified_stencil_ref_supported: bool,
    ) -> Self {
        self.0.PSSpecifiedStencilRefSupported =
            ps_specified_stencil_ref_supported as i32;
        self
    }

    pub fn set_typed_uav_load_additional_formats(
        mut self,
        typed_uav_load_additional_formats: bool,
    ) -> Self {
        self.0.TypedUAVLoadAdditionalFormats =
            typed_uav_load_additional_formats as i32;
        self
    }

    pub fn set_rovs_supported(mut self, rovs_supported: bool) -> Self {
        self.0.ROVsSupported = rovs_supported as i32;
        self
    }

    pub fn set_conservative_rasterization_tier(
        mut self,
        conservative_rasterization_tier: ConservativeRasterizationTier,
    ) -> Self {
        self.0.ConservativeRasterizationTier =
            conservative_rasterization_tier as i32;
        self
    }

    pub fn set_max_gpu_virtual_address_bits_per_resource(
        mut self,
        max_gpu_virtual_address_bits_per_resource: u32,
    ) -> Self {
        self.0.MaxGPUVirtualAddressBitsPerResource =
            max_gpu_virtual_address_bits_per_resource;
        self
    }

    pub fn set_standard_swizzle_64_kb_supported(
        mut self,
        standard_swizzle_64_kb_supported: bool,
    ) -> Self {
        self.0.StandardSwizzle64KBSupported =
            standard_swizzle_64_kb_supported as i32;
        self
    }

    pub fn set_cross_node_sharing_tier(
        mut self,
        cross_node_sharing_tier: CrossNodeSharingTier,
    ) -> Self {
        self.0.CrossNodeSharingTier = cross_node_sharing_tier as i32;
        self
    }

    pub fn set_cross_adapter_row_major_texture_supported(
        mut self,
        cross_adapter_row_major_texture_supported: bool,
    ) -> Self {
        self.0.CrossAdapterRowMajorTextureSupported =
            cross_adapter_row_major_texture_supported as i32;
        self
    }

    pub fn set_vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation(
        mut self,
        vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation: bool,
    ) -> Self {
        self.0.VPAndRTArrayIndexFromAnyShaderFeedingRasterizerSupportedWithoutGSEmulation = vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation as i32;
        self
    }

    pub fn set_resource_heap_tier(
        mut self,
        resource_heap_tier: ResourceHeapTier,
    ) -> Self {
        self.0.ResourceHeapTier = resource_heap_tier as i32;
        self
    }
}

#[derive(Default, Debug)]
#[repr(transparent)]
pub struct ResourceAllocationInfo(pub D3D12_RESOURCE_ALLOCATION_INFO);

impl ResourceAllocationInfo {
    pub fn set_size_in_bytes(mut self, size_in_bytes: Bytes) -> Self {
        self.0.SizeInBytes = size_in_bytes.0;
        self
    }

    pub fn set_alignment(mut self, alignment: Bytes) -> Self {
        self.0.Alignment = alignment.0;
        self
    }
}

#[derive(Default)]
#[repr(transparent)]
pub struct HeapDesc(pub D3D12_HEAP_DESC);

impl HeapDesc {
    pub fn set_size_in_bytes(mut self, size_in_bytes: Bytes) -> Self {
        self.0.SizeInBytes = size_in_bytes.0;
        self
    }

    pub fn set_properties(mut self, properties: &HeapProperties) -> Self {
        self.0.Properties = properties.0;
        self
    }

    pub fn set_alignment(mut self, alignment: Bytes) -> Self {
        self.0.Alignment = alignment.0;
        self
    }

    pub fn set_flags(mut self, flags: HeapFlags) -> Self {
        self.0.Flags = flags.bits();
        self
    }
}
