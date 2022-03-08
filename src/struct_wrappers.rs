#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use std::ffi::{CStr, CString, NulError};
use std::slice;
use std::str::Utf8Error;
use std::{convert::TryFrom, marker::PhantomData, mem::size_of};

use widestring::WideCStr;

use crate::utils::*;
use crate::{const_wrappers::*, PipelineState};
use crate::{enum_wrappers::*, RootSignature};
use crate::{raw_bindings::d3d12::*, DxError};

use crate::Resource;

// Only newtypes for data structs etc. live here;
// if a struct is not identical to the raw one,
// it should be placed directly in lib.rs

// ToDo: make namespaces for DXGI types and D3D12 since currently they're
// mixed up??

/// Wrapper around D3D12_GPU_VIRTUAL_ADDRESS structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug, Clone, Copy)]
pub struct GpuVirtualAddress(pub D3D12_GPU_VIRTUAL_ADDRESS);

/// Wrapper around DXGI_SWAP_CHAIN_DESC1 structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug, Clone)]
pub struct SwapChainDesc(pub(crate) DXGI_SWAP_CHAIN_DESC1);

impl Default for SwapChainDesc {
    fn default() -> Self {
        SwapChainDesc(DXGI_SWAP_CHAIN_DESC1 {
            Width: 0,
            Height: 0,
            Format: DXGI_FORMAT_DXGI_FORMAT_R8G8B8A8_UNORM,
            Stereo: 0,
            SampleDesc: SampleDesc::default().0,
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

impl SwapChainDesc {
    pub fn set_width(&mut self, width: u32) -> &mut Self {
        self.0.Width = width;
        self
    }

    pub fn with_width(mut self, width: u32) -> Self {
        self.set_width(width);
        self
    }

    pub fn width(&self) -> u32 {
        self.0.Width
    }

    pub fn set_height(&mut self, height: u32) -> &mut Self {
        self.0.Height = height;
        self
    }

    pub fn with_height(mut self, height: u32) -> Self {
        self.set_height(height);
        self
    }

    pub fn height(&self) -> u32 {
        self.0.Height
    }

    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_stereo(&mut self, stereo: bool) -> &mut Self {
        self.0.Stereo = stereo as i32;
        self
    }

    pub fn with_stereo(mut self, stereo: bool) -> Self {
        self.set_stereo(stereo);
        self
    }

    pub fn stereo(&self) -> bool {
        self.0.Stereo != 0
    }

    pub fn set_sample_desc(&mut self, sample_desc: SampleDesc) -> &mut Self {
        self.0.SampleDesc = sample_desc.0;
        self
    }

    pub fn with_sample_desc(mut self, sample_desc: SampleDesc) -> Self {
        self.set_sample_desc(sample_desc);
        self
    }

    pub fn sample_desc(&self) -> SampleDesc {
        SampleDesc(self.0.SampleDesc)
    }

    pub fn set_buffer_usage(&mut self, buffer_usage: Usage) -> &mut Self {
        self.0.BufferUsage = buffer_usage.bits();
        self
    }

    pub fn with_buffer_usage(mut self, buffer_usage: Usage) -> Self {
        self.set_buffer_usage(buffer_usage);
        self
    }

    pub fn buffer_usage(&self) -> Usage {
        unsafe { Usage::from_bits_unchecked(self.0.BufferUsage) }
    }

    pub fn set_buffer_count(&mut self, buffer_count: u32) -> &mut Self {
        self.0.BufferCount = buffer_count;
        self
    }

    pub fn with_buffer_count(mut self, buffer_count: u32) -> Self {
        self.set_buffer_count(buffer_count);
        self
    }

    pub fn buffer_count(&self) -> u32 {
        self.0.BufferCount
    }

    pub fn set_scaling(&mut self, scaling: Scaling) -> &mut Self {
        self.0.Scaling = scaling as i32;
        self
    }

    pub fn with_scaling(mut self, scaling: Scaling) -> Self {
        self.set_scaling(scaling);
        self
    }

    pub fn scaling(&self) -> Scaling {
        unsafe { std::mem::transmute(self.0.Scaling) }
    }

    pub fn set_swap_effect(&mut self, swap_effect: SwapEffect) -> &mut Self {
        self.0.SwapEffect = swap_effect as i32;
        self
    }

    pub fn with_swap_effect(mut self, swap_effect: SwapEffect) -> Self {
        self.set_swap_effect(swap_effect);
        self
    }

    pub fn swap_effect(&self) -> SwapEffect {
        unsafe { std::mem::transmute(self.0.SwapEffect) }
    }

    pub fn set_alpha_mode(&mut self, alpha_mode: AlphaMode) -> &mut Self {
        self.0.AlphaMode = alpha_mode as i32;
        self
    }

    pub fn with_alpha_mode(mut self, alpha_mode: AlphaMode) -> Self {
        self.set_alpha_mode(alpha_mode);
        self
    }

    pub fn alpha_mode(&self) -> AlphaMode {
        unsafe { std::mem::transmute(self.0.AlphaMode) }
    }

    pub fn set_flags(&mut self, flags: SwapChainFlags) -> &mut Self {
        self.0.Flags = flags.bits() as u32;
        self
    }

    pub fn with_flags(mut self, flags: SwapChainFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> SwapChainFlags {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

/// Wrapper around DXGI_ADAPTER_DESC1 structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct AdapterDesc(pub(crate) DXGI_ADAPTER_DESC1);

impl AdapterDesc {
    pub fn is_software(&self) -> bool {
        self.0.Flags & DXGI_ADAPTER_FLAG_DXGI_ADAPTER_FLAG_SOFTWARE as u32 != 0
    }

    // ToDo: clean up?
    pub fn description(&self) -> Option<String> {
        WideCStr::from_slice_with_nul(&self.0.Description)
            .map(|wide_cstr| wide_cstr.to_string_lossy())
            .ok()
    }

    pub fn set_vendor_id(&mut self, vendor_id: u32) -> &mut Self {
        self.0.VendorId = vendor_id;
        self
    }

    pub fn with_vendor_id(mut self, vendor_id: u32) -> Self {
        self.set_vendor_id(vendor_id);
        self
    }

    pub fn vendor_id(&self) -> u32 {
        self.0.VendorId
    }

    pub fn set_device_id(&mut self, device_id: u32) -> &mut Self {
        self.0.DeviceId = device_id;
        self
    }

    pub fn with_device_id(mut self, device_id: u32) -> Self {
        self.set_device_id(device_id);
        self
    }

    pub fn device_id(&self) -> u32 {
        self.0.DeviceId
    }

    pub fn set_sub_sys_id(&mut self, sub_sys_id: u32) -> &mut Self {
        self.0.SubSysId = sub_sys_id;
        self
    }

    pub fn with_sub_sys_id(mut self, sub_sys_id: u32) -> Self {
        self.set_sub_sys_id(sub_sys_id);
        self
    }

    pub fn sub_sys_id(&self) -> u32 {
        self.0.SubSysId
    }

    pub fn set_revision(&mut self, revision: u32) -> &mut Self {
        self.0.Revision = revision;
        self
    }

    pub fn with_revision(mut self, revision: u32) -> Self {
        self.set_revision(revision);
        self
    }

    pub fn revision(&self) -> u32 {
        self.0.Revision
    }

    pub fn set_dedicated_video_memory(
        &mut self,
        dedicated_video_memory: u64,
    ) -> &mut Self {
        self.0.DedicatedVideoMemory = dedicated_video_memory;
        self
    }

    pub fn with_dedicated_video_memory(
        mut self,
        dedicated_video_memory: u64,
    ) -> Self {
        self.set_dedicated_video_memory(dedicated_video_memory);
        self
    }

    pub fn dedicated_video_memory(&self) -> u64 {
        self.0.DedicatedVideoMemory
    }

    pub fn set_dedicated_system_memory(
        &mut self,
        dedicated_system_memory: u64,
    ) -> &mut Self {
        self.0.DedicatedSystemMemory = dedicated_system_memory;
        self
    }

    pub fn with_dedicated_system_memory(
        mut self,
        dedicated_system_memory: u64,
    ) -> Self {
        self.set_dedicated_system_memory(dedicated_system_memory);
        self
    }

    pub fn dedicated_system_memory(&self) -> u64 {
        self.0.DedicatedSystemMemory
    }

    pub fn set_shared_system_memory(
        &mut self,
        shared_system_memory: u64,
    ) -> &mut Self {
        self.0.SharedSystemMemory = shared_system_memory;
        self
    }

    pub fn with_shared_system_memory(
        mut self,
        shared_system_memory: u64,
    ) -> Self {
        self.set_shared_system_memory(shared_system_memory);
        self
    }

    pub fn shared_system_memory(&self) -> u64 {
        self.0.SharedSystemMemory
    }

    pub fn set_flags(&mut self, flags: AdapterFlag) -> &mut Self {
        self.0.Flags = flags as u32;
        self
    }

    pub fn with_flags(mut self, flags: AdapterFlag) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> AdapterFlag {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

impl Default for AdapterDesc {
    fn default() -> Self {
        AdapterDesc(DXGI_ADAPTER_DESC1 {
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

impl std::fmt::Display for AdapterDesc {
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

impl std::fmt::Debug for AdapterDesc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

/// Wrapper around DXGI_SAMPLE_DESC structure
#[derive(Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct SampleDesc(pub(crate) DXGI_SAMPLE_DESC);

impl Default for SampleDesc {
    fn default() -> Self {
        Self(DXGI_SAMPLE_DESC {
            Count: 1,
            Quality: 0,
        })
    }
}

impl SampleDesc {
    pub fn set_count(&mut self, count: u32) -> &mut Self {
        self.0.Count = count;
        self
    }

    pub fn with_count(mut self, count: u32) -> Self {
        self.set_count(count);
        self
    }

    pub fn count(&self) -> u32 {
        self.0.Count
    }

    pub fn set_quality(&mut self, quality: u32) -> &mut Self {
        self.0.Quality = quality;
        self
    }

    pub fn with_quality(mut self, quality: u32) -> Self {
        self.set_quality(quality);
        self
    }

    pub fn quality(&self) -> u32 {
        self.0.Quality
    }
}

/// Wrapper around D3D12_RESOURCE_DESC structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Debug)]
pub struct ResourceDesc(pub(crate) D3D12_RESOURCE_DESC);

impl Default for ResourceDesc {
    fn default() -> Self {
        ResourceDesc(D3D12_RESOURCE_DESC {
            Dimension: ResourceDimension::Unknown as i32,
            Alignment: D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64,
            Width: 0,
            Height: 1,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: Format::Unknown as i32,
            SampleDesc: SampleDesc::default().0,
            Layout: TextureLayout::Unknown as i32,
            Flags: ResourceFlags::None.bits(),
        })
    }
}

impl ResourceDesc {
    pub fn set_dimension(&mut self, dimension: ResourceDimension) -> &mut Self {
        self.0.Dimension = dimension as i32;
        self
    }

    pub fn with_dimension(mut self, dimension: ResourceDimension) -> Self {
        self.set_dimension(dimension);
        self
    }

    pub fn dimension(&self) -> ResourceDimension {
        unsafe { std::mem::transmute(self.0.Dimension) }
    }

    pub fn set_alignment(&mut self, alignment: u64) -> &mut Self {
        self.0.Alignment = alignment;
        self
    }

    pub fn with_alignment(mut self, alignment: u64) -> Self {
        self.set_alignment(alignment);
        self
    }

    pub fn alignment(&self) -> u64 {
        self.0.Alignment
    }

    pub fn set_width(&mut self, width: u64) -> &mut Self {
        self.0.Width = width;
        self
    }

    pub fn with_width(mut self, width: u64) -> Self {
        self.set_width(width);
        self
    }

    pub fn width(&self) -> u64 {
        self.0.Width
    }

    pub fn set_height(&mut self, height: u32) -> &mut Self {
        self.0.Height = height;
        self
    }

    pub fn with_height(mut self, height: u32) -> Self {
        self.set_height(height);
        self
    }

    pub fn height(&self) -> u32 {
        self.0.Height
    }

    pub fn set_depth_or_array_size(
        &mut self,
        depth_or_array_size: u16,
    ) -> &mut Self {
        self.0.DepthOrArraySize = depth_or_array_size;
        self
    }

    pub fn with_depth_or_array_size(
        mut self,
        depth_or_array_size: u16,
    ) -> Self {
        self.set_depth_or_array_size(depth_or_array_size);
        self
    }

    pub fn depth_or_array_size(&self) -> u16 {
        self.0.DepthOrArraySize
    }

    pub fn set_mip_levels(&mut self, mip_levels: u16) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u16) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u16 {
        self.0.MipLevels
    }

    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_sample_desc(&mut self, sample_desc: SampleDesc) -> &mut Self {
        self.0.SampleDesc = sample_desc.0;
        self
    }

    pub fn with_sample_desc(mut self, sample_desc: SampleDesc) -> Self {
        self.set_sample_desc(sample_desc);
        self
    }

    pub fn sample_desc(&self) -> SampleDesc {
        SampleDesc(self.0.SampleDesc)
    }

    pub fn set_layout(&mut self, layout: TextureLayout) -> &mut Self {
        self.0.Layout = layout as i32;
        self
    }

    pub fn with_layout(mut self, layout: TextureLayout) -> Self {
        self.set_layout(layout);
        self
    }

    pub fn layout(&self) -> TextureLayout {
        unsafe { std::mem::transmute(self.0.Layout) }
    }

    pub fn set_flags(&mut self, flags: ResourceFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: ResourceFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> ResourceFlags {
        unsafe { ResourceFlags::from_bits_unchecked(self.0.Flags) }
    }
}

/// Wrapper around D3D12_MESSAGE structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug, Clone)]
pub struct Message(pub(crate) D3D12_MESSAGE);

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

/// Wrapper around D3D12_HEAP_PROPERTIES structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct HeapProperties(pub(crate) D3D12_HEAP_PROPERTIES);

impl HeapProperties {
    pub fn set_heap_type(&mut self, heap_type: HeapType) -> &mut Self {
        self.0.Type = heap_type as i32;
        self
    }

    pub fn with_heap_type(mut self, heap_type: HeapType) -> Self {
        self.set_heap_type(heap_type);
        self
    }

    pub fn heap_type(&self) -> HeapType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_cpu_page_property(
        &mut self,
        cpu_page_property: CpuPageProperty,
    ) -> &mut Self {
        self.0.CPUPageProperty = cpu_page_property as i32;
        self
    }

    pub fn with_cpu_page_property(
        mut self,
        cpu_page_property: CpuPageProperty,
    ) -> Self {
        self.set_cpu_page_property(cpu_page_property);
        self
    }

    pub fn cpu_page_property(&self) -> CpuPageProperty {
        unsafe { std::mem::transmute(self.0.CPUPageProperty) }
    }

    pub fn set_memory_pool_preference(
        &mut self,
        memory_pool_preference: MemoryPool,
    ) -> &mut Self {
        self.0.MemoryPoolPreference = memory_pool_preference as i32;
        self
    }

    pub fn with_memory_pool_preference(
        mut self,
        memory_pool_preference: MemoryPool,
    ) -> Self {
        self.set_memory_pool_preference(memory_pool_preference);
        self
    }

    pub fn memory_pool_preference(&self) -> MemoryPool {
        unsafe { std::mem::transmute(self.0.MemoryPoolPreference) }
    }

    pub fn set_creation_node_mask(
        &mut self,
        creation_node_mask: u32,
    ) -> &mut Self {
        self.0.CreationNodeMask = creation_node_mask;
        self
    }

    pub fn with_creation_node_mask(mut self, creation_node_mask: u32) -> Self {
        self.set_creation_node_mask(creation_node_mask);
        self
    }

    pub fn creation_node_mask(&self) -> u32 {
        self.0.CreationNodeMask
    }

    pub fn set_visible_node_mask(
        &mut self,
        visible_node_mask: u32,
    ) -> &mut Self {
        self.0.VisibleNodeMask = visible_node_mask;
        self
    }

    pub fn with_visible_node_mask(mut self, visible_node_mask: u32) -> Self {
        self.set_visible_node_mask(visible_node_mask);
        self
    }

    pub fn visible_node_mask(&self) -> u32 {
        self.0.VisibleNodeMask
    }
}

/// Wrapper around D3D12_RANGE structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct Range(pub(crate) D3D12_RANGE);

impl Range {
    pub fn set_begin(&mut self, begin: ByteCount) -> &mut Self {
        self.0.Begin = begin.0;
        self
    }

    pub fn with_begin(mut self, begin: ByteCount) -> Self {
        self.set_begin(begin);
        self
    }

    pub fn begin(&self) -> ByteCount {
        ByteCount(self.0.Begin)
    }

    pub fn set_end(&mut self, end: ByteCount) -> &mut Self {
        self.0.End = end.0;
        self
    }

    pub fn with_end(mut self, end: ByteCount) -> Self {
        self.set_end(end);
        self
    }

    pub fn end(&self) -> ByteCount {
        ByteCount(self.0.End)
    }
}

// ToDo: impl Hash where it's needed but cannot be derived
/// Wrapper around D3D12_RESOURCE_BARRIER structure. Note this type is not Clone since it contains a raw pointer
#[repr(transparent)]
#[derive(Debug)]
pub struct ResourceBarrier(pub(crate) D3D12_RESOURCE_BARRIER);

impl ResourceBarrier {
    pub fn set_barrier_type(
        &mut self,
        barrier_type: ResourceBarrierType,
    ) -> &mut Self {
        self.0.Type = barrier_type as i32;
        self
    }

    pub fn with_barrier_type(
        mut self,
        barrier_type: ResourceBarrierType,
    ) -> Self {
        self.set_barrier_type(barrier_type);
        self
    }

    pub fn barrier_type(&self) -> ResourceBarrierType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_flags(&mut self, flags: ResourceBarrierFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: ResourceBarrierFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> ResourceBarrierFlags {
        unsafe { ResourceBarrierFlags::from_bits_unchecked(self.0.Flags) }
    }

    // ToDo: rename it??
    pub fn new_transition(desc: &ResourceTransitionBarrier) -> Self {
        Self(D3D12_RESOURCE_BARRIER {
            Type: ResourceBarrierType::Transition as i32,
            Flags: ResourceBarrierFlags::None.bits(),
            __bindgen_anon_1: D3D12_RESOURCE_BARRIER__bindgen_ty_1 {
                Transition: desc.0,
            },
        })
    }

    pub fn transition(&self) -> Option<ResourceTransitionBarrier> {
        unsafe {
            match self.barrier_type() {
                ResourceBarrierType::Transition => {
                    Some(ResourceTransitionBarrier(
                        self.0.__bindgen_anon_1.Transition,
                    ))
                }
                _ => None,
            }
        }
    }

    pub fn new_aliasing(desc: &ResourceAliasingBarrier) -> Self {
        Self(D3D12_RESOURCE_BARRIER {
            Type: ResourceBarrierType::Aliasing as i32,
            Flags: ResourceBarrierFlags::None.bits(),
            __bindgen_anon_1: D3D12_RESOURCE_BARRIER__bindgen_ty_1 {
                Aliasing: desc.0,
            },
        })
    }

    pub fn aliasing(&self) -> Option<ResourceAliasingBarrier> {
        unsafe {
            match self.barrier_type() {
                ResourceBarrierType::Aliasing => Some(ResourceAliasingBarrier(
                    self.0.__bindgen_anon_1.Aliasing,
                )),
                _ => None,
            }
        }
    }

    pub fn new_uav(desc: &ResourceUavBarrier) -> Self {
        Self(D3D12_RESOURCE_BARRIER {
            Type: ResourceBarrierType::Uav as i32,
            Flags: ResourceBarrierFlags::None.bits(),
            __bindgen_anon_1: D3D12_RESOURCE_BARRIER__bindgen_ty_1 {
                UAV: desc.0,
            },
        })
    }

    pub fn uav(&self) -> Option<ResourceUavBarrier> {
        unsafe {
            match self.barrier_type() {
                ResourceBarrierType::Uav => {
                    Some(ResourceUavBarrier(self.0.__bindgen_anon_1.UAV))
                }
                _ => None,
            }
        }
    }
}

/// Wrapper around D3D12_RESOURCE_TRANSITION_BARRIER structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct ResourceTransitionBarrier(
    pub(crate) D3D12_RESOURCE_TRANSITION_BARRIER,
);

impl ResourceTransitionBarrier {
    pub fn set_resource(&mut self, resource: &Resource) -> &mut Self {
        self.0.pResource = resource.this;
        self
    }

    pub fn with_resource(mut self, resource: &Resource) -> Self {
        self.set_resource(resource);
        self
    }

    // ToDo: return reference?
    pub fn resource(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResource,
        };
        resource.add_ref();
        resource
    }

    // None value means "all subresources"
    pub fn set_subresource(&mut self, subresource: Option<u32>) -> &mut Self {
        match subresource {
            Some(index) => self.0.Subresource = index,
            None => {
                self.0.Subresource = D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES
            }
        }
        self
    }

    pub fn with_subresource(mut self, subresource: Option<u32>) -> Self {
        self.set_subresource(subresource);
        self
    }

    pub fn subresource(&self) -> Option<u32> {
        match self.0.Subresource {
            D3D12_RESOURCE_BARRIER_ALL_SUBRESOURCES => None,
            _ => Some(self.0.Subresource),
        }
    }

    pub fn set_state_before(
        &mut self,
        state_before: ResourceStates,
    ) -> &mut Self {
        self.0.StateBefore = state_before.bits();
        self
    }

    pub fn with_state_before(mut self, state_before: ResourceStates) -> Self {
        self.set_state_before(state_before);
        self
    }

    pub fn state_before(&self) -> ResourceStates {
        unsafe { ResourceStates::from_bits_unchecked(self.0.StateBefore) }
    }

    pub fn set_state_after(
        &mut self,
        state_after: ResourceStates,
    ) -> &mut Self {
        self.0.StateAfter = state_after.bits();
        self
    }

    pub fn with_state_after(mut self, state_after: ResourceStates) -> Self {
        self.set_state_after(state_after);
        self
    }

    pub fn state_after(&self) -> ResourceStates {
        unsafe { ResourceStates::from_bits_unchecked(self.0.StateAfter) }
    }
}

/// Wrapper around D3D12_RESOURCE_ALIASING_BARRIER structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct ResourceAliasingBarrier(pub(crate) D3D12_RESOURCE_ALIASING_BARRIER);

impl ResourceAliasingBarrier {
    pub fn set_resource_before(
        &mut self,
        resource_before: &Resource,
    ) -> &mut Self {
        self.0.pResourceBefore = resource_before.this;
        self
    }

    pub fn with_resource_before(mut self, resource_before: &Resource) -> Self {
        self.set_resource_before(resource_before);
        self
    }

    pub fn resource_before(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResourceBefore,
        };
        resource.add_ref();
        resource
    }

    pub fn set_resource_after(
        &mut self,
        resource_after: &Resource,
    ) -> &mut Self {
        self.0.pResourceAfter = resource_after.this;
        self
    }

    pub fn with_resource_after(mut self, resource_after: &Resource) -> Self {
        self.set_resource_after(resource_after);
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

/// Wrapper around D3D12_RESOURCE_UAV_BARRIER structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct ResourceUavBarrier(pub(crate) D3D12_RESOURCE_UAV_BARRIER);

impl ResourceUavBarrier {
    pub fn set_resource(&mut self, resource: &Resource) -> &mut Self {
        self.0.pResource = resource.this;
        self
    }

    pub fn with_resource(mut self, resource: &Resource) -> Self {
        self.set_resource(resource);
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

/// Wrapper around D3D12_VIEWPORT structure
#[derive(Default, Debug, PartialOrd, PartialEq, Clone, Copy)]
#[repr(transparent)]
pub struct Viewport(pub(crate) D3D12_VIEWPORT);

impl Viewport {
    pub fn set_top_left_x(&mut self, top_left_x: f32) -> &mut Self {
        self.0.TopLeftX = top_left_x;
        self
    }

    pub fn with_top_left_x(mut self, top_left_x: f32) -> Self {
        self.set_top_left_x(top_left_x);
        self
    }

    pub fn top_left_x(&self) -> f32 {
        self.0.TopLeftX
    }

    pub fn set_top_left_y(&mut self, top_left_y: f32) -> &mut Self {
        self.0.TopLeftY = top_left_y;
        self
    }

    pub fn with_top_left_y(mut self, top_left_y: f32) -> Self {
        self.set_top_left_y(top_left_y);
        self
    }

    pub fn top_left_y(&self) -> f32 {
        self.0.TopLeftY
    }

    pub fn set_width(&mut self, width: f32) -> &mut Self {
        self.0.Width = width;
        self
    }

    pub fn with_width(mut self, width: f32) -> Self {
        self.set_width(width);
        self
    }

    pub fn width(&self) -> f32 {
        self.0.Width
    }

    pub fn set_height(&mut self, height: f32) -> &mut Self {
        self.0.Height = height;
        self
    }

    pub fn with_height(mut self, height: f32) -> Self {
        self.set_height(height);
        self
    }

    pub fn height(&self) -> f32 {
        self.0.Height
    }

    pub fn set_min_depth(&mut self, min_depth: f32) -> &mut Self {
        self.0.MinDepth = min_depth;
        self
    }

    pub fn with_min_depth(mut self, min_depth: f32) -> Self {
        self.set_min_depth(min_depth);
        self
    }

    pub fn min_depth(&self) -> f32 {
        self.0.MinDepth
    }

    pub fn set_max_depth(&mut self, max_depth: f32) -> &mut Self {
        self.0.MaxDepth = max_depth;
        self
    }

    pub fn with_max_depth(mut self, max_depth: f32) -> Self {
        self.set_max_depth(max_depth);
        self
    }

    pub fn max_depth(&self) -> f32 {
        self.0.MaxDepth
    }
}

/// Wrapper around D3D12_RECT structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Copy, Debug)]
#[repr(transparent)]
pub struct Rect(pub(crate) D3D12_RECT);

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
    pub fn set_left(&mut self, left: i32) -> &mut Self {
        self.0.left = left;
        self
    }

    pub fn with_left(mut self, left: i32) -> Self {
        self.set_left(left);
        self
    }

    pub fn left(&self) -> i32 {
        self.0.left
    }

    pub fn set_top(&mut self, top: i32) -> &mut Self {
        self.0.top = top;
        self
    }

    pub fn with_top(mut self, top: i32) -> Self {
        self.set_top(top);
        self
    }

    pub fn top(&self) -> i32 {
        self.0.top
    }

    pub fn set_right(&mut self, right: i32) -> &mut Self {
        self.0.right = right;
        self
    }

    pub fn with_right(mut self, right: i32) -> Self {
        self.set_right(right);
        self
    }

    pub fn right(&self) -> i32 {
        self.0.right
    }

    pub fn set_bottom(&mut self, bottom: i32) -> &mut Self {
        self.0.bottom = bottom;
        self
    }

    pub fn with_bottom(mut self, bottom: i32) -> Self {
        self.set_bottom(bottom);
        self
    }

    pub fn bottom(&self) -> i32 {
        self.0.bottom
    }
}

/// Wrapper around D3D12_TEXTURE_COPY_LOCATION structure
// ToDo: add lifetime since we're taking `this` from a Resource?
#[repr(transparent)]
#[derive(Debug)]
pub struct TextureCopyLocation(pub(crate) D3D12_TEXTURE_COPY_LOCATION);

impl TextureCopyLocation {
    pub fn new_placed_footprint(
        resource: &Resource,
        footprint: PlacedSubresourceFootprint,
    ) -> Self {
        Self(D3D12_TEXTURE_COPY_LOCATION {
            pResource: resource.this,
            Type: TextureCopyType::PlacedFootprint as i32,
            __bindgen_anon_1: D3D12_TEXTURE_COPY_LOCATION__bindgen_ty_1 {
                PlacedFootprint: footprint.0,
            },
        })
    }

    pub fn new_subresource_index(resource: &Resource, index: u32) -> Self {
        Self(D3D12_TEXTURE_COPY_LOCATION {
            pResource: resource.this,
            Type: TextureCopyType::SubresourceIndex as i32,
            __bindgen_anon_1: D3D12_TEXTURE_COPY_LOCATION__bindgen_ty_1 {
                SubresourceIndex: index,
            },
        })
    }

    pub fn resource(&self) -> Resource {
        let resource = Resource {
            this: self.0.pResource,
        };
        resource.add_ref();
        resource
    }

    pub fn copy_type(&self) -> TextureCopyType {
        unsafe { std::mem::transmute(self.0.Type) }
    }
}

/// Wrapper around D3D12_BOX structure
#[derive(Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct Box(pub(crate) D3D12_BOX);

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
    pub fn set_left(&mut self, left: u32) -> &mut Self {
        self.0.left = left;
        self
    }

    pub fn with_left(mut self, left: u32) -> Self {
        self.set_left(left);
        self
    }

    pub fn left(&self) -> u32 {
        self.0.left
    }

    pub fn set_top(&mut self, top: u32) -> &mut Self {
        self.0.top = top;
        self
    }

    pub fn with_top(mut self, top: u32) -> Self {
        self.set_top(top);
        self
    }

    pub fn top(&self) -> u32 {
        self.0.top
    }

    pub fn set_front(&mut self, front: u32) -> &mut Self {
        self.0.front = front;
        self
    }

    pub fn with_front(mut self, front: u32) -> Self {
        self.set_front(front);
        self
    }

    pub fn front(&self) -> u32 {
        self.0.front
    }

    pub fn set_right(&mut self, right: u32) -> &mut Self {
        self.0.right = right;
        self
    }

    pub fn with_right(mut self, right: u32) -> Self {
        self.set_right(right);
        self
    }

    pub fn right(&self) -> u32 {
        self.0.right
    }

    pub fn set_bottom(&mut self, bottom: u32) -> &mut Self {
        self.0.bottom = bottom;
        self
    }

    pub fn with_bottom(mut self, bottom: u32) -> Self {
        self.set_bottom(bottom);
        self
    }

    pub fn bottom(&self) -> u32 {
        self.0.bottom
    }

    pub fn set_back(&mut self, back: u32) -> &mut Self {
        self.0.back = back;
        self
    }

    pub fn with_back(mut self, back: u32) -> Self {
        self.set_back(back);
        self
    }

    pub fn back(&self) -> u32 {
        self.0.back
    }
}

/// Wrapper around D3D12_VERTEX_BUFFER_VIEW structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct VertexBufferView(pub(crate) D3D12_VERTEX_BUFFER_VIEW);

impl VertexBufferView {
    pub fn set_buffer_location(
        &mut self,
        buffer_location: GpuVirtualAddress,
    ) -> &mut Self {
        self.0.BufferLocation = buffer_location.0;
        self
    }

    pub fn with_buffer_location(
        mut self,
        buffer_location: GpuVirtualAddress,
    ) -> Self {
        self.set_buffer_location(buffer_location);
        self
    }

    pub fn buffer_location(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(self.0.BufferLocation)
    }

    pub fn set_size_in_bytes(&mut self, size_in_bytes: ByteCount) -> &mut Self {
        self.0.SizeInBytes = size_in_bytes.0 as u32;
        self
    }

    pub fn with_size_in_bytes(mut self, size_in_bytes: ByteCount) -> Self {
        self.set_size_in_bytes(size_in_bytes);
        self
    }

    pub fn size_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.SizeInBytes)
    }

    pub fn set_stride_in_bytes(
        &mut self,
        stride_in_bytes: ByteCount,
    ) -> &mut Self {
        self.0.StrideInBytes = stride_in_bytes.0 as u32;
        self
    }

    pub fn with_stride_in_bytes(mut self, stride_in_bytes: ByteCount) -> Self {
        self.set_stride_in_bytes(stride_in_bytes);
        self
    }

    pub fn stride_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.StrideInBytes)
    }
}

/// Wrapper around D3D12_INPUT_ELEMENT_DESC structure
#[repr(transparent)]
#[derive(Debug, Hash, PartialOrd, Ord, PartialEq, Eq)]
pub struct InputElementDesc<'a>(
    pub D3D12_INPUT_ELEMENT_DESC,
    PhantomData<&'a CStr>,
);

impl<'a> Default for InputElementDesc<'a> {
    fn default() -> InputElementDesc<'a> {
        InputElementDesc(D3D12_INPUT_ELEMENT_DESC {
            SemanticName: std::ptr::null(),
            SemanticIndex: 0,
            Format: Format::Unknown as i32,
            InputSlot: 0,
            AlignedByteOffset: 0,
            InputSlotClass:
        D3D12_INPUT_CLASSIFICATION_D3D12_INPUT_CLASSIFICATION_PER_VERTEX_DATA,
            InstanceDataStepRate: 0,
        },
        PhantomData
    )
    }
}

// ToDo: macro for generating input element desc from vertex struct type?

impl<'a> InputElementDesc<'a> {
    pub fn set_semantic_name(
        &mut self,
        name: &'a str,
    ) -> Result<&mut Self, NulError> {
        let owned = CString::new(name)?;
        self.0.SemanticName = owned.into_raw() as *const i8;
        self.1 = PhantomData;
        Ok(self)
    }

    pub fn with_semantic_name(
        mut self,
        name: &'a str,
    ) -> Result<Self, NulError> {
        match self.set_semantic_name(name) {
            Ok(_) => Ok(self),
            Err(err) => Err(err),
        }
    }

    pub fn semantic_name(&self) -> Result<&'a str, Utf8Error> {
        Ok(unsafe { std::ffi::CStr::from_ptr(self.0.SemanticName).to_str()? })
    }

    pub fn set_semantic_index(&mut self, semantic_index: u32) -> &mut Self {
        self.0.SemanticIndex = semantic_index;
        self
    }

    pub fn with_semantic_index(mut self, semantic_index: u32) -> Self {
        self.set_semantic_index(semantic_index);
        self
    }

    pub fn semantic_index(&self) -> u32 {
        self.0.SemanticIndex
    }

    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_input_slot(&mut self, input_slot: u32) -> &mut Self {
        self.0.InputSlot = input_slot;
        self
    }

    pub fn with_input_slot(mut self, input_slot: u32) -> Self {
        self.set_input_slot(input_slot);
        self
    }

    pub fn input_slot(&self) -> u32 {
        self.0.InputSlot
    }

    pub fn set_aligned_byte_offset(
        &mut self,
        aligned_byte_offset: ByteCount,
    ) -> &mut Self {
        self.0.AlignedByteOffset = aligned_byte_offset.0 as u32;
        self
    }

    pub fn with_aligned_byte_offset(
        mut self,
        aligned_byte_offset: ByteCount,
    ) -> Self {
        self.set_aligned_byte_offset(aligned_byte_offset);
        self
    }

    pub fn aligned_byte_offset(&self) -> ByteCount {
        ByteCount::from(self.0.AlignedByteOffset)
    }

    pub fn set_input_slot_class(
        &mut self,
        input_slot_class: InputClassification,
    ) -> &mut Self {
        self.0.InputSlotClass = input_slot_class as i32;
        self
    }

    pub fn with_input_slot_class(
        mut self,
        input_slot_class: InputClassification,
    ) -> Self {
        self.set_input_slot_class(input_slot_class);
        self
    }

    pub fn input_slot_class(&self) -> InputClassification {
        unsafe { std::mem::transmute(self.0.InputSlotClass) }
    }

    pub fn set_instance_data_step_rate(
        &mut self,
        instance_data_step_rate: u32,
    ) -> &mut Self {
        self.0.InstanceDataStepRate = instance_data_step_rate;
        self
    }

    pub fn with_instance_data_step_rate(
        mut self,
        instance_data_step_rate: u32,
    ) -> Self {
        self.set_instance_data_step_rate(instance_data_step_rate);
        self
    }

    pub fn instance_data_step_rate(&self) -> u32 {
        self.0.InstanceDataStepRate
    }
}

// We need this because we transfer ownership of the CString "name" into
// the raw C string (const char*) "SemanticName". Since this memory has to be
// valid until the destruction of this struct, we need to regain that memory
// back so it can be destroyed correctly
impl<'a> Drop for InputElementDesc<'a> {
    fn drop(&mut self) {
        unsafe {
            let _regained_name = CString::from_raw(
                self.0.SemanticName as *mut std::os::raw::c_char,
            );
        }
    }
}

/// Wrapper around D3D12_INDEX_BUFFER_VIEW structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone, Copy)]
#[repr(transparent)]
pub struct IndexBufferView(pub(crate) D3D12_INDEX_BUFFER_VIEW);

impl IndexBufferView {
    pub fn set_buffer_location(
        &mut self,
        buffer_location: GpuVirtualAddress,
    ) -> &mut Self {
        self.0.BufferLocation = buffer_location.0;
        self
    }

    pub fn with_buffer_location(
        mut self,
        buffer_location: GpuVirtualAddress,
    ) -> Self {
        self.set_buffer_location(buffer_location);
        self
    }

    pub fn buffer_location(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(self.0.BufferLocation)
    }

    pub fn set_size_in_bytes(&mut self, size_in_bytes: ByteCount) -> &mut Self {
        self.0.SizeInBytes = size_in_bytes.0 as u32;
        self
    }

    pub fn with_size_in_bytes(mut self, size_in_bytes: ByteCount) -> Self {
        self.set_size_in_bytes(size_in_bytes);
        self
    }

    pub fn size_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.SizeInBytes)
    }

    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }
}

/// Wrapper around D3D12_SHADER_BYTECODE structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub struct ShaderBytecode<'a>(
    pub(crate) D3D12_SHADER_BYTECODE,
    PhantomData<&'a [u8]>,
);

impl<'a> Default for ShaderBytecode<'a> {
    fn default() -> ShaderBytecode<'a> {
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
    pub fn new(data: &'a [u8]) -> ShaderBytecode<'a> {
        Self(
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: data.as_ptr() as *const std::ffi::c_void,
                BytecodeLength: data.len() as u64,
            },
            PhantomData,
        )
    }
}

/// Wrapper around D3D12_SO_DECLARATION_ENTRY structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub struct SoDeclarationEntry<'a>(
    pub D3D12_SO_DECLARATION_ENTRY,
    PhantomData<&'a str>,
);

impl<'a> SoDeclarationEntry<'a> {
    pub fn set_stream(&mut self, stream: u32) -> &mut Self {
        self.0.Stream = stream;
        self
    }

    pub fn with_stream(mut self, stream: u32) -> Self {
        self.set_stream(stream);
        self
    }

    pub fn stream(&self) -> u32 {
        self.0.Stream
    }

    pub fn set_semantic_name(
        &mut self,
        name: &'a str,
    ) -> Result<&mut Self, NulError> {
        let owned = CString::new(name)?;
        self.0.SemanticName = owned.into_raw() as *const i8;
        self.1 = PhantomData;
        Ok(self)
    }

    pub fn with_semantic_name(
        mut self,
        name: &'a str,
    ) -> Result<Self, NulError> {
        match self.set_semantic_name(name) {
            Ok(_) => Ok(self),
            Err(err) => Err(err),
        }
    }

    pub fn semantic_name(&self) -> Result<&'a str, Utf8Error> {
        Ok(unsafe { std::ffi::CStr::from_ptr(self.0.SemanticName).to_str()? })
    }

    pub fn set_semantic_index(&mut self, semantic_index: u32) -> &mut Self {
        self.0.SemanticIndex = semantic_index;
        self
    }

    pub fn with_semantic_index(mut self, semantic_index: u32) -> Self {
        self.set_semantic_index(semantic_index);
        self
    }

    pub fn semantic_index(&self) -> u32 {
        self.0.SemanticIndex
    }

    pub fn set_start_component(&mut self, start_component: u8) -> &mut Self {
        self.0.StartComponent = start_component;
        self
    }

    pub fn with_start_component(mut self, start_component: u8) -> Self {
        self.set_start_component(start_component);
        self
    }

    pub fn start_component(&self) -> u8 {
        self.0.StartComponent
    }

    pub fn set_component_count(&mut self, component_count: u8) -> &mut Self {
        self.0.ComponentCount = component_count;
        self
    }

    pub fn with_component_count(mut self, component_count: u8) -> Self {
        self.set_component_count(component_count);
        self
    }

    pub fn component_count(&self) -> u8 {
        self.0.ComponentCount
    }

    pub fn set_output_slot(&mut self, output_slot: u8) -> &mut Self {
        self.0.OutputSlot = output_slot;
        self
    }

    pub fn with_output_slot(mut self, output_slot: u8) -> Self {
        self.set_output_slot(output_slot);
        self
    }

    pub fn output_slot(&self) -> u8 {
        self.0.OutputSlot
    }
}

// We need this because we transfer ownership of the CString "name" into
// the raw C string (const char*) "SemanticName". Since this memory has to be
// valid until the destruction of this struct, we need to regain that memory
// back so it can be destroyed correctly
impl<'a> Drop for SoDeclarationEntry<'a> {
    fn drop(&mut self) {
        unsafe {
            let _regained_name = CString::from_raw(
                self.0.SemanticName as *mut std::os::raw::c_char,
            );
        }
    }
}

/// Wrapper around D3D12_STREAM_OUTPUT_DESC structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug)]
pub struct StreamOutputDesc<'a>(
    pub D3D12_STREAM_OUTPUT_DESC,
    PhantomData<&'a [SoDeclarationEntry<'a>]>,
);

impl<'a> Default for StreamOutputDesc<'a> {
    fn default() -> Self {
        Self(
            D3D12_STREAM_OUTPUT_DESC {
                pSODeclaration: std::ptr::null(),
                NumEntries: 0,
                pBufferStrides: std::ptr::null(),
                NumStrides: 0,
                RasterizedStream: 0,
            },
            PhantomData,
        )
    }
}

impl<'a> StreamOutputDesc<'a> {
    pub fn set_so_declarations(
        &mut self,
        so_declarations: &'a [SoDeclarationEntry],
    ) -> &mut StreamOutputDesc<'a> {
        self.0.pSODeclaration =
            so_declarations.as_ptr() as *const D3D12_SO_DECLARATION_ENTRY;
        self.0.NumEntries = so_declarations.len() as u32;
        self.1 = PhantomData;
        self
    }

    pub fn with_so_declarations(
        mut self,
        so_declarations: &'a [SoDeclarationEntry],
    ) -> Self {
        self.set_so_declarations(so_declarations);
        self
    }

    pub fn so_declarations(&self) -> &'a [SoDeclarationEntry] {
        unsafe {
            slice::from_raw_parts(
                self.0.pSODeclaration as *const SoDeclarationEntry,
                self.0.NumEntries as usize,
            )
        }
    }

    // Note there are no setters since they are both useless and can break the invariant
    pub fn num_entries(&self) -> u32 {
        self.0.NumEntries
    }

    pub fn set_buffer_strides(&mut self, buffer_strides: &[u32]) -> &mut Self {
        self.0.pBufferStrides = buffer_strides.as_ptr();
        self.0.NumStrides = buffer_strides.len() as u32;
        self.1 = PhantomData;
        self
    }

    pub fn with_buffer_strides(mut self, buffer_strides: &[u32]) -> Self {
        self.set_buffer_strides(buffer_strides);
        self
    }

    pub fn buffer_strides(&self) -> &'a [u32] {
        unsafe {
            slice::from_raw_parts(
                self.0.pBufferStrides as *const u32,
                self.0.NumStrides as usize,
            )
        }
    }

    // Note there are no setters since they are both useless and can break the invariant
    pub fn num_strides(&self) -> u32 {
        self.0.NumStrides
    }

    pub fn set_rasterized_stream(
        &mut self,
        rasterized_stream: u32,
    ) -> &mut Self {
        self.0.RasterizedStream = rasterized_stream;
        self
    }

    pub fn with_rasterized_stream(mut self, rasterized_stream: u32) -> Self {
        self.set_rasterized_stream(rasterized_stream);
        self
    }

    pub fn rasterized_stream(&self) -> u32 {
        self.0.RasterizedStream
    }
}

/// Wrapper around D3D12_RENDER_TARGET_BLEND_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct RenderTargetBlendDesc(pub(crate) D3D12_RENDER_TARGET_BLEND_DESC);

// defaults from d3dx12.h
impl Default for RenderTargetBlendDesc {
    fn default() -> Self {
        Self(D3D12_RENDER_TARGET_BLEND_DESC {
            BlendEnable: 0,
            LogicOpEnable: 0,
            SrcBlend: Blend::One as i32,
            DestBlend: Blend::Zero as i32,
            BlendOp: BlendOp::Add as i32,
            SrcBlendAlpha: Blend::One as i32,
            DestBlendAlpha: Blend::Zero as i32,
            BlendOpAlpha: BlendOp::Add as i32,
            LogicOp: LogicOp::NoOp as i32,
            RenderTargetWriteMask:
                D3D12_COLOR_WRITE_ENABLE_D3D12_COLOR_WRITE_ENABLE_ALL as u8,
        })
    }
}

impl RenderTargetBlendDesc {
    pub fn set_blend_enable(&mut self, blend_enable: bool) -> &mut Self {
        self.0.BlendEnable = blend_enable as i32;
        self
    }

    pub fn with_blend_enable(mut self, blend_enable: bool) -> Self {
        self.set_blend_enable(blend_enable);
        self
    }

    pub fn blend_enable(&self) -> bool {
        self.0.BlendEnable != 0
    }

    pub fn set_logic_op_enable(&mut self, logic_op_enable: bool) -> &mut Self {
        self.0.LogicOpEnable = logic_op_enable as i32;
        self
    }

    pub fn with_logic_op_enable(mut self, logic_op_enable: bool) -> Self {
        self.set_logic_op_enable(logic_op_enable);
        self
    }

    pub fn logic_op_enable(&self) -> bool {
        self.0.LogicOpEnable != 0
    }

    pub fn set_src_blend(&mut self, src_blend: Blend) -> &mut Self {
        self.0.SrcBlend = src_blend as i32;
        self
    }

    pub fn with_src_blend(mut self, src_blend: Blend) -> Self {
        self.set_src_blend(src_blend);
        self
    }

    pub fn src_blend(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.SrcBlend) }
    }

    pub fn set_dest_blend(&mut self, dest_blend: Blend) -> &mut Self {
        self.0.DestBlend = dest_blend as i32;
        self
    }

    pub fn with_dest_blend(mut self, dest_blend: Blend) -> Self {
        self.set_dest_blend(dest_blend);
        self
    }

    pub fn dest_blend(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.DestBlend) }
    }

    pub fn set_blend_op(&mut self, blend_op: BlendOp) -> &mut Self {
        self.0.BlendOp = blend_op as i32;
        self
    }

    pub fn with_blend_op(mut self, blend_op: BlendOp) -> Self {
        self.set_blend_op(blend_op);
        self
    }

    pub fn blend_op(&self) -> BlendOp {
        unsafe { std::mem::transmute(self.0.BlendOp) }
    }

    pub fn set_src_blend_alpha(&mut self, src_blend_alpha: Blend) -> &mut Self {
        self.0.SrcBlendAlpha = src_blend_alpha as i32;
        self
    }

    pub fn with_src_blend_alpha(mut self, src_blend_alpha: Blend) -> Self {
        self.set_src_blend_alpha(src_blend_alpha);
        self
    }

    pub fn src_blend_alpha(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.SrcBlendAlpha) }
    }

    pub fn set_dest_blend_alpha(
        &mut self,
        dest_blend_alpha: Blend,
    ) -> &mut Self {
        self.0.DestBlendAlpha = dest_blend_alpha as i32;
        self
    }

    pub fn with_dest_blend_alpha(mut self, dest_blend_alpha: Blend) -> Self {
        self.set_dest_blend_alpha(dest_blend_alpha);
        self
    }

    pub fn dest_blend_alpha(&self) -> Blend {
        unsafe { std::mem::transmute(self.0.DestBlendAlpha) }
    }

    pub fn set_blend_op_alpha(&mut self, blend_op_alpha: BlendOp) -> &mut Self {
        self.0.BlendOpAlpha = blend_op_alpha as i32;
        self
    }

    pub fn with_blend_op_alpha(mut self, blend_op_alpha: BlendOp) -> Self {
        self.set_blend_op_alpha(blend_op_alpha);
        self
    }

    pub fn blend_op_alpha(&self) -> BlendOp {
        unsafe { std::mem::transmute(self.0.BlendOpAlpha) }
    }

    pub fn set_logic_op(&mut self, logic_op: LogicOp) -> &mut Self {
        self.0.LogicOp = logic_op as i32;
        self
    }

    pub fn with_logic_op(mut self, logic_op: LogicOp) -> Self {
        self.set_logic_op(logic_op);
        self
    }

    pub fn logic_op(&self) -> LogicOp {
        unsafe { std::mem::transmute(self.0.LogicOp) }
    }

    pub fn set_render_target_write_mask(
        &mut self,
        render_target_write_mask: ColorWriteEnable,
    ) -> &mut Self {
        self.0.RenderTargetWriteMask = render_target_write_mask.bits() as u8;
        self
    }

    pub fn with_render_target_write_mask(
        mut self,
        render_target_write_mask: ColorWriteEnable,
    ) -> Self {
        self.set_render_target_write_mask(render_target_write_mask);
        self
    }

    pub fn render_target_write_mask(&self) -> ColorWriteEnable {
        unsafe {
            ColorWriteEnable::from_bits_unchecked(
                self.0.RenderTargetWriteMask as i32,
            )
        }
    }
}

/// Wrapper around D3D12_BLEND_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct BlendDesc(pub(crate) D3D12_BLEND_DESC);

// defaults from d3dx12.h
impl Default for BlendDesc {
    fn default() -> Self {
        Self(D3D12_BLEND_DESC {
            AlphaToCoverageEnable: 0,
            IndependentBlendEnable: 0,
            RenderTarget: [RenderTargetBlendDesc::default().0; 8usize],
        })
    }
}

impl BlendDesc {
    pub fn set_alpha_to_coverage_enable(
        &mut self,
        alpha_to_coverage_enable: bool,
    ) -> &mut Self {
        self.0.AlphaToCoverageEnable = alpha_to_coverage_enable as i32;
        self
    }

    pub fn with_alpha_to_coverage_enable(
        mut self,
        alpha_to_coverage_enable: bool,
    ) -> Self {
        self.set_alpha_to_coverage_enable(alpha_to_coverage_enable);
        self
    }

    pub fn alpha_to_coverage_enable(&self) -> bool {
        self.0.AlphaToCoverageEnable != 0
    }

    pub fn set_independent_blend_enable(
        &mut self,
        independent_blend_enable: bool,
    ) -> &mut Self {
        self.0.IndependentBlendEnable = independent_blend_enable as i32;
        self
    }

    pub fn with_independent_blend_enable(
        mut self,
        independent_blend_enable: bool,
    ) -> Self {
        self.set_independent_blend_enable(independent_blend_enable);
        self
    }

    pub fn independent_blend_enable(&self) -> bool {
        self.0.IndependentBlendEnable != 0
    }

    pub fn set_render_targets(
        &mut self,
        rt_blend_descs: &[RenderTargetBlendDesc],
    ) -> &mut Self {
        for rt_index in 0..rt_blend_descs.len() {
            // transmute is okay due to repr::transparent
            self.0.RenderTarget[rt_index] =
                unsafe { std::mem::transmute(rt_blend_descs[rt_index]) };
        }
        self
    }

    pub fn with_render_targets(
        mut self,
        rt_blend_descs: &[RenderTargetBlendDesc],
    ) -> Self {
        self.set_render_targets(rt_blend_descs);
        self
    }

    pub fn render_targets(
        &self,
    ) -> [RenderTargetBlendDesc; SIMULTANEOUS_RENDER_TARGET_COUNT as usize]
    {
        // transmute is okay due to repr::transparent
        unsafe { std::mem::transmute(self.0.RenderTarget) }
    }
}

/// Wrapper around D3D12_RASTERIZER_DESC structure
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct RasterizerDesc(pub(crate) D3D12_RASTERIZER_DESC);

// defaults from d3dx12.h
impl Default for RasterizerDesc {
    fn default() -> Self {
        Self(D3D12_RASTERIZER_DESC {
            FillMode: FillMode::Solid as i32,
            CullMode: CullMode::Back as i32,
            FrontCounterClockwise: 0,
            DepthBias: DEFAULT_DEPTH_BIAS as i32,
            DepthBiasClamp: DEFAULT_DEPTH_BIAS_CLAMP as f32,
            SlopeScaledDepthBias: DEFAULT_SLOPE_SCALED_DEPTH_BIAS as f32,
            DepthClipEnable: 1,
            MultisampleEnable: 0,
            AntialiasedLineEnable: 0,
            ForcedSampleCount: 0,
            ConservativeRaster: ConservativeRasterizationMode::Off as i32,
        })
    }
}

impl RasterizerDesc {
    pub fn set_fill_mode(&mut self, fill_mode: FillMode) -> &mut Self {
        self.0.FillMode = fill_mode as i32;
        self
    }

    pub fn with_fill_mode(mut self, fill_mode: FillMode) -> Self {
        self.set_fill_mode(fill_mode);
        self
    }

    pub fn fill_mode(&self) -> FillMode {
        unsafe { std::mem::transmute(self.0.FillMode) }
    }

    pub fn set_cull_mode(&mut self, cull_mode: CullMode) -> &mut Self {
        self.0.CullMode = cull_mode as i32;
        self
    }

    pub fn with_cull_mode(mut self, cull_mode: CullMode) -> Self {
        self.set_cull_mode(cull_mode);
        self
    }

    pub fn cull_mode(&self) -> CullMode {
        unsafe { std::mem::transmute(self.0.CullMode) }
    }

    pub fn set_front_counter_clockwise(
        &mut self,
        front_counter_clockwise: bool,
    ) -> &mut Self {
        self.0.FrontCounterClockwise = front_counter_clockwise as i32;
        self
    }

    pub fn with_front_counter_clockwise(
        mut self,
        front_counter_clockwise: bool,
    ) -> Self {
        self.set_front_counter_clockwise(front_counter_clockwise);
        self
    }

    pub fn front_counter_clockwise(&self) -> bool {
        self.0.FrontCounterClockwise != 0
    }

    pub fn set_depth_bias(&mut self, depth_bias: i32) -> &mut Self {
        self.0.DepthBias = depth_bias;
        self
    }

    pub fn with_depth_bias(mut self, depth_bias: i32) -> Self {
        self.set_depth_bias(depth_bias);
        self
    }

    pub fn depth_bias(&self) -> i32 {
        self.0.DepthBias
    }

    pub fn set_depth_bias_clamp(&mut self, depth_bias_clamp: f32) -> &mut Self {
        self.0.DepthBiasClamp = depth_bias_clamp;
        self
    }

    pub fn with_depth_bias_clamp(mut self, depth_bias_clamp: f32) -> Self {
        self.set_depth_bias_clamp(depth_bias_clamp);
        self
    }

    pub fn depth_bias_clamp(&self) -> f32 {
        self.0.DepthBiasClamp
    }

    pub fn set_slope_scaled_depth_bias(
        &mut self,
        slope_scaled_depth_bias: f32,
    ) -> &mut Self {
        self.0.SlopeScaledDepthBias = slope_scaled_depth_bias;
        self
    }

    pub fn with_slope_scaled_depth_bias(
        mut self,
        slope_scaled_depth_bias: f32,
    ) -> Self {
        self.set_slope_scaled_depth_bias(slope_scaled_depth_bias);
        self
    }

    pub fn slope_scaled_depth_bias(&self) -> f32 {
        self.0.SlopeScaledDepthBias
    }

    pub fn set_depth_clip_enable(
        &mut self,
        depth_clip_enable: bool,
    ) -> &mut Self {
        self.0.DepthClipEnable = depth_clip_enable as i32;
        self
    }

    pub fn with_depth_clip_enable(mut self, depth_clip_enable: bool) -> Self {
        self.set_depth_clip_enable(depth_clip_enable);
        self
    }

    pub fn depth_clip_enable(&self) -> bool {
        self.0.DepthClipEnable != 0
    }

    pub fn set_multisample_enable(
        &mut self,
        multisample_enable: bool,
    ) -> &mut Self {
        self.0.MultisampleEnable = multisample_enable as i32;
        self
    }

    pub fn with_multisample_enable(mut self, multisample_enable: bool) -> Self {
        self.set_multisample_enable(multisample_enable);
        self
    }

    pub fn multisample_enable(&self) -> bool {
        self.0.MultisampleEnable != 0
    }

    pub fn set_antialiased_line_enable(
        &mut self,
        antialiased_line_enable: bool,
    ) -> &mut Self {
        self.0.AntialiasedLineEnable = antialiased_line_enable as i32;
        self
    }

    pub fn with_antialiased_line_enable(
        mut self,
        antialiased_line_enable: bool,
    ) -> Self {
        self.set_antialiased_line_enable(antialiased_line_enable);
        self
    }

    pub fn antialiased_line_enable(&self) -> bool {
        self.0.AntialiasedLineEnable != 0
    }

    pub fn set_forced_sample_count(
        &mut self,
        forced_sample_count: u32,
    ) -> &mut Self {
        self.0.ForcedSampleCount = forced_sample_count;
        self
    }

    pub fn with_forced_sample_count(
        mut self,
        forced_sample_count: u32,
    ) -> Self {
        self.set_forced_sample_count(forced_sample_count);
        self
    }

    pub fn forced_sample_count(&self) -> u32 {
        self.0.ForcedSampleCount
    }

    pub fn set_conservative_raster(
        &mut self,
        conservative_raster: ConservativeRasterizationMode,
    ) -> &mut Self {
        self.0.ConservativeRaster = conservative_raster as i32;
        self
    }

    pub fn with_conservative_raster(
        mut self,
        conservative_raster: ConservativeRasterizationMode,
    ) -> Self {
        self.set_conservative_raster(conservative_raster);
        self
    }

    pub fn conservative_raster(&self) -> ConservativeRasterizationMode {
        unsafe { std::mem::transmute(self.0.ConservativeRaster) }
    }
}

/// Wrapper around D3D12_DEPTH_STENCILOP_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct DepthStencilOpDesc(pub(crate) D3D12_DEPTH_STENCILOP_DESC);

// defaults from d3dx12.h
impl Default for DepthStencilOpDesc {
    fn default() -> Self {
        Self(D3D12_DEPTH_STENCILOP_DESC {
            StencilFailOp: StencilOp::Keep as i32,
            StencilDepthFailOp: StencilOp::Keep as i32,
            StencilPassOp: StencilOp::Keep as i32,
            StencilFunc: ComparisonFunc::Always as i32,
        })
    }
}

impl DepthStencilOpDesc {
    pub fn set_stencil_fail_op(
        &mut self,
        stencil_fail_op: StencilOp,
    ) -> &mut Self {
        self.0.StencilFailOp = stencil_fail_op as i32;
        self
    }

    pub fn with_stencil_fail_op(mut self, stencil_fail_op: StencilOp) -> Self {
        self.set_stencil_fail_op(stencil_fail_op);
        self
    }

    pub fn stencil_fail_op(&self) -> StencilOp {
        unsafe { std::mem::transmute(self.0.StencilFailOp) }
    }

    pub fn set_stencil_depth_fail_op(
        &mut self,
        stencil_depth_fail_op: StencilOp,
    ) -> &mut Self {
        self.0.StencilDepthFailOp = stencil_depth_fail_op as i32;
        self
    }

    pub fn with_stencil_depth_fail_op(
        mut self,
        stencil_depth_fail_op: StencilOp,
    ) -> Self {
        self.set_stencil_depth_fail_op(stencil_depth_fail_op);
        self
    }

    pub fn stencil_depth_fail_op(&self) -> StencilOp {
        unsafe { std::mem::transmute(self.0.StencilDepthFailOp) }
    }

    pub fn set_stencil_pass_op(
        &mut self,
        stencil_pass_op: StencilOp,
    ) -> &mut Self {
        self.0.StencilPassOp = stencil_pass_op as i32;
        self
    }

    pub fn with_stencil_pass_op(mut self, stencil_pass_op: StencilOp) -> Self {
        self.set_stencil_pass_op(stencil_pass_op);
        self
    }

    pub fn stencil_pass_op(&self) -> StencilOp {
        unsafe { std::mem::transmute(self.0.StencilPassOp) }
    }

    pub fn set_stencil_func(
        &mut self,
        stencil_func: ComparisonFunc,
    ) -> &mut Self {
        self.0.StencilFunc = stencil_func as i32;
        self
    }

    pub fn with_stencil_func(mut self, stencil_func: ComparisonFunc) -> Self {
        self.set_stencil_func(stencil_func);
        self
    }

    pub fn stencil_func(&self) -> ComparisonFunc {
        unsafe { std::mem::transmute(self.0.StencilFunc) }
    }
}

/// Wrapper around D3D12_DEPTH_STENCIL_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct DepthStencilDesc(pub(crate) D3D12_DEPTH_STENCIL_DESC);

// defaults from d3dx12.h: less depth test with writes; no stencil
impl Default for DepthStencilDesc {
    fn default() -> Self {
        Self(D3D12_DEPTH_STENCIL_DESC {
            DepthEnable: 1,
            DepthWriteMask: DepthWriteMask::All as i32,
            DepthFunc: ComparisonFunc::Less as i32,
            StencilEnable: 0,
            StencilReadMask: DEFAULT_STENCIL_READ_MASK as u8,
            StencilWriteMask: DEFAULT_STENCIL_WRITE_MASK as u8,
            FrontFace: DepthStencilOpDesc::default().0,
            BackFace: DepthStencilOpDesc::default().0,
        })
    }
}

impl DepthStencilDesc {
    pub fn set_depth_enable(&mut self, depth_enable: bool) -> &mut Self {
        self.0.DepthEnable = depth_enable as i32;
        self
    }

    pub fn with_depth_enable(mut self, depth_enable: bool) -> Self {
        self.set_depth_enable(depth_enable);
        self
    }

    pub fn depth_enable(&self) -> bool {
        self.0.DepthEnable != 0
    }

    pub fn set_depth_write_mask(
        &mut self,
        depth_write_mask: DepthWriteMask,
    ) -> &mut Self {
        self.0.DepthWriteMask = depth_write_mask as i32;
        self
    }

    pub fn with_depth_write_mask(
        mut self,
        depth_write_mask: DepthWriteMask,
    ) -> Self {
        self.set_depth_write_mask(depth_write_mask);
        self
    }

    pub fn depth_write_mask(&self) -> DepthWriteMask {
        unsafe { std::mem::transmute(self.0.DepthWriteMask) }
    }

    pub fn set_depth_func(&mut self, depth_func: ComparisonFunc) -> &mut Self {
        self.0.DepthFunc = depth_func as i32;
        self
    }

    pub fn with_depth_func(mut self, depth_func: ComparisonFunc) -> Self {
        self.set_depth_func(depth_func);
        self
    }

    pub fn depth_func(&self) -> ComparisonFunc {
        unsafe { std::mem::transmute(self.0.DepthFunc) }
    }

    pub fn set_stencil_enable(&mut self, stencil_enable: bool) -> &mut Self {
        self.0.StencilEnable = stencil_enable as i32;
        self
    }

    pub fn with_stencil_enable(mut self, stencil_enable: bool) -> Self {
        self.set_stencil_enable(stencil_enable);
        self
    }

    pub fn stencil_enable(&self) -> bool {
        self.0.StencilEnable != 0
    }

    pub fn set_stencil_read_mask(
        &mut self,
        stencil_read_mask: u8,
    ) -> &mut Self {
        self.0.StencilReadMask = stencil_read_mask;
        self
    }

    pub fn with_stencil_read_mask(mut self, stencil_read_mask: u8) -> Self {
        self.set_stencil_read_mask(stencil_read_mask);
        self
    }

    pub fn stencil_read_mask(&self) -> u8 {
        self.0.StencilReadMask
    }

    pub fn set_stencil_write_mask(
        &mut self,
        stencil_write_mask: u8,
    ) -> &mut Self {
        self.0.StencilWriteMask = stencil_write_mask;
        self
    }

    pub fn with_stencil_write_mask(mut self, stencil_write_mask: u8) -> Self {
        self.set_stencil_write_mask(stencil_write_mask);
        self
    }

    pub fn stencil_write_mask(&self) -> u8 {
        self.0.StencilWriteMask
    }

    pub fn set_front_face(
        &mut self,
        front_face: DepthStencilOpDesc,
    ) -> &mut Self {
        self.0.FrontFace = front_face.0;
        self
    }

    pub fn with_front_face(mut self, front_face: DepthStencilOpDesc) -> Self {
        self.set_front_face(front_face);
        self
    }

    pub fn front_face(&self) -> DepthStencilOpDesc {
        DepthStencilOpDesc(self.0.FrontFace)
    }

    pub fn set_back_face(
        &mut self,
        back_face: DepthStencilOpDesc,
    ) -> &mut Self {
        self.0.BackFace = back_face.0;
        self
    }

    pub fn with_back_face(mut self, back_face: DepthStencilOpDesc) -> Self {
        self.set_back_face(back_face);
        self
    }

    pub fn back_face(&self) -> DepthStencilOpDesc {
        DepthStencilOpDesc(self.0.BackFace)
    }
}

/// Wrapper around D3D12_INPUT_LAYOUT_DESC structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug)]
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

// ToDo: ShaderBytecode is a similar struct, but it uses new() method
impl<'a> InputLayoutDesc<'a> {
    pub fn set_input_elements(
        &mut self,
        layout: &'a [InputElementDesc<'a>],
    ) -> &mut Self {
        self.0.pInputElementDescs =
            layout.as_ptr() as *const D3D12_INPUT_ELEMENT_DESC;
        self.0.NumElements = layout.len() as u32;
        self.1 = PhantomData;
        self
    }

    pub fn with_input_elements(
        mut self,
        layout: &'a [InputElementDesc<'a>],
    ) -> Self {
        self.set_input_elements(layout);
        self
    }

    pub fn input_elements(&self) -> &'a [InputElementDesc] {
        unsafe {
            slice::from_raw_parts(
                self.0.pInputElementDescs as *const InputElementDesc,
                self.0.NumElements as usize,
            )
        }
    }
}

/// Wrapper around D3D12_CACHED_PIPELINE_STATE structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug)]
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
    pub fn set_cached_blob(&mut self, cached_blob: &'a [u8]) -> &mut Self {
        self.0.pCachedBlob = cached_blob.as_ptr() as *const std::ffi::c_void;
        self.0.CachedBlobSizeInBytes = cached_blob.len() as u64;
        self.1 = PhantomData;
        self
    }

    pub fn with_cached_blob(mut self, cached_blob: &'a [u8]) -> Self {
        self.set_cached_blob(cached_blob);
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
}

// ToDo: do we need different lifetimes for all shaders?
/// Wrapper around D3D12_GRAPHICS_PIPELINE_STATE_DESC structure
#[repr(transparent)]
#[derive(Debug)]
pub struct GraphicsPipelineStateDesc<'rs, 'sh, 'so, 'il>(
    pub D3D12_GRAPHICS_PIPELINE_STATE_DESC,
    PhantomData<&'rs RootSignature>,
    PhantomData<&'sh ShaderBytecode<'sh>>,
    PhantomData<&'so StreamOutputDesc<'so>>,
    PhantomData<&'il InputLayoutDesc<'il>>,
);

impl<'rs, 'sh, 'so, 'il> Default
    for GraphicsPipelineStateDesc<'rs, 'sh, 'so, 'il>
{
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
                IBStripCutValue: IndexBufferStripCutValue::Disabled as i32,
                PrimitiveTopologyType: PrimitiveTopologyType::Undefined as i32,
                NumRenderTargets: SIMULTANEOUS_RENDER_TARGET_COUNT as u32,
                RTVFormats: [Format::Unknown as i32;
                    SIMULTANEOUS_RENDER_TARGET_COUNT as usize],
                DSVFormat: Format::Unknown as i32,
                SampleDesc: SampleDesc::default().0,
                NodeMask: 0,
                CachedPSO: CachedPipelineState::default().0,
                Flags: PipelineStateFlags::None.bits(),
            },
            PhantomData, // rs
            PhantomData, // sh
            PhantomData, // so
            PhantomData, // il
        )
    }
}

impl<'rs, 'sh, 'so, 'il> GraphicsPipelineStateDesc<'rs, 'sh, 'so, 'il> {
    pub fn set_root_signature(
        &mut self,
        root_signature: &'rs RootSignature,
    ) -> &mut Self {
        self.0.pRootSignature = root_signature.this;
        self.1 = PhantomData;
        self
    }

    pub fn with_root_signature(
        mut self,
        root_signature: &'rs RootSignature,
    ) -> GraphicsPipelineStateDesc<'rs, 'sh, 'so, 'il> {
        self.set_root_signature(root_signature);
        self
    }

    pub fn root_signature(&self) -> RootSignature {
        let root_signature = RootSignature {
            this: self.0.pRootSignature,
        };
        root_signature.add_ref();
        root_signature
    }

    pub fn set_vs_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.0.VS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_vs_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_vs_bytecode(bytecode);
        self
    }

    pub fn vs_bytecode(&self) -> &'sh ShaderBytecode {
        unsafe {
            &*(&self.0.VS as *const D3D12_SHADER_BYTECODE
                as *const ShaderBytecode)
        }
    }

    pub fn set_ps_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.0.PS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_ps_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_ps_bytecode(bytecode);
        self
    }

    pub fn ps_bytecode(&self) -> &'sh ShaderBytecode {
        unsafe {
            &*(&self.0.PS as *const D3D12_SHADER_BYTECODE
                as *const ShaderBytecode)
        }
    }

    pub fn set_ds_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.0.DS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_ds_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_ds_bytecode(bytecode);
        self
    }

    pub fn ds_bytecode(&self) -> &'sh ShaderBytecode {
        unsafe {
            &*(&self.0.DS as *const D3D12_SHADER_BYTECODE
                as *const ShaderBytecode)
        }
    }

    pub fn set_hs_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.0.HS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_hs_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_hs_bytecode(bytecode);
        self
    }

    pub fn hs_bytecode(&self) -> &'sh ShaderBytecode {
        unsafe {
            &*(&self.0.HS as *const D3D12_SHADER_BYTECODE
                as *const ShaderBytecode)
        }
    }

    pub fn set_gs_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.0.GS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_gs_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_gs_bytecode(bytecode);
        self
    }

    pub fn gs_bytecode(&self) -> &'sh ShaderBytecode {
        unsafe {
            &*(&self.0.GS as *const D3D12_SHADER_BYTECODE
                as *const ShaderBytecode)
        }
    }

    pub fn set_stream_output(
        &mut self,
        stream_output: StreamOutputDesc,
    ) -> &mut Self {
        self.0.StreamOutput = stream_output.0;
        self
    }

    pub fn with_stream_output(
        mut self,
        stream_output: StreamOutputDesc,
    ) -> Self {
        self.set_stream_output(stream_output);
        self
    }

    pub fn stream_output(&self) -> &'so StreamOutputDesc {
        unsafe {
            &*(&self.0.StreamOutput as *const D3D12_STREAM_OUTPUT_DESC
                as *const StreamOutputDesc)
        }
    }

    pub fn set_blend_state(&mut self, blend_state: BlendDesc) -> &mut Self {
        self.0.BlendState = blend_state.0;
        self
    }

    pub fn with_blend_state(mut self, blend_state: BlendDesc) -> Self {
        self.set_blend_state(blend_state);
        self
    }

    pub fn blend_state(&self) -> BlendDesc {
        BlendDesc(self.0.BlendState)
    }

    pub fn set_sample_mask(&mut self, sample_mask: u32) -> &mut Self {
        self.0.SampleMask = sample_mask;
        self
    }

    pub fn with_sample_mask(mut self, sample_mask: u32) -> Self {
        self.set_sample_mask(sample_mask);
        self
    }
    pub fn sample_mask(&self) -> u32 {
        self.0.SampleMask
    }

    pub fn set_rasterizer_state(
        &mut self,
        rasterizer_state: RasterizerDesc,
    ) -> &mut Self {
        self.0.RasterizerState = rasterizer_state.0;
        self
    }

    pub fn with_rasterizer_state(
        mut self,
        rasterizer_state: RasterizerDesc,
    ) -> Self {
        self.set_rasterizer_state(rasterizer_state);
        self
    }

    pub fn rasterizer_state(&self) -> RasterizerDesc {
        RasterizerDesc(self.0.RasterizerState)
    }

    pub fn set_depth_stencil_state(
        &mut self,
        depth_stencil_state: DepthStencilDesc,
    ) -> &mut Self {
        self.0.DepthStencilState = depth_stencil_state.0;
        self
    }

    pub fn with_depth_stencil_state(
        mut self,
        depth_stencil_state: DepthStencilDesc,
    ) -> Self {
        self.set_depth_stencil_state(depth_stencil_state);
        self
    }

    pub fn depth_stencil_state(&self) -> DepthStencilDesc {
        DepthStencilDesc(self.0.DepthStencilState)
    }

    pub fn set_input_layout(
        &mut self,
        input_layout: &'il InputLayoutDesc,
    ) -> &mut Self {
        self.0.InputLayout = input_layout.0;
        self.4 = PhantomData;
        self
    }

    pub fn with_input_layout(
        mut self,
        input_layout: &'il InputLayoutDesc,
    ) -> Self {
        self.set_input_layout(input_layout);
        self
    }

    pub fn input_layout(&self) -> &'il InputLayoutDesc {
        unsafe {
            &*(&self.0.InputLayout as *const D3D12_INPUT_LAYOUT_DESC
                as *const InputLayoutDesc)
        }
    }

    pub fn set_ib_strip_cut_value(
        &mut self,
        ib_strip_cut_value: IndexBufferStripCutValue,
    ) -> &mut Self {
        self.0.IBStripCutValue = ib_strip_cut_value as i32;
        self
    }

    pub fn with_ib_strip_cut_value(
        mut self,
        ib_strip_cut_value: IndexBufferStripCutValue,
    ) -> Self {
        self.set_ib_strip_cut_value(ib_strip_cut_value);
        self
    }

    pub fn ib_strip_cut_value(&self) -> IndexBufferStripCutValue {
        unsafe { std::mem::transmute(self.0.IBStripCutValue) }
    }

    pub fn set_primitive_topology_type(
        &mut self,
        primitive_topology_type: PrimitiveTopologyType,
    ) -> &mut Self {
        self.0.PrimitiveTopologyType = primitive_topology_type as i32;
        self
    }

    pub fn with_primitive_topology_type(
        mut self,
        primitive_topology_type: PrimitiveTopologyType,
    ) -> Self {
        self.set_primitive_topology_type(primitive_topology_type);
        self
    }

    pub fn primitive_topology_type(&self) -> PrimitiveTopologyType {
        unsafe { std::mem::transmute(self.0.PrimitiveTopologyType) }
    }

    pub fn set_rtv_formats(&mut self, rtv_formats: &[Format]) -> &mut Self {
        for format_index in 0..rtv_formats.len() {
            self.0.RTVFormats[format_index] = rtv_formats[format_index] as i32;
        }
        self.0.NumRenderTargets = rtv_formats.len() as u32;
        self
    }

    pub fn with_rtv_formats(mut self, rtv_formats: &[Format]) -> Self {
        self.set_rtv_formats(rtv_formats);
        self
    }

    pub fn rtv_formats(&self) -> &[Format] {
        unsafe {
            slice::from_raw_parts(
                self.0.RTVFormats.as_ptr() as *const Format,
                self.0.NumRenderTargets as usize,
            )
        }
    }

    // Note there are no setters since they are both useless and can break the invariant
    pub fn num_render_targets(&self) -> u32 {
        self.0.NumRenderTargets
    }

    pub fn set_dsv_format(&mut self, dsv_format: Format) -> &mut Self {
        self.0.DSVFormat = dsv_format as i32;
        self
    }

    pub fn with_dsv_format(mut self, dsv_format: Format) -> Self {
        self.set_dsv_format(dsv_format);
        self
    }

    pub fn dsv_format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.DSVFormat) }
    }

    pub fn set_sample_desc(&mut self, sample_desc: SampleDesc) -> &mut Self {
        self.0.SampleDesc = sample_desc.0;
        self
    }

    pub fn with_sample_desc(mut self, sample_desc: SampleDesc) -> Self {
        self.set_sample_desc(sample_desc);
        self
    }

    pub fn sample_desc(&self) -> SampleDesc {
        SampleDesc(self.0.SampleDesc)
    }

    pub fn set_node_mask(&mut self, node_mask: u32) -> &mut Self {
        self.0.NodeMask = node_mask;
        self
    }

    pub fn with_node_mask(mut self, node_mask: u32) -> Self {
        self.set_node_mask(node_mask);
        self
    }

    pub fn node_mask(&self) -> u32 {
        self.0.NodeMask
    }

    pub fn set_cached_pso(
        &mut self,
        cached_pso: &'sh CachedPipelineState,
    ) -> &mut Self {
        self.0.CachedPSO = cached_pso.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_cached_pso(
        mut self,
        cached_pso: &'sh CachedPipelineState,
    ) -> Self {
        self.set_cached_pso(cached_pso);
        self
    }

    // ToDo: probably it'd be simpler to just have one lifetime
    // parameter on GraphicsPipelineStateDesc?
    pub fn cached_pso(&self) -> &'sh CachedPipelineState {
        unsafe {
            &*(&self.0.CachedPSO as *const D3D12_CACHED_PIPELINE_STATE
                as *const CachedPipelineState)
        }
    }

    pub fn set_flags(&mut self, flags: PipelineStateFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: PipelineStateFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> PipelineStateFlags {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

/// Wrapper around D3D12_COMPUTE_PIPELINE_STATE_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct ComputePipelineStateDesc<'rs, 'sh>(
    pub D3D12_COMPUTE_PIPELINE_STATE_DESC,
    PhantomData<&'rs RootSignature>,
    PhantomData<&'sh ShaderBytecode<'sh>>,
);

impl<'rs, 'sh> ComputePipelineStateDesc<'rs, 'sh> {
    pub fn set_root_signature(
        &mut self,
        root_signature: &'rs RootSignature,
    ) -> &mut ComputePipelineStateDesc<'rs, 'sh> {
        self.0.pRootSignature = root_signature.this;
        self.1 = PhantomData;
        self
    }

    pub fn with_root_signature(
        mut self,
        root_signature: &'rs RootSignature,
    ) -> ComputePipelineStateDesc<'rs, 'sh> {
        self.set_root_signature(root_signature);
        self
    }

    pub fn root_signature(&self) -> RootSignature {
        let root_signature = RootSignature {
            this: self.0.pRootSignature,
        };
        root_signature.add_ref();
        root_signature
    }

    pub fn set_cs_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut ComputePipelineStateDesc<'rs, 'sh> {
        self.0.CS = bytecode.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_cs_bytecode(
        mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> ComputePipelineStateDesc<'rs, 'sh> {
        self.set_cs_bytecode(bytecode);
        self
    }

    pub fn cs_bytecode(&self) -> &'sh ShaderBytecode {
        unsafe {
            &*(&self.0.CS as *const D3D12_SHADER_BYTECODE
                as *const ShaderBytecode)
        }
    }

    pub fn set_node_mask(&mut self, node_mask: u32) -> &mut Self {
        self.0.NodeMask = node_mask;
        self
    }

    pub fn with_node_mask(mut self, node_mask: u32) -> Self {
        self.set_node_mask(node_mask);
        self
    }

    pub fn node_mask(&self) -> u32 {
        self.0.NodeMask
    }

    pub fn set_cached_pso(
        &mut self,
        cached_pso: &'sh CachedPipelineState,
    ) -> &mut ComputePipelineStateDesc<'rs, 'sh> {
        self.0.CachedPSO = cached_pso.0;
        self.2 = PhantomData;
        self
    }

    pub fn with_cached_pso(
        mut self,
        cached_pso: &'sh CachedPipelineState,
    ) -> ComputePipelineStateDesc<'rs, 'sh> {
        self.set_cached_pso(cached_pso);
        self
    }

    // ToDo: probably it'd be simpler to just have one lifetime
    // parameter on ComputePipelineStateDesc?
    pub fn cached_pso(&self) -> &'sh CachedPipelineState {
        unsafe {
            &*(&self.0.CachedPSO as *const D3D12_CACHED_PIPELINE_STATE
                as *const CachedPipelineState)
        }
    }

    pub fn set_flags(
        &mut self,
        pipeline_state_flags: PipelineStateFlags,
    ) -> &mut Self {
        self.0.Flags = pipeline_state_flags.bits();
        self
    }

    pub fn with_flags(
        mut self,
        pipeline_state_flags: PipelineStateFlags,
    ) -> Self {
        self.set_flags(pipeline_state_flags);
        self
    }

    pub fn flags(&self) -> PipelineStateFlags {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

/// Wrapper around D3D12_SUBRESOURCE_FOOTPRINT structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct SubresourceFootprint(pub(crate) D3D12_SUBRESOURCE_FOOTPRINT);

impl Default for SubresourceFootprint {
    fn default() -> Self {
        Self(D3D12_SUBRESOURCE_FOOTPRINT {
            Format: Format::R8G8B8A8Unorm as i32,
            Width: 0,
            Height: 1,
            Depth: 1,
            RowPitch: 0,
        })
    }
}

impl SubresourceFootprint {
    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_width(&mut self, width: u32) -> &mut Self {
        self.0.Width = width;
        self
    }

    pub fn with_width(mut self, width: u32) -> Self {
        self.set_width(width);
        self
    }

    pub fn width(&self) -> u32 {
        self.0.Width
    }

    pub fn set_height(&mut self, height: u32) -> &mut Self {
        self.0.Height = height;
        self
    }

    pub fn with_height(mut self, height: u32) -> Self {
        self.set_height(height);
        self
    }

    pub fn height(&self) -> u32 {
        self.0.Height
    }

    pub fn set_depth(&mut self, depth: u32) -> &mut Self {
        self.0.Depth = depth;
        self
    }

    pub fn with_depth(mut self, depth: u32) -> Self {
        self.set_depth(depth);
        self
    }

    pub fn depth(&self) -> u32 {
        self.0.Depth
    }

    pub fn set_row_pitch(&mut self, row_pitch: ByteCount) -> &mut Self {
        self.0.RowPitch = row_pitch.0 as u32;
        self
    }

    pub fn with_row_pitch(mut self, row_pitch: ByteCount) -> Self {
        self.set_row_pitch(row_pitch);
        self
    }

    pub fn row_pitch(&self) -> ByteCount {
        ByteCount::from(self.0.RowPitch)
    }
}

/// Wrapper around D3D12_PLACED_SUBRESOURCE_FOOTPRINT structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct PlacedSubresourceFootprint(
    pub(crate) D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
);

impl Default for PlacedSubresourceFootprint {
    fn default() -> Self {
        Self(D3D12_PLACED_SUBRESOURCE_FOOTPRINT {
            Offset: 0,
            Footprint: SubresourceFootprint::default().0,
        })
    }
}

impl PlacedSubresourceFootprint {
    pub fn set_offset(&mut self, offset: ByteCount) -> &mut Self {
        self.0.Offset = offset.0 as u64;
        self
    }

    pub fn with_offset(mut self, offset: ByteCount) -> Self {
        self.set_offset(offset);
        self
    }

    pub fn offset(&self) -> ByteCount {
        ByteCount::from(self.0.Offset)
    }

    pub fn set_footprint(
        &mut self,
        footprint: SubresourceFootprint,
    ) -> &mut Self {
        self.0.Footprint = footprint.0;
        self
    }

    pub fn with_footprint(mut self, footprint: SubresourceFootprint) -> Self {
        self.set_footprint(footprint);
        self
    }

    pub fn footprint(&self) -> SubresourceFootprint {
        SubresourceFootprint(self.0.Footprint)
    }
}

/// Wrapper around D3D12_CONSTANT_BUFFER_VIEW_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct ConstantBufferViewDesc(pub(crate) D3D12_CONSTANT_BUFFER_VIEW_DESC);

impl ConstantBufferViewDesc {
    pub fn set_buffer_location(
        &mut self,
        buffer_location: GpuVirtualAddress,
    ) -> &mut Self {
        self.0.BufferLocation = buffer_location.0;
        self
    }

    pub fn with_buffer_location(
        mut self,
        buffer_location: GpuVirtualAddress,
    ) -> Self {
        self.set_buffer_location(buffer_location);
        self
    }

    pub fn buffer_location(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(self.0.BufferLocation)
    }

    pub fn set_size_in_bytes(&mut self, size_in_bytes: ByteCount) -> &mut Self {
        self.0.SizeInBytes = size_in_bytes.0 as u32;
        self
    }

    pub fn with_size_in_bytes(mut self, size_in_bytes: ByteCount) -> Self {
        self.set_size_in_bytes(size_in_bytes);
        self
    }

    pub fn size_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.SizeInBytes)
    }
}

// ToDo: rethink the 'pub's in such wrappers
/// Wrapper around D3D12_DESCRIPTOR_HEAP_DESC structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
pub struct DescriptorHeapDesc(pub(crate) D3D12_DESCRIPTOR_HEAP_DESC);

impl Default for DescriptorHeapDesc {
    fn default() -> Self {
        Self(D3D12_DESCRIPTOR_HEAP_DESC {
            Type: DescriptorHeapType::CbvSrvUav as i32,
            NumDescriptors: 0,
            Flags: DescriptorHeapFlags::None.bits(),
            NodeMask: 0,
        })
    }
}

impl DescriptorHeapDesc {
    pub fn set_heap_type(
        &mut self,
        heap_type: DescriptorHeapType,
    ) -> &mut Self {
        self.0.Type = heap_type as i32;
        self
    }

    pub fn with_heap_type(mut self, heap_type: DescriptorHeapType) -> Self {
        self.set_heap_type(heap_type);
        self
    }

    pub fn heap_type(&self) -> DescriptorHeapType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_num_descriptors(&mut self, num_descriptors: u32) -> &mut Self {
        self.0.NumDescriptors = num_descriptors;
        self
    }

    pub fn with_num_descriptors(mut self, num_descriptors: u32) -> Self {
        self.set_num_descriptors(num_descriptors);
        self
    }

    pub fn num_descriptors(&self) -> u32 {
        self.0.NumDescriptors
    }

    pub fn set_flags(&mut self, flags: DescriptorHeapFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: DescriptorHeapFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> DescriptorHeapFlags {
        unsafe { DescriptorHeapFlags::from_bits_unchecked(self.0.Flags) }
    }

    pub fn set_node_mask(&mut self, node_mask: u32) -> &mut Self {
        self.0.NodeMask = node_mask;
        self
    }

    pub fn with_node_mask(mut self, node_mask: u32) -> Self {
        self.set_node_mask(node_mask);
        self
    }

    pub fn node_mask(&self) -> u32 {
        self.0.NodeMask
    }
}

/// Wrapper around D3D12_COMMAND_QUEUE_DESC structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct CommandQueueDesc(pub(crate) D3D12_COMMAND_QUEUE_DESC);

impl CommandQueueDesc {
    pub fn set_queue_type(&mut self, queue_type: CommandListType) -> &mut Self {
        self.0.Type = queue_type as i32;
        self
    }

    pub fn with_queue_type(mut self, queue_type: CommandListType) -> Self {
        self.set_queue_type(queue_type);
        self
    }

    pub fn queue_type(&self) -> CommandListType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_priority(&mut self, priority: i32) -> &mut Self {
        self.0.Priority = priority;
        self
    }

    pub fn with_priority(mut self, priority: i32) -> Self {
        self.set_priority(priority);
        self
    }

    pub fn priority(&self) -> i32 {
        self.0.Priority
    }

    pub fn set_flags(&mut self, flags: CommandQueueFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: CommandQueueFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> CommandQueueFlags {
        unsafe { CommandQueueFlags::from_bits_unchecked(self.0.Flags) }
    }

    pub fn set_node_mask(&mut self, node_mask: u32) -> &mut Self {
        self.0.NodeMask = node_mask;
        self
    }

    pub fn with_node_mask(mut self, node_mask: u32) -> Self {
        self.set_node_mask(node_mask);
        self
    }

    pub fn node_mask(&self) -> u32 {
        self.0.NodeMask
    }
}

/// Wrapper around D3D12_FEATURE_DATA_ROOT_SIGNATURE structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
pub struct FeatureDataRootSignature(
    pub(crate) D3D12_FEATURE_DATA_ROOT_SIGNATURE,
);

impl FeatureDataRootSignature {
    pub fn new(version: RootSignatureVersion) -> Self {
        Self(D3D12_FEATURE_DATA_ROOT_SIGNATURE {
            HighestVersion: version as i32,
        })
    }

    pub fn set_highest_version(
        &mut self,
        highest_version: RootSignatureVersion,
    ) -> &mut Self {
        self.0.HighestVersion = highest_version as i32;
        self
    }

    pub fn with_highest_version(
        mut self,
        highest_version: RootSignatureVersion,
    ) -> Self {
        self.set_highest_version(highest_version);
        self
    }

    pub fn highest_version(&self) -> RootSignatureVersion {
        unsafe { std::mem::transmute(self.0.HighestVersion) }
    }
}

/// Newtype around [u32] since it has a special value of [DESCRIPTOR_RANGE_OFFSET_APPEND]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
pub struct DescriptorRangeOffset(pub(crate) u32);

impl From<u32> for DescriptorRangeOffset {
    fn from(count: u32) -> Self {
        Self(count)
    }
}

impl DescriptorRangeOffset {
    pub fn append() -> Self {
        Self(D3D12_DESCRIPTOR_RANGE_OFFSET_APPEND)
    }
}

/// Wrapper around D3D12_DESCRIPTOR_RANGE1 structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct DescriptorRange(pub(crate) D3D12_DESCRIPTOR_RANGE1);

impl DescriptorRange {
    pub fn set_range_type(
        &mut self,
        range_type: DescriptorRangeType,
    ) -> &mut Self {
        self.0.RangeType = range_type as i32;
        self
    }

    pub fn with_range_type(mut self, range_type: DescriptorRangeType) -> Self {
        self.set_range_type(range_type);
        self
    }

    pub fn range_type(&self) -> DescriptorRangeType {
        unsafe { std::mem::transmute(self.0.RangeType) }
    }

    pub fn set_num_descriptors(&mut self, num_descriptors: u32) -> &mut Self {
        self.0.NumDescriptors = num_descriptors;
        self
    }

    pub fn with_num_descriptors(mut self, num_descriptors: u32) -> Self {
        self.set_num_descriptors(num_descriptors);
        self
    }

    pub fn num_descriptors(&self) -> u32 {
        self.0.NumDescriptors
    }

    pub fn set_base_shader_register(
        &mut self,
        base_shader_register: u32,
    ) -> &mut Self {
        self.0.BaseShaderRegister = base_shader_register;
        self
    }

    pub fn with_base_shader_register(
        mut self,
        base_shader_register: u32,
    ) -> Self {
        self.set_base_shader_register(base_shader_register);
        self
    }

    pub fn base_shader_register(&self) -> u32 {
        self.0.BaseShaderRegister
    }

    pub fn set_register_space(&mut self, register_space: u32) -> &mut Self {
        self.0.RegisterSpace = register_space;
        self
    }

    pub fn with_register_space(mut self, register_space: u32) -> Self {
        self.set_register_space(register_space);
        self
    }

    pub fn register_space(&self) -> u32 {
        self.0.RegisterSpace
    }

    pub fn set_flags(&mut self, flags: DescriptorRangeFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: DescriptorRangeFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> DescriptorRangeFlags {
        unsafe { DescriptorRangeFlags::from_bits_unchecked(self.0.Flags) }
    }

    pub fn set_offset_in_descriptors_from_table_start(
        &mut self,
        offset_in_descriptors_from_table_start: DescriptorRangeOffset,
    ) -> &mut Self {
        self.0.OffsetInDescriptorsFromTableStart =
            offset_in_descriptors_from_table_start.0;
        self
    }

    pub fn with_offset_in_descriptors_from_table_start(
        mut self,
        offset_in_descriptors_from_table_start: DescriptorRangeOffset,
    ) -> Self {
        self.set_offset_in_descriptors_from_table_start(
            offset_in_descriptors_from_table_start,
        );
        self
    }

    pub fn offset_in_descriptors_from_table_start(
        &self,
    ) -> DescriptorRangeOffset {
        self.0.OffsetInDescriptorsFromTableStart.into()
    }
}

/// Wrapper around D3D12_ROOT_PARAMETER1 structure
#[derive(Debug, Default)]
#[repr(transparent)]
pub struct RootParameter<'a>(
    pub(crate) D3D12_ROOT_PARAMETER1,
    PhantomData<&'a RootDescriptorTable<'a>>,
);

impl<'a> RootParameter<'a> {
    pub fn parameter_type(&self) -> RootParameterType {
        unsafe { std::mem::transmute(self.0.ParameterType) }
    }

    pub fn new_descriptor_table(
        mut self,
        descriptor_table: &'a RootDescriptorTable<'a>,
    ) -> Self {
        self.0.ParameterType = RootParameterType::DescriptorTable as i32;
        self.0.__bindgen_anon_1.DescriptorTable = descriptor_table.0;
        self.1 = PhantomData;
        self
    }

    pub fn descriptor_table(&self) -> Option<RootDescriptorTable> {
        unsafe {
            match self.parameter_type() {
                RootParameterType::DescriptorTable => {
                    Some(RootDescriptorTable(
                        self.0.__bindgen_anon_1.DescriptorTable,
                        PhantomData,
                    ))
                }
                _ => None,
            }
        }
    }

    pub fn new_constants(mut self, constants: &RootConstants) -> Self {
        self.0.ParameterType = RootParameterType::T32BitConstants as i32;
        self.0.__bindgen_anon_1.Constants = constants.0;
        self
    }

    pub fn constants(&self) -> Option<RootConstants> {
        unsafe {
            match self.parameter_type() {
                RootParameterType::T32BitConstants => {
                    Some(RootConstants(self.0.__bindgen_anon_1.Constants))
                }
                _ => None,
            }
        }
    }

    pub fn new_descriptor(
        mut self,
        descriptor: &RootDescriptor,
        descriptor_type: RootParameterType,
    ) -> Self {
        assert!(
            descriptor_type == RootParameterType::Cbv
                || descriptor_type == RootParameterType::Srv
                || descriptor_type == RootParameterType::Uav
        );
        self.0.ParameterType = descriptor_type as i32;
        self.0.__bindgen_anon_1.Descriptor = descriptor.0;
        self
    }

    pub fn descriptor(&self) -> Option<RootDescriptor> {
        unsafe {
            match self.parameter_type() {
                RootParameterType::Cbv
                | RootParameterType::Srv
                | RootParameterType::Uav => {
                    Some(RootDescriptor(self.0.__bindgen_anon_1.Descriptor))
                }
                _ => None,
            }
        }
    }

    pub fn set_shader_visibility(
        &mut self,
        shader_visibility: ShaderVisibility,
    ) -> &mut Self {
        self.0.ShaderVisibility = shader_visibility as i32;
        self
    }

    pub fn with_shader_visibility(
        mut self,
        shader_visibility: ShaderVisibility,
    ) -> Self {
        self.set_shader_visibility(shader_visibility);
        self
    }

    pub fn shader_visibility(&self) -> ShaderVisibility {
        unsafe { std::mem::transmute(self.0.ShaderVisibility) }
    }
}

/// Wrapper around D3D12_ROOT_DESCRIPTOR_TABLE1 structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct RootDescriptorTable<'a>(
    pub D3D12_ROOT_DESCRIPTOR_TABLE1,
    PhantomData<&'a DescriptorRange>,
);

impl<'a> RootDescriptorTable<'a> {
    pub fn set_descriptor_ranges(
        &mut self,
        ranges: &'a [DescriptorRange],
    ) -> &mut Self {
        self.0.NumDescriptorRanges = ranges.len() as u32;
        self.0.pDescriptorRanges =
            ranges.as_ptr() as *const D3D12_DESCRIPTOR_RANGE1;
        self.1 = PhantomData;
        self
    }

    pub fn with_descriptor_ranges(
        mut self,
        ranges: &'a [DescriptorRange],
    ) -> Self {
        self.set_descriptor_ranges(ranges);
        self
    }

    pub fn descriptor_ranges(&self) -> &'a [DescriptorRange] {
        unsafe {
            std::slice::from_raw_parts(
                self.0.pDescriptorRanges as *const D3D12_DESCRIPTOR_RANGE1
                    as *const DescriptorRange,
                self.0.NumDescriptorRanges as usize,
            )
        }
    }
}

/// Wrapper around D3D12_ROOT_CONSTANTS structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct RootConstants(pub(crate) D3D12_ROOT_CONSTANTS);

impl RootConstants {
    pub fn set_shader_register(&mut self, shader_register: u32) -> &mut Self {
        self.0.ShaderRegister = shader_register;
        self
    }

    pub fn with_shader_register(mut self, shader_register: u32) -> Self {
        self.set_shader_register(shader_register);
        self
    }

    pub fn shader_register(&self) -> u32 {
        self.0.ShaderRegister
    }

    pub fn set_register_space(&mut self, register_space: u32) -> &mut Self {
        self.0.RegisterSpace = register_space;
        self
    }

    pub fn with_register_space(mut self, register_space: u32) -> Self {
        self.set_register_space(register_space);
        self
    }

    pub fn register_space(&self) -> u32 {
        self.0.RegisterSpace
    }

    pub fn set_num_32_bit_values(
        &mut self,
        num_32_bit_values: u32,
    ) -> &mut Self {
        self.0.Num32BitValues = num_32_bit_values;
        self
    }

    pub fn with_num_32_bit_values(mut self, num_32_bit_values: u32) -> Self {
        self.set_num_32_bit_values(num_32_bit_values);
        self
    }

    pub fn num_32_bit_values(&self) -> u32 {
        self.0.Num32BitValues
    }
}

/// Wrapper around D3D12_ROOT_DESCRIPTOR1 structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct RootDescriptor(pub(crate) D3D12_ROOT_DESCRIPTOR1);

impl RootDescriptor {
    pub fn set_shader_register(&mut self, shader_register: u32) -> &mut Self {
        self.0.ShaderRegister = shader_register;
        self
    }

    pub fn with_shader_register(mut self, shader_register: u32) -> Self {
        self.set_shader_register(shader_register);
        self
    }

    pub fn shader_register(&self) -> u32 {
        self.0.ShaderRegister
    }

    pub fn set_register_space(&mut self, register_space: u32) -> &mut Self {
        self.0.RegisterSpace = register_space;
        self
    }

    pub fn with_register_space(mut self, register_space: u32) -> Self {
        self.set_register_space(register_space);
        self
    }

    pub fn register_space(&self) -> u32 {
        self.0.RegisterSpace
    }

    pub fn set_flags(&mut self, flags: RootDescriptorFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: RootDescriptorFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> RootDescriptorFlags {
        unsafe { RootDescriptorFlags::from_bits_unchecked(self.0.Flags) }
    }
}

/// Wrapper around D3D12_SAMPLER_DESC structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct SamplerDesc(pub(crate) D3D12_SAMPLER_DESC);

impl SamplerDesc {
    pub fn set_filter(&mut self, filter: Filter) -> &mut Self {
        self.0.Filter = filter as i32;
        self
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.set_filter(filter);
        self
    }

    pub fn filter(&self) -> Filter {
        unsafe { std::mem::transmute(self.0.Filter) }
    }

    pub fn set_address_u(
        &mut self,
        address_u: TextureAddressMode,
    ) -> &mut Self {
        self.0.AddressU = address_u as i32;
        self
    }

    pub fn with_address_u(mut self, address_u: TextureAddressMode) -> Self {
        self.set_address_u(address_u);
        self
    }

    pub fn address_u(&self) -> TextureAddressMode {
        unsafe { std::mem::transmute(self.0.AddressU) }
    }

    pub fn set_address_v(
        &mut self,
        address_v: TextureAddressMode,
    ) -> &mut Self {
        self.0.AddressV = address_v as i32;
        self
    }

    pub fn with_address_v(mut self, address_v: TextureAddressMode) -> Self {
        self.set_address_v(address_v);
        self
    }

    pub fn address_v(&self) -> TextureAddressMode {
        unsafe { std::mem::transmute(self.0.AddressV) }
    }

    pub fn set_address_w(
        &mut self,
        address_w: TextureAddressMode,
    ) -> &mut Self {
        self.0.AddressW = address_w as i32;
        self
    }

    pub fn with_address_w(mut self, address_w: TextureAddressMode) -> Self {
        self.set_address_w(address_w);
        self
    }

    pub fn address_w(&self) -> TextureAddressMode {
        unsafe { std::mem::transmute(self.0.AddressW) }
    }

    pub fn set_mip_lod_bias(&mut self, mip_lod_bias: f32) -> &mut Self {
        self.0.MipLODBias = mip_lod_bias;
        self
    }

    pub fn with_mip_lod_bias(mut self, mip_lod_bias: f32) -> Self {
        self.set_mip_lod_bias(mip_lod_bias);
        self
    }

    pub fn mip_lod_bias(&self) -> f32 {
        self.0.MipLODBias
    }

    pub fn set_max_anisotropy(&mut self, max_anisotropy: u32) -> &mut Self {
        self.0.MaxAnisotropy = max_anisotropy;
        self
    }

    pub fn with_max_anisotropy(mut self, max_anisotropy: u32) -> Self {
        self.set_max_anisotropy(max_anisotropy);
        self
    }

    pub fn max_anisotropy(&self) -> u32 {
        self.0.MaxAnisotropy
    }

    pub fn set_comparison_func(
        &mut self,
        comparison_func: ComparisonFunc,
    ) -> &mut Self {
        self.0.ComparisonFunc = comparison_func as i32;
        self
    }

    pub fn with_comparison_func(
        mut self,
        comparison_func: ComparisonFunc,
    ) -> Self {
        self.set_comparison_func(comparison_func);
        self
    }

    pub fn comparison_func(&self) -> ComparisonFunc {
        unsafe { std::mem::transmute(self.0.ComparisonFunc) }
    }

    pub fn set_border_color(
        &mut self,
        border_color: [f32; 4usize],
    ) -> &mut Self {
        self.0.BorderColor = border_color;
        self
    }

    pub fn with_border_color(mut self, border_color: [f32; 4usize]) -> Self {
        self.set_border_color(border_color);
        self
    }

    pub fn border_color(&self) -> [f32; 4usize] {
        self.0.BorderColor
    }

    pub fn set_min_lod(&mut self, min_lod: f32) -> &mut Self {
        self.0.MinLOD = min_lod;
        self
    }

    pub fn with_min_lod(mut self, min_lod: f32) -> Self {
        self.set_min_lod(min_lod);
        self
    }

    pub fn min_lod(&self) -> f32 {
        self.0.MinLOD
    }

    pub fn set_max_lod(&mut self, max_lod: f32) -> &mut Self {
        self.0.MaxLOD = max_lod;
        self
    }

    pub fn with_max_lod(mut self, max_lod: f32) -> Self {
        self.set_max_lod(max_lod);
        self
    }

    pub fn max_lod(&self) -> f32 {
        self.0.MaxLOD
    }
}

/// Wrapper around D3D12_STATIC_SAMPLER_DESC structure
#[repr(transparent)]
#[derive(Copy, Clone, Debug)]
pub struct StaticSamplerDesc(pub(crate) D3D12_STATIC_SAMPLER_DESC);

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
    pub fn set_filter(&mut self, filter: Filter) -> &mut Self {
        self.0.Filter = filter as i32;
        self
    }

    pub fn with_filter(mut self, filter: Filter) -> Self {
        self.set_filter(filter);
        self
    }

    pub fn filter(&self) -> Filter {
        unsafe { std::mem::transmute(self.0.Filter) }
    }

    pub fn set_address_u(
        &mut self,
        address_u: TextureAddressMode,
    ) -> &mut Self {
        self.0.AddressU = address_u as i32;
        self
    }

    pub fn with_address_u(mut self, address_u: TextureAddressMode) -> Self {
        self.set_address_u(address_u);
        self
    }

    pub fn address_u(&self) -> TextureAddressMode {
        unsafe { std::mem::transmute(self.0.AddressU) }
    }

    pub fn set_address_v(
        &mut self,
        address_v: TextureAddressMode,
    ) -> &mut Self {
        self.0.AddressV = address_v as i32;
        self
    }

    pub fn with_address_v(mut self, address_v: TextureAddressMode) -> Self {
        self.set_address_v(address_v);
        self
    }

    pub fn address_v(&self) -> TextureAddressMode {
        unsafe { std::mem::transmute(self.0.AddressV) }
    }

    pub fn set_address_w(
        &mut self,
        address_w: TextureAddressMode,
    ) -> &mut Self {
        self.0.AddressW = address_w as i32;
        self
    }

    pub fn with_address_w(mut self, address_w: TextureAddressMode) -> Self {
        self.set_address_w(address_w);
        self
    }

    pub fn address_w(&self) -> TextureAddressMode {
        unsafe { std::mem::transmute(self.0.AddressW) }
    }

    pub fn set_mip_lod_bias(&mut self, mip_lod_bias: f32) -> &mut Self {
        self.0.MipLODBias = mip_lod_bias;
        self
    }

    pub fn with_mip_lod_bias(mut self, mip_lod_bias: f32) -> Self {
        self.set_mip_lod_bias(mip_lod_bias);
        self
    }

    pub fn mip_lod_bias(&self) -> f32 {
        self.0.MipLODBias
    }

    pub fn set_max_anisotropy(&mut self, max_anisotropy: u32) -> &mut Self {
        self.0.MaxAnisotropy = max_anisotropy;
        self
    }

    pub fn with_max_anisotropy(mut self, max_anisotropy: u32) -> Self {
        self.set_max_anisotropy(max_anisotropy);
        self
    }

    pub fn max_anisotropy(&self) -> u32 {
        self.0.MaxAnisotropy
    }

    pub fn set_comparison_func(
        &mut self,
        comparison_func: ComparisonFunc,
    ) -> &mut Self {
        self.0.ComparisonFunc = comparison_func as i32;
        self
    }

    pub fn with_comparison_func(
        mut self,
        comparison_func: ComparisonFunc,
    ) -> Self {
        self.set_comparison_func(comparison_func);
        self
    }

    pub fn comparison_func(&self) -> ComparisonFunc {
        unsafe { std::mem::transmute(self.0.ComparisonFunc) }
    }

    pub fn set_border_color(
        &mut self,
        border_color: StaticBorderColor,
    ) -> &mut Self {
        self.0.BorderColor = border_color as i32;
        self
    }

    pub fn with_border_color(
        mut self,
        border_color: StaticBorderColor,
    ) -> Self {
        self.set_border_color(border_color);
        self
    }

    pub fn border_color(&self) -> StaticBorderColor {
        unsafe { std::mem::transmute(self.0.BorderColor) }
    }

    pub fn set_min_lod(&mut self, min_lod: f32) -> &mut Self {
        self.0.MinLOD = min_lod;
        self
    }

    pub fn with_min_lod(mut self, min_lod: f32) -> Self {
        self.set_min_lod(min_lod);
        self
    }

    pub fn min_lod(&self) -> f32 {
        self.0.MinLOD
    }

    pub fn set_max_lod(&mut self, max_lod: f32) -> &mut Self {
        self.0.MaxLOD = max_lod;
        self
    }

    pub fn with_max_lod(mut self, max_lod: f32) -> Self {
        self.set_max_lod(max_lod);
        self
    }

    pub fn max_lod(&self) -> f32 {
        self.0.MaxLOD
    }

    pub fn set_shader_register(&mut self, shader_register: u32) -> &mut Self {
        self.0.ShaderRegister = shader_register;
        self
    }

    pub fn with_shader_register(mut self, shader_register: u32) -> Self {
        self.set_shader_register(shader_register);
        self
    }

    pub fn shader_register(&self) -> u32 {
        self.0.ShaderRegister
    }

    pub fn set_register_space(&mut self, register_space: u32) -> &mut Self {
        self.0.RegisterSpace = register_space;
        self
    }

    pub fn with_register_space(mut self, register_space: u32) -> Self {
        self.set_register_space(register_space);
        self
    }

    pub fn register_space(&self) -> u32 {
        self.0.RegisterSpace
    }

    pub fn set_shader_visibility(
        &mut self,
        shader_visibility: ShaderVisibility,
    ) -> &mut Self {
        self.0.ShaderVisibility = shader_visibility as i32;
        self
    }

    pub fn with_shader_visibility(
        mut self,
        shader_visibility: ShaderVisibility,
    ) -> Self {
        self.set_shader_visibility(shader_visibility);
        self
    }

    pub fn shader_visibility(&self) -> ShaderVisibility {
        unsafe { std::mem::transmute(self.0.ShaderVisibility) }
    }
}

/// Wrapper around D3D12_VERSIONED_ROOT_SIGNATURE_DESC structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct VersionedRootSignatureDesc(
    pub(crate) D3D12_VERSIONED_ROOT_SIGNATURE_DESC,
);

impl VersionedRootSignatureDesc {
    // RS v1.0 is not supported
    // pub fn set_desc_1_0(self, _desc_1_0: &RootSignatureDesc) -> Self {
    //     unimplemented!();
    // }

    pub fn set_desc_1_1(&mut self, desc_1_1: &RootSignatureDesc) -> &mut Self {
        self.0.Version =
            D3D_ROOT_SIGNATURE_VERSION_D3D_ROOT_SIGNATURE_VERSION_1_1;
        self.0.__bindgen_anon_1.Desc_1_1 = desc_1_1.0;
        self
    }

    pub fn with_desc_1_1(mut self, desc_1_1: &RootSignatureDesc) -> Self {
        self.set_desc_1_1(desc_1_1);
        self
    }

    pub fn desc_1_1(&self) -> RootSignatureDesc {
        unsafe {
            RootSignatureDesc(
                self.0.__bindgen_anon_1.Desc_1_1,
                PhantomData,
                PhantomData,
            )
        }
    }
}

/// Wrapper around D3D12_ROOT_SIGNATURE_DESC1 structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct RootSignatureDesc<'a, 'b>(
    pub D3D12_ROOT_SIGNATURE_DESC1,
    PhantomData<&'a RootParameter<'a>>,
    PhantomData<&'b StaticSamplerDesc>,
);

impl<'a, 'b> RootSignatureDesc<'a, 'b> {
    pub fn set_parameters(
        &mut self,
        parameters: &'a [RootParameter],
    ) -> &mut Self {
        self.0.NumParameters = parameters.len() as u32;
        self.0.pParameters =
            parameters.as_ptr() as *const D3D12_ROOT_PARAMETER1;
        self.1 = PhantomData;
        self
    }

    pub fn with_parameters(mut self, parameters: &'a [RootParameter]) -> Self {
        self.set_parameters(parameters);
        self
    }

    pub fn parameters(&self) -> &'a [RootParameter] {
        unsafe {
            slice::from_raw_parts(
                self.0.pParameters as *const D3D12_ROOT_PARAMETER1
                    as *const RootParameter,
                self.0.NumParameters as usize,
            )
        }
    }

    pub fn set_static_samplers(
        &mut self,
        static_samplers: &'b [StaticSamplerDesc],
    ) -> &mut Self {
        self.0.NumStaticSamplers = static_samplers.len() as u32;
        self.0.pStaticSamplers =
            static_samplers.as_ptr() as *const D3D12_STATIC_SAMPLER_DESC;
        self.2 = PhantomData;
        self
    }

    pub fn with_static_samplers(
        mut self,
        static_samplers: &'b [StaticSamplerDesc],
    ) -> Self {
        self.set_static_samplers(static_samplers);
        self
    }

    pub fn static_samplers(&self) -> &'a [StaticSamplerDesc] {
        unsafe {
            slice::from_raw_parts(
                self.0.pStaticSamplers as *const D3D12_STATIC_SAMPLER_DESC
                    as *const StaticSamplerDesc,
                self.0.NumStaticSamplers as usize,
            )
        }
    }

    pub fn set_flags(&mut self, flags: RootSignatureFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: RootSignatureFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> RootSignatureFlags {
        unsafe { RootSignatureFlags::from_bits_unchecked(self.0.Flags) }
    }
}

/// Wrapper around D3D12_SUBRESOURCE_DATA structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct SubresourceData<'a>(
    pub D3D12_SUBRESOURCE_DATA,
    PhantomData<&'a [()]>,
);

impl<'a> SubresourceData<'a> {
    pub fn set_data<T>(&mut self, data: &'a [T]) -> &mut Self {
        self.0.pData = data.as_ptr() as *const std::ffi::c_void;
        self.1 = PhantomData;
        self
    }

    pub fn with_data<T>(mut self, data: &'a [T]) -> Self {
        self.set_data(data);
        self
    }

    // ToDo?
    // pub fn data<T>(&self) -> &'a [T] {
    //     unsafe {
    //         slice::from_raw_parts(
    //             self.0.pData as *const T,
    //             self.0.SizeInBytes as usize,
    //         )
    //     }
    // }

    pub fn set_row_pitch(&mut self, row_pitch: ByteCount) -> &mut Self {
        self.0.RowPitch = row_pitch.0 as i64;
        self
    }

    pub fn with_row_pitch(mut self, row_pitch: ByteCount) -> Self {
        self.set_row_pitch(row_pitch);
        self
    }

    pub fn row_pitch(&self) -> ByteCount {
        ByteCount::from(self.0.RowPitch)
    }

    pub fn set_slice_pitch(&mut self, slice_pitch: ByteCount) -> &mut Self {
        self.0.SlicePitch = slice_pitch.0 as i64;
        self
    }

    pub fn with_slice_pitch(mut self, slice_pitch: ByteCount) -> Self {
        self.set_slice_pitch(slice_pitch);
        self
    }

    pub fn slice_pitch(&self) -> ByteCount {
        ByteCount::from(self.0.SlicePitch)
    }
}

/// Wrapper around D3D12_SHADER_RESOURCE_VIEW_DESC structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct ShaderResourceViewDesc(pub(crate) D3D12_SHADER_RESOURCE_VIEW_DESC);

impl ShaderResourceViewDesc {
    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn view_dimension(&self) -> SrvDimension {
        unsafe { std::mem::transmute(self.0.ViewDimension) }
    }

    pub fn set_shader_4_component_mapping(
        &mut self,
        shader4_component_mapping: ShaderComponentMapping,
    ) -> &mut Self {
        self.0.Shader4ComponentMapping = shader4_component_mapping.into();
        self
    }

    pub fn with_shader_4_component_mapping(
        mut self,
        shader4_component_mapping: ShaderComponentMapping,
    ) -> Self {
        self.set_shader_4_component_mapping(shader4_component_mapping);
        self
    }

    pub fn shader_4_component_mapping(&self) -> ShaderComponentMapping {
        self.0.Shader4ComponentMapping.into()
    }

    // ToDo: rename these new* since at the call site they look
    // like a regular setter. Another option is to remove Default derive
    pub fn new_buffer(mut self, buffer: &BufferSrv) -> Self {
        self.0.ViewDimension = SrvDimension::Buffer as i32;
        self.0.__bindgen_anon_1.Buffer = buffer.0;
        self
    }

    pub fn buffer(&self) -> Option<BufferSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Buffer => {
                    Some(BufferSrv(self.0.__bindgen_anon_1.Buffer))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_1d(mut self, texture_1d: &Tex1DSrv) -> Self {
        self.0.ViewDimension = SrvDimension::Texture1D as i32;
        self.0.__bindgen_anon_1.Texture1D = texture_1d.0;
        self
    }

    pub fn texture_1d(&self) -> Option<Tex1DSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture1D => {
                    Some(Tex1DSrv(self.0.__bindgen_anon_1.Texture1D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_1d_array(
        mut self,
        texture_1d_array: &Tex1DArraySrv,
    ) -> Self {
        self.0.ViewDimension = SrvDimension::Texture1DArray as i32;
        self.0.__bindgen_anon_1.Texture1DArray = texture_1d_array.0;
        self
    }

    pub fn texture_1d_array(&self) -> Option<Tex1DArraySrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture1DArray => {
                    Some(Tex1DArraySrv(self.0.__bindgen_anon_1.Texture1DArray))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d(mut self, texture_2d: &Tex2DSrv) -> Self {
        self.0.ViewDimension = SrvDimension::Texture2D as i32;
        self.0.__bindgen_anon_1.Texture2D = texture_2d.0;
        self
    }

    pub fn texture_2d(&self) -> Option<Tex2DSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture2D => {
                    Some(Tex2DSrv(self.0.__bindgen_anon_1.Texture2D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_array(
        mut self,
        texture_2d_array: &Tex2DArraySrv,
    ) -> Self {
        self.0.ViewDimension = SrvDimension::Texture2DArray as i32;
        self.0.__bindgen_anon_1.Texture2DArray = texture_2d_array.0;
        self
    }

    pub fn texture_2d_array(&self) -> Option<Tex2DArraySrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture2DArray => {
                    Some(Tex2DArraySrv(self.0.__bindgen_anon_1.Texture2DArray))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_ms(mut self, texture_2d_ms: &Tex2DMsSrv) -> Self {
        self.0.ViewDimension = SrvDimension::Texture2DMs as i32;
        self.0.__bindgen_anon_1.Texture2DMS = texture_2d_ms.0;
        self
    }

    pub fn texture_2d_ms(&self) -> Option<Tex2DMsSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture2DMs => {
                    Some(Tex2DMsSrv(self.0.__bindgen_anon_1.Texture2DMS))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_ms_array(
        mut self,
        texture_2d_ms_array: &Tex2DMsArraySrv,
    ) -> Self {
        self.0.ViewDimension = SrvDimension::Texture2DMsArray as i32;
        self.0.__bindgen_anon_1.Texture2DMSArray = texture_2d_ms_array.0;
        self
    }

    pub fn texture_2d_ms_array(&self) -> Option<Tex2DMsArraySrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture2DMsArray => Some(Tex2DMsArraySrv(
                    self.0.__bindgen_anon_1.Texture2DMSArray,
                )),
                _ => None,
            }
        }
    }

    pub fn new_texture_3d(mut self, texture_3d: &Tex3DSrv) -> Self {
        self.0.ViewDimension = SrvDimension::Texture3D as i32;
        self.0.__bindgen_anon_1.Texture3D = texture_3d.0;
        self
    }

    pub fn texture_3d(&self) -> Option<Tex3DSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::Texture3D => {
                    Some(Tex3DSrv(self.0.__bindgen_anon_1.Texture3D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_cube(mut self, texture_cube: &TexcubeSrv) -> Self {
        self.0.ViewDimension = SrvDimension::TextureCube as i32;
        self.0.__bindgen_anon_1.TextureCube = texture_cube.0;
        self
    }

    pub fn texture_cube(&self) -> Option<TexcubeSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::TextureCube => {
                    Some(TexcubeSrv(self.0.__bindgen_anon_1.TextureCube))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_cube_array(
        mut self,
        texture_cube_array: &TexcubeArraySrv,
    ) -> Self {
        self.0.ViewDimension = SrvDimension::TextureCubeArray as i32;
        self.0.__bindgen_anon_1.TextureCubeArray = texture_cube_array.0;
        self
    }

    pub fn texture_cube_array(&self) -> Option<TexcubeArraySrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::TextureCubeArray => Some(TexcubeArraySrv(
                    self.0.__bindgen_anon_1.TextureCubeArray,
                )),
                _ => None,
            }
        }
    }

    pub fn new_raytracing_acceleration_structure(
        mut self,
        raytracing_acceleration_structure: &RaytracingAccelerationStructureSrv,
    ) -> Self {
        self.0.ViewDimension =
            SrvDimension::RaytracingAccelerationStructure as i32;
        self.0.__bindgen_anon_1.RaytracingAccelerationStructure =
            raytracing_acceleration_structure.0;
        self
    }

    pub fn raytracing_acceleration_structure(
        &self,
    ) -> Option<RaytracingAccelerationStructureSrv> {
        unsafe {
            match self.view_dimension() {
                SrvDimension::RaytracingAccelerationStructure => {
                    Some(RaytracingAccelerationStructureSrv(
                        self.0.__bindgen_anon_1.RaytracingAccelerationStructure,
                    ))
                }
                _ => None,
            }
        }
    }
}

/// Wrapper around D3D12_BUFFER_SRV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct BufferSrv(pub(crate) D3D12_BUFFER_SRV);

impl BufferSrv {
    pub fn set_first_element(&mut self, first_element: u64) -> &mut Self {
        self.0.FirstElement = first_element;
        self
    }

    pub fn with_first_element(mut self, first_element: u64) -> Self {
        self.set_first_element(first_element);
        self
    }

    pub fn first_element(&self) -> u64 {
        self.0.FirstElement
    }

    pub fn set_num_elements(&mut self, num_elements: u32) -> &mut Self {
        self.0.NumElements = num_elements;
        self
    }

    pub fn with_num_elements(mut self, num_elements: u32) -> Self {
        self.set_num_elements(num_elements);
        self
    }

    pub fn num_elements(&self) -> u32 {
        self.0.NumElements
    }

    pub fn set_structure_byte_stride(
        &mut self,
        structure_byte_stride: ByteCount,
    ) -> &mut Self {
        self.0.StructureByteStride = structure_byte_stride.0 as u32;
        self
    }

    pub fn with_structure_byte_stride(
        mut self,
        structure_byte_stride: ByteCount,
    ) -> Self {
        self.set_structure_byte_stride(structure_byte_stride);
        self
    }

    pub fn structure_byte_stride(&self) -> ByteCount {
        ByteCount::from(self.0.StructureByteStride)
    }

    pub fn set_flags(&mut self, flags: BufferSrvFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: BufferSrvFlags) -> Self {
        self.set_flags(flags);
        self
    }

    // ToDo: truncate instead of unchecked?
    pub fn flags(&self) -> BufferSrvFlags {
        unsafe { BufferSrvFlags::from_bits_unchecked(self.0.Flags) }
    }
}

/// Wrapper around D3D12_TEX1D_SRV structure
#[derive(Default, Debug, PartialOrd, PartialEq, Clone)]
#[repr(transparent)]
pub struct Tex1DSrv(pub(crate) D3D12_TEX1D_SRV);

impl Tex1DSrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_TEX1D_ARRAY_SRV structure
#[derive(Default, Debug, PartialOrd, PartialEq, Clone)]
#[repr(transparent)]
pub struct Tex1DArraySrv(pub(crate) D3D12_TEX1D_ARRAY_SRV);

impl Tex1DArraySrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_TEX2D_SRV structure
#[derive(Default, Debug, PartialOrd, PartialEq, Clone)]
#[repr(transparent)]
pub struct Tex2DSrv(pub(crate) D3D12_TEX2D_SRV);

impl Tex2DSrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_plane_slice(&mut self, plane_slice: u32) -> &mut Self {
        self.0.PlaneSlice = plane_slice;
        self
    }

    pub fn with_plane_slice(mut self, plane_slice: u32) -> Self {
        self.set_plane_slice(plane_slice);
        self
    }

    pub fn plane_slice(&self) -> u32 {
        self.0.PlaneSlice
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_TEX2D_ARRAY_SRV structure
#[derive(Default, Debug, PartialOrd, PartialEq, Clone)]
#[repr(transparent)]
pub struct Tex2DArraySrv(pub(crate) D3D12_TEX2D_ARRAY_SRV);

impl Tex2DArraySrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }

    pub fn set_plane_slice(&mut self, plane_slice: u32) -> &mut Self {
        self.0.PlaneSlice = plane_slice;
        self
    }

    pub fn with_plane_slice(mut self, plane_slice: u32) -> Self {
        self.set_plane_slice(plane_slice);
        self
    }

    pub fn plane_slice(&self) -> u32 {
        self.0.PlaneSlice
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_TEX2DMS_SRV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex2DMsSrv(pub(crate) D3D12_TEX2DMS_SRV);

/// Wrapper around D3D12_TEX2DMS_ARRAY_SRV structure
#[derive(Default, Debug, Hash, PartialOrd, Ord, PartialEq, Eq, Clone)]
#[repr(transparent)]
pub struct Tex2DMsArraySrv(pub(crate) D3D12_TEX2DMS_ARRAY_SRV);

impl Tex2DMsArraySrv {
    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }
}

/// Wrapper around D3D12_TEX3D_SRV structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex3DSrv(pub(crate) D3D12_TEX3D_SRV);

impl Tex3DSrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_TEXCUBE_SRV structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct TexcubeSrv(pub(crate) D3D12_TEXCUBE_SRV);

impl TexcubeSrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_TEXCUBE_ARRAY_SRV structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct TexcubeArraySrv(pub(crate) D3D12_TEXCUBE_ARRAY_SRV);

impl TexcubeArraySrv {
    pub fn set_most_detailed_mip(
        &mut self,
        most_detailed_mip: u32,
    ) -> &mut Self {
        self.0.MostDetailedMip = most_detailed_mip;
        self
    }

    pub fn with_most_detailed_mip(mut self, most_detailed_mip: u32) -> Self {
        self.set_most_detailed_mip(most_detailed_mip);
        self
    }

    pub fn most_detailed_mip(&self) -> u32 {
        self.0.MostDetailedMip
    }

    pub fn set_mip_levels(&mut self, mip_levels: u32) -> &mut Self {
        self.0.MipLevels = mip_levels;
        self
    }

    pub fn with_mip_levels(mut self, mip_levels: u32) -> Self {
        self.set_mip_levels(mip_levels);
        self
    }

    pub fn mip_levels(&self) -> u32 {
        self.0.MipLevels
    }

    pub fn set_first_2d_array_face(
        &mut self,
        first_2d_array_face: u32,
    ) -> &mut Self {
        self.0.First2DArrayFace = first_2d_array_face;
        self
    }

    pub fn with_first_2d_array_face(
        mut self,
        first_2d_array_face: u32,
    ) -> Self {
        self.set_first_2d_array_face(first_2d_array_face);
        self
    }

    pub fn first_2d_array_face(&self) -> u32 {
        self.0.First2DArrayFace
    }

    pub fn set_num_cubes(&mut self, num_cubes: u32) -> &mut Self {
        self.0.NumCubes = num_cubes;
        self
    }

    pub fn with_num_cubes(mut self, num_cubes: u32) -> Self {
        self.set_num_cubes(num_cubes);
        self
    }

    pub fn num_cubes(&self) -> u32 {
        self.0.NumCubes
    }

    pub fn set_resource_min_lod_clamp(
        &mut self,
        resource_min_lod_clamp: f32,
    ) -> &mut Self {
        self.0.ResourceMinLODClamp = resource_min_lod_clamp;
        self
    }

    pub fn with_resource_min_lod_clamp(
        mut self,
        resource_min_lod_clamp: f32,
    ) -> Self {
        self.set_resource_min_lod_clamp(resource_min_lod_clamp);
        self
    }

    pub fn resource_min_lod_clamp(&self) -> f32 {
        self.0.ResourceMinLODClamp
    }
}

/// Wrapper around D3D12_RAYTRACING_ACCELERATION_STRUCTURE_SRV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct RaytracingAccelerationStructureSrv(
    pub D3D12_RAYTRACING_ACCELERATION_STRUCTURE_SRV,
);

impl RaytracingAccelerationStructureSrv {
    pub fn set_location(&mut self, location: GpuVirtualAddress) -> &mut Self {
        self.0.Location = location.0;
        self
    }

    pub fn with_location(mut self, location: GpuVirtualAddress) -> Self {
        self.set_location(location);
        self
    }

    pub fn location(&self) -> GpuVirtualAddress {
        GpuVirtualAddress(self.0.Location)
    }
}

/// Wrapper around D3D12_UNORDERED_ACCESS_VIEW_DESC structure
#[repr(transparent)]
#[derive(Copy, Clone, Default, Debug)]
pub struct UnorderedAccessViewDesc(pub(crate) D3D12_UNORDERED_ACCESS_VIEW_DESC);

impl UnorderedAccessViewDesc {
    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn view_dimension(&self) -> UavDimension {
        unsafe { std::mem::transmute(self.0.ViewDimension) }
    }

    // ToDo: rename these new* since at the call site they look
    // like a regular setter. Another option is to remove Default derive
    pub fn new_buffer(mut self, buffer: &BufferUav) -> Self {
        self.0.ViewDimension = UavDimension::Buffer as i32;
        self.0.__bindgen_anon_1.Buffer = buffer.0;
        self
    }

    pub fn buffer(&self) -> Option<BufferUav> {
        unsafe {
            match self.view_dimension() {
                UavDimension::Buffer => {
                    Some(BufferUav(self.0.__bindgen_anon_1.Buffer))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_1d(mut self, texture_1d: &Tex1DUav) -> Self {
        self.0.ViewDimension = UavDimension::Texture1D as i32;
        self.0.__bindgen_anon_1.Texture1D = texture_1d.0;
        self
    }

    pub fn texture_1d(&self) -> Option<Tex1DUav> {
        unsafe {
            match self.view_dimension() {
                UavDimension::Texture1D => {
                    Some(Tex1DUav(self.0.__bindgen_anon_1.Texture1D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_1d_array(
        mut self,
        texture_1d_array: &Tex1DArrayUav,
    ) -> Self {
        self.0.ViewDimension = UavDimension::Texture1DArray as i32;
        self.0.__bindgen_anon_1.Texture1DArray = texture_1d_array.0;
        self
    }

    pub fn texture_1d_array(&self) -> Option<Tex1DArrayUav> {
        unsafe {
            match self.view_dimension() {
                UavDimension::Texture1DArray => {
                    Some(Tex1DArrayUav(self.0.__bindgen_anon_1.Texture1DArray))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d(mut self, texture_2d: &Tex2DUav) -> Self {
        self.0.ViewDimension = UavDimension::Texture2D as i32;
        self.0.__bindgen_anon_1.Texture2D = texture_2d.0;
        self
    }

    pub fn texture_2d(&self) -> Option<Tex2DUav> {
        unsafe {
            match self.view_dimension() {
                UavDimension::Texture2D => {
                    Some(Tex2DUav(self.0.__bindgen_anon_1.Texture2D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_array(
        mut self,
        texture_2d_array: &Tex2DArrayUav,
    ) -> Self {
        self.0.ViewDimension = UavDimension::Texture2DArray as i32;
        self.0.__bindgen_anon_1.Texture2DArray = texture_2d_array.0;
        self
    }

    pub fn texture_2d_array(&self) -> Option<Tex2DArrayUav> {
        unsafe {
            match self.view_dimension() {
                UavDimension::Texture2DArray => {
                    Some(Tex2DArrayUav(self.0.__bindgen_anon_1.Texture2DArray))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_3d(mut self, texture_3d: &Tex3DUav) -> Self {
        self.0.ViewDimension = UavDimension::Texture3D as i32;
        self.0.__bindgen_anon_1.Texture3D = texture_3d.0;
        self
    }

    pub fn texture_3d(&self) -> Option<Tex3DUav> {
        unsafe {
            match self.view_dimension() {
                UavDimension::Texture3D => {
                    Some(Tex3DUav(self.0.__bindgen_anon_1.Texture3D))
                }
                _ => None,
            }
        }
    }
}

/// Wrapper around D3D12_BUFFER_UAV structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
pub struct BufferUav(pub(crate) D3D12_BUFFER_UAV);

impl BufferUav {
    pub fn set_first_element(&mut self, first_element: u64) -> &mut Self {
        self.0.FirstElement = first_element;
        self
    }

    pub fn with_first_element(mut self, first_element: u64) -> Self {
        self.set_first_element(first_element);
        self
    }

    pub fn first_element(&self) -> u64 {
        self.0.FirstElement
    }

    pub fn set_num_elements(&mut self, num_elements: u32) -> &mut Self {
        self.0.NumElements = num_elements;
        self
    }

    pub fn with_num_elements(mut self, num_elements: u32) -> Self {
        self.set_num_elements(num_elements);
        self
    }

    pub fn num_elements(&self) -> u32 {
        self.0.NumElements
    }

    pub fn set_structure_byte_stride(
        &mut self,
        structure_byte_stride: ByteCount,
    ) -> &mut Self {
        self.0.StructureByteStride = structure_byte_stride.0 as u32;
        self
    }

    pub fn with_structure_byte_stride(
        mut self,
        structure_byte_stride: ByteCount,
    ) -> Self {
        self.set_structure_byte_stride(structure_byte_stride);
        self
    }

    pub fn structure_byte_stride(&self) -> ByteCount {
        ByteCount::from(self.0.StructureByteStride)
    }

    pub fn set_counter_offset_in_bytes(
        &mut self,
        counter_offset_in_bytes: ByteCount,
    ) -> &mut Self {
        self.0.CounterOffsetInBytes = counter_offset_in_bytes.0;
        self
    }

    pub fn with_counter_offset_in_bytes(
        mut self,
        counter_offset_in_bytes: ByteCount,
    ) -> Self {
        self.set_counter_offset_in_bytes(counter_offset_in_bytes);
        self
    }

    pub fn counter_offset_in_bytes(&self) -> ByteCount {
        ByteCount(self.0.CounterOffsetInBytes)
    }

    pub fn set_flags(&mut self, flags: BufferUavFlags) -> &mut Self {
        self.0.Flags = flags as i32;
        self
    }

    pub fn with_flags(mut self, flags: BufferUavFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> BufferUavFlags {
        unsafe { std::mem::transmute(self.0.Flags) }
    }
}

/// Wrapper around D3D12_TEX1D_UAV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex1DUav(pub(crate) D3D12_TEX1D_UAV);

impl Tex1DUav {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }
}

/// Wrapper around D3D12_TEX1D_ARRAY_UAV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Tex1DArrayUav(pub(crate) D3D12_TEX1D_ARRAY_UAV);

impl Tex1DArrayUav {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }

    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }
}

/// Wrapper around D3D12_TEX2D_UAV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Tex2DUav(pub(crate) D3D12_TEX2D_UAV);

impl Tex2DUav {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }

    pub fn set_plane_slice(&mut self, plane_slice: u32) -> &mut Self {
        self.0.PlaneSlice = plane_slice;
        self
    }

    pub fn with_plane_slice(mut self, plane_slice: u32) -> Self {
        self.set_plane_slice(plane_slice);
        self
    }

    pub fn plane_slice(&self) -> u32 {
        self.0.PlaneSlice
    }
}

/// Wrapper around D3D12_TEX2D_ARRAY_UAV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Tex2DArrayUav(pub(crate) D3D12_TEX2D_ARRAY_UAV);

impl Tex2DArrayUav {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }

    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }

    pub fn set_plane_slice(&mut self, plane_slice: u32) -> &mut Self {
        self.0.PlaneSlice = plane_slice;
        self
    }

    pub fn with_plane_slice(mut self, plane_slice: u32) -> Self {
        self.set_plane_slice(plane_slice);
        self
    }

    pub fn plane_slice(&self) -> u32 {
        self.0.PlaneSlice
    }
}

/// Wrapper around D3D12_TEX3D_UAV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Tex3DUav(pub(crate) D3D12_TEX3D_UAV);

impl Tex3DUav {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }

    pub fn set_first_w_slice(&mut self, first_w_slice: u32) -> &mut Self {
        self.0.FirstWSlice = first_w_slice;
        self
    }

    pub fn with_first_w_slice(mut self, first_w_slice: u32) -> Self {
        self.set_first_w_slice(first_w_slice);
        self
    }

    pub fn first_w_slice(&self) -> u32 {
        self.0.FirstWSlice
    }

    pub fn set_w_size(&mut self, w_size: u32) -> &mut Self {
        self.0.WSize = w_size;
        self
    }

    pub fn with_w_size(mut self, w_size: u32) -> Self {
        self.set_w_size(w_size);
        self
    }

    pub fn w_size(&self) -> u32 {
        self.0.WSize
    }
}

/// Wrapper around D3D12_CLEAR_VALUE structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct ClearValue(pub(crate) D3D12_CLEAR_VALUE);

impl ClearValue {
    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_color(&mut self, color: [f32; 4usize]) -> &mut Self {
        self.0.__bindgen_anon_1.Color = color;
        self
    }

    pub fn with_color(mut self, color: [f32; 4usize]) -> Self {
        self.set_color(color);
        self
    }

    /// # Safety
    ///
    /// This function doesn't verify the current union variant
    pub unsafe fn color(&self) -> [f32; 4usize] {
        self.0.__bindgen_anon_1.Color
    }

    pub fn set_depth_stencil(
        &mut self,
        depth_stencil: &DepthStencilValue,
    ) -> &mut Self {
        self.0.__bindgen_anon_1.DepthStencil = depth_stencil.0;
        self
    }

    pub fn with_depth_stencil(
        mut self,
        depth_stencil: &DepthStencilValue,
    ) -> Self {
        self.set_depth_stencil(depth_stencil);
        self
    }

    /// # Safety
    ///
    /// This function doesn't verify the current union variant
    pub unsafe fn depth_stencil(&self) -> DepthStencilValue {
        DepthStencilValue(self.0.__bindgen_anon_1.DepthStencil)
    }
}

/// Wrapper around D3D12_DEPTH_STENCIL_VALUE structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct DepthStencilValue(pub(crate) D3D12_DEPTH_STENCIL_VALUE);

impl DepthStencilValue {
    pub fn set_depth(&mut self, depth: f32) -> &mut Self {
        self.0.Depth = depth;
        self
    }

    pub fn with_depth(mut self, depth: f32) -> Self {
        self.set_depth(depth);
        self
    }

    pub fn depth(&self) -> f32 {
        self.0.Depth
    }

    pub fn set_stencil(&mut self, stencil: u8) -> &mut Self {
        self.0.Stencil = stencil;
        self
    }

    pub fn with_stencil(mut self, stencil: u8) -> Self {
        self.set_stencil(stencil);
        self
    }

    pub fn stencil(&self) -> u8 {
        self.0.Stencil
    }
}

/// Wrapper around D3D12_DEPTH_STENCIL_VIEW_DESC structure
#[derive(Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct DepthStencilViewDesc(pub(crate) D3D12_DEPTH_STENCIL_VIEW_DESC);

// ToDo: encode the union variant in wrapper's type?
impl DepthStencilViewDesc {
    pub fn set_format(&mut self, format: Format) -> &mut Self {
        self.0.Format = format as i32;
        self
    }

    pub fn with_format(mut self, format: Format) -> Self {
        self.set_format(format);
        self
    }

    pub fn format(&self) -> Format {
        unsafe { std::mem::transmute(self.0.Format) }
    }

    pub fn set_view_dimension(
        &mut self,
        view_dimension: DsvDimension,
    ) -> &mut Self {
        self.0.ViewDimension = view_dimension as i32;
        self
    }

    pub fn with_view_dimension(mut self, view_dimension: DsvDimension) -> Self {
        self.set_view_dimension(view_dimension);
        self
    }

    pub fn view_dimension(&self) -> DsvDimension {
        unsafe { std::mem::transmute(self.0.ViewDimension) }
    }

    pub fn set_flags(&mut self, flags: DsvFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: DsvFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> DsvFlags {
        unsafe { DsvFlags::from_bits_unchecked(self.0.Flags) }
    }

    pub fn new_texture_1d(mut self, texture_1d: Tex1DDsv) -> Self {
        self.0.ViewDimension = DsvDimension::Texture1D as i32;
        self.0.__bindgen_anon_1.Texture1D = texture_1d.0;
        self
    }

    pub fn texture_1d(&self) -> Option<Tex1DDsv> {
        unsafe {
            match self.view_dimension() {
                DsvDimension::Texture1D => {
                    Some(Tex1DDsv(self.0.__bindgen_anon_1.Texture1D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_1d_array(
        mut self,
        texture_1d_array: Tex1DArrayDsv,
    ) -> Self {
        self.0.ViewDimension = DsvDimension::Texture1DArray as i32;
        self.0.__bindgen_anon_1.Texture1DArray = texture_1d_array.0;
        self
    }

    pub fn texture_1d_array(&self) -> Option<Tex1DArrayDsv> {
        unsafe {
            match self.view_dimension() {
                DsvDimension::Texture1DArray => {
                    Some(Tex1DArrayDsv(self.0.__bindgen_anon_1.Texture1DArray))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d(mut self, texture_2d: Tex2DDsv) -> Self {
        self.0.ViewDimension = DsvDimension::Texture2D as i32;
        self.0.__bindgen_anon_1.Texture2D = texture_2d.0;
        self
    }

    pub fn texture_2d(&self) -> Option<Tex2DDsv> {
        unsafe {
            match self.view_dimension() {
                DsvDimension::Texture2D => {
                    Some(Tex2DDsv(self.0.__bindgen_anon_1.Texture2D))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_array(
        mut self,
        texture_2d_array: Tex2DArrayDsv,
    ) -> Self {
        self.0.ViewDimension = DsvDimension::Texture2DArray as i32;
        self.0.__bindgen_anon_1.Texture2DArray = texture_2d_array.0;
        self
    }

    pub fn texture_2d_array(&self) -> Option<Tex2DArrayDsv> {
        unsafe {
            match self.view_dimension() {
                DsvDimension::Texture2DArray => {
                    Some(Tex2DArrayDsv(self.0.__bindgen_anon_1.Texture2DArray))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_ms(mut self, texture_2d_ms: Tex2DmsDsv) -> Self {
        self.0.ViewDimension = DsvDimension::Texture2DMs as i32;
        self.0.__bindgen_anon_1.Texture2DMS = texture_2d_ms.0;
        self
    }

    pub fn texture_2d_ms(&self) -> Option<Tex2DmsDsv> {
        unsafe {
            match self.view_dimension() {
                DsvDimension::Texture2DMs => {
                    Some(Tex2DmsDsv(self.0.__bindgen_anon_1.Texture2DMS))
                }
                _ => None,
            }
        }
    }

    pub fn new_texture_2d_ms_array(
        mut self,
        texture_2d_ms_array: Tex2DmsArrayDsv,
    ) -> Self {
        self.0.ViewDimension = DsvDimension::Texture2DMsArray as i32;
        self.0.__bindgen_anon_1.Texture2DMSArray = texture_2d_ms_array.0;
        self
    }

    pub fn texture_2d_ms_array(&self) -> Option<Tex2DmsArrayDsv> {
        unsafe {
            match self.view_dimension() {
                DsvDimension::Texture2DMsArray => Some(Tex2DmsArrayDsv(
                    self.0.__bindgen_anon_1.Texture2DMSArray,
                )),
                _ => None,
            }
        }
    }
}

/// Wrapper around D3D12_TEX1D_DSV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex1DDsv(pub(crate) D3D12_TEX1D_DSV);

impl Tex1DDsv {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }
}

/// Wrapper around D3D12_TEX1D_ARRAY_DSV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex1DArrayDsv(pub(crate) D3D12_TEX1D_ARRAY_DSV);

impl Tex1DArrayDsv {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }

    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }
}

/// Wrapper around D3D12_TEX2D_DSV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex2DDsv(pub(crate) D3D12_TEX2D_DSV);

impl Tex2DDsv {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }
}

/// Wrapper around D3D12_TEX2D_ARRAY_DSV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex2DArrayDsv(pub(crate) D3D12_TEX2D_ARRAY_DSV);

impl Tex2DArrayDsv {
    pub fn set_mip_slice(&mut self, mip_slice: u32) -> &mut Self {
        self.0.MipSlice = mip_slice;
        self
    }

    pub fn with_mip_slice(mut self, mip_slice: u32) -> Self {
        self.set_mip_slice(mip_slice);
        self
    }

    pub fn mip_slice(&self) -> u32 {
        self.0.MipSlice
    }

    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }
}

/// Wrapper around D3D12_TEX2DMS_DSV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex2DmsDsv(pub(crate) D3D12_TEX2DMS_DSV);

/// Wrapper around D3D12_TEX2DMS_ARRAY_DSV structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct Tex2DmsArrayDsv(pub(crate) D3D12_TEX2DMS_ARRAY_DSV);

impl Tex2DmsArrayDsv {
    pub fn set_first_array_slice(
        &mut self,
        first_array_slice: u32,
    ) -> &mut Self {
        self.0.FirstArraySlice = first_array_slice;
        self
    }

    pub fn with_first_array_slice(mut self, first_array_slice: u32) -> Self {
        self.set_first_array_slice(first_array_slice);
        self
    }

    pub fn first_array_slice(&self) -> u32 {
        self.0.FirstArraySlice
    }

    pub fn set_array_size(&mut self, array_size: u32) -> &mut Self {
        self.0.ArraySize = array_size;
        self
    }

    pub fn with_array_size(mut self, array_size: u32) -> Self {
        self.set_array_size(array_size);
        self
    }

    pub fn array_size(&self) -> u32 {
        self.0.ArraySize
    }
}

// ToDo: more ::new() constructors for one-field structs?
/// Wrapper around D3D12_FEATURE_DATA_SHADER_MODEL structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Debug)]
#[repr(transparent)]
pub struct FeatureDataShaderModel(pub(crate) D3D12_FEATURE_DATA_SHADER_MODEL);

impl FeatureDataShaderModel {
    pub fn new(highest_shader_model: ShaderModel) -> Self {
        Self(D3D12_FEATURE_DATA_SHADER_MODEL {
            HighestShaderModel: highest_shader_model as i32,
        })
    }

    pub fn highest_shader_model(&self) -> ShaderModel {
        unsafe { std::mem::transmute(self.0.HighestShaderModel) }
    }
}

// ToDo: Default derives in the structs where they don't make sense
// should be cleaned up (in favor of Builder pattern?)
/// Wrapper around D3D12_PIPELINE_STATE_STREAM_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug)]
#[repr(transparent)]
pub struct PipelineStateStreamDesc<'a>(
    pub D3D12_PIPELINE_STATE_STREAM_DESC,
    PhantomData<&'a [u8]>,
);

impl<'a> PipelineStateStreamDesc<'a> {
    pub fn size_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.SizeInBytes)
    }

    pub fn set_pipeline_state_subobject_stream(
        &mut self,
        subobject_stream: &'a [u8],
    ) -> &mut Self {
        self.0.SizeInBytes = subobject_stream.len() as u64;
        self.0.pPipelineStateSubobjectStream =
            subobject_stream.as_ptr() as *mut std::ffi::c_void;
        self.1 = PhantomData;

        self
    }

    pub fn with_pipeline_state_subobject_stream(
        mut self,
        subobject_stream: &'a [u8],
    ) -> Self {
        self.set_pipeline_state_subobject_stream(subobject_stream);
        self
    }

    pub fn pipeline_state_subobject_stream(&self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(
                self.0.pPipelineStateSubobjectStream as *const u8,
                self.0.SizeInBytes as usize,
            )
        }
    }
}

/// An element of a pipeline subobject stream (element type + subobject itself)
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

// ToDo: a similar adapter for GraphicsPipelineState? In d3dx12.h
// they have one, and also one more for compute PSO's
// ToDo: do we realistically need getters here?
/// Mesh shader pipeline description struct (a convenience struct that does not have C counterpart)
#[repr(C)]
#[derive(Debug)]
pub struct MeshShaderPipelineStateDesc<'rs, 'sh> {
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
    sh_phantom_data: PhantomData<ShaderBytecode<'sh>>,
}

impl<'rs, 'sh> Default for MeshShaderPipelineStateDesc<'rs, 'sh> {
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
            Format::Unknown as i32,
        );
        pso_desc.sample_desc = PipelineStateSubobject::new(
            PipelineStateSubobjectType::SampleDesc,
            SampleDesc::default().0,
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
        pso_desc.sh_phantom_data = PhantomData;
        pso_desc
    }
}

impl<'rs, 'sh> MeshShaderPipelineStateDesc<'rs, 'sh> {
    pub fn set_root_signature(
        &mut self,
        root_signature: &'rs RootSignature,
    ) -> &mut Self {
        self.root_signature = PipelineStateSubobject::new(
            PipelineStateSubobjectType::RootSignature,
            root_signature.this,
        );
        self.rs_phantom_data = PhantomData;
        self
    }

    pub fn with_root_signature(
        mut self,
        root_signature: &'rs RootSignature,
    ) -> Self {
        self.set_root_signature(root_signature);
        self
    }

    // ToDo: get rid of lifetimes on COM objects??
    pub fn root_signature(&self) -> RootSignature {
        let root_signature = RootSignature {
            this: self.root_signature.subobject,
        };
        root_signature.add_ref();
        root_signature
    }

    pub fn set_as_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.amplification_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::AS,
            bytecode.0,
        );
        self.sh_phantom_data = PhantomData;
        self
    }

    pub fn with_as_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_as_bytecode(bytecode);
        self
    }

    pub fn as_bytecode(&self) -> ShaderBytecode<'sh> {
        ShaderBytecode(
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: self
                    .amplification_shader
                    .subobject
                    .pShaderBytecode,
                BytecodeLength: self
                    .amplification_shader
                    .subobject
                    .BytecodeLength,
            },
            PhantomData,
        )
    }

    pub fn set_ms_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.mesh_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::MS,
            bytecode.0,
        );
        self.sh_phantom_data = PhantomData;
        self
    }

    pub fn with_ms_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_ms_bytecode(bytecode);
        self
    }

    pub fn ms_bytecode(&self) -> ShaderBytecode<'sh> {
        ShaderBytecode(
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: self.mesh_shader.subobject.pShaderBytecode,
                BytecodeLength: self.mesh_shader.subobject.BytecodeLength,
            },
            PhantomData,
        )
    }

    pub fn set_ps_bytecode(
        &mut self,
        bytecode: &'sh ShaderBytecode,
    ) -> &mut Self {
        self.pixel_shader = PipelineStateSubobject::new(
            PipelineStateSubobjectType::PS,
            bytecode.0,
        );

        self.sh_phantom_data = PhantomData;
        self
    }

    pub fn with_ps_bytecode(mut self, bytecode: &'sh ShaderBytecode) -> Self {
        self.set_ps_bytecode(bytecode);
        self
    }

    pub fn ps_bytecode(&self) -> ShaderBytecode<'sh> {
        ShaderBytecode(
            D3D12_SHADER_BYTECODE {
                pShaderBytecode: self.pixel_shader.subobject.pShaderBytecode,
                BytecodeLength: self.pixel_shader.subobject.BytecodeLength,
            },
            PhantomData,
        )
    }

    pub fn set_blend_state(&mut self, blend_state: BlendDesc) -> &mut Self {
        self.blend_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Blend,
            blend_state.0,
        );
        self
    }

    pub fn with_blend_state(mut self, blend_state: BlendDesc) -> Self {
        self.set_blend_state(blend_state);
        self
    }

    pub fn blend_state(&self) -> BlendDesc {
        BlendDesc(self.blend_state.subobject)
    }

    pub fn set_rasterizer_state(
        &mut self,
        rasterizer_state: RasterizerDesc,
    ) -> &mut Self {
        self.rasterizer_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Rasterizer,
            rasterizer_state.0,
        );
        self
    }

    pub fn with_rasterizer_state(
        mut self,
        rasterizer_state: RasterizerDesc,
    ) -> Self {
        self.set_rasterizer_state(rasterizer_state);
        self
    }

    // ToDo: return reference in such cases??
    pub fn rasterizer_state(&self) -> RasterizerDesc {
        RasterizerDesc(self.rasterizer_state.subobject)
    }

    pub fn set_depth_stencil_state(
        &mut self,
        depth_stencil_state: DepthStencilDesc,
    ) -> &mut Self {
        self.depth_stencil_state = PipelineStateSubobject::new(
            PipelineStateSubobjectType::DepthStencil,
            depth_stencil_state.0,
        );
        self
    }

    pub fn with_depth_stencil_state(
        mut self,
        depth_stencil_state: DepthStencilDesc,
    ) -> Self {
        self.set_depth_stencil_state(depth_stencil_state);
        self
    }

    pub fn depth_stencil_state(&self) -> DepthStencilDesc {
        DepthStencilDesc(self.depth_stencil_state.subobject)
    }

    pub fn set_primitive_topology_type(
        &mut self,
        primitive_topology_type: PrimitiveTopologyType,
    ) -> &mut Self {
        self.primitive_topology_type = PipelineStateSubobject::new(
            PipelineStateSubobjectType::PrimitiveTopology,
            primitive_topology_type as i32,
        );
        self
    }

    pub fn with_primitive_topology_type(
        mut self,
        primitive_topology_type: PrimitiveTopologyType,
    ) -> Self {
        self.set_primitive_topology_type(primitive_topology_type);
        self
    }

    pub fn primitive_topology_type(&self) -> PrimitiveTopologyType {
        unsafe { std::mem::transmute(self.primitive_topology_type.subobject) }
    }

    pub fn set_rtv_formats(&mut self, rtv_formats: &[Format]) -> &mut Self {
        let rt_format_struct =
            RtFormatArray::default().with_rt_formats(rtv_formats);
        self.rtv_formats = PipelineStateSubobject::new(
            PipelineStateSubobjectType::RenderTargetFormats,
            rt_format_struct.0,
        );
        self
    }

    pub fn with_rtv_formats(mut self, rtv_formats: &[Format]) -> Self {
        self.set_rtv_formats(rtv_formats);
        self
    }

    pub fn rtv_formats(&self) -> &[Format] {
        unsafe {
            slice::from_raw_parts(
                self.rtv_formats.subobject.RTFormats.as_ptr() as *const Format,
                self.rtv_formats.subobject.NumRenderTargets as usize,
            )
        }
    }

    pub fn set_dsv_format(&mut self, dsv_format: Format) -> &mut Self {
        self.dsv_format = PipelineStateSubobject::new(
            PipelineStateSubobjectType::DepthStencilFormat,
            dsv_format as i32,
        );
        self
    }

    pub fn with_dsv_format(mut self, dsv_format: Format) -> Self {
        self.set_dsv_format(dsv_format);
        self
    }

    pub fn set_flags(
        &mut self,
        pipeline_state_flags: PipelineStateFlags,
    ) -> &mut Self {
        self.flags = PipelineStateSubobject::new(
            PipelineStateSubobjectType::Flags,
            pipeline_state_flags.bits(),
        );
        self
    }

    pub fn with_flags(
        mut self,
        pipeline_state_flags: PipelineStateFlags,
    ) -> Self {
        self.set_flags(pipeline_state_flags);
        self
    }

    pub fn flags(&self) -> PipelineStateFlags {
        unsafe { PipelineStateFlags::from_bits_unchecked(self.flags.subobject) }
    }

    pub fn as_byte_stream(&self) -> &[u8] {
        unsafe {
            std::slice::from_raw_parts(
                self as *const Self as *const u8,
                std::mem::size_of::<Self>(),
            )
        }
    }
}

/// Wrapper around D3D12_RT_FORMAT_ARRAY structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Copy, Clone, Default, Debug)]
#[repr(transparent)]
pub struct RtFormatArray(pub(crate) D3D12_RT_FORMAT_ARRAY);

impl RtFormatArray {
    pub fn set_rt_formats(&mut self, rt_formats: &[Format]) -> &mut Self {
        for format_index in 0..rt_formats.len() {
            self.0.RTFormats[format_index] = rt_formats[format_index] as i32;
        }
        self.0.NumRenderTargets = rt_formats.len() as u32;
        self
    }

    pub fn with_rt_formats(mut self, rt_formats: &[Format]) -> Self {
        self.set_rt_formats(rt_formats);
        self
    }

    pub fn rt_formats(&self) -> &[Format] {
        unsafe {
            slice::from_raw_parts(
                self.0.RTFormats.as_ptr() as *const Format,
                self.0.NumRenderTargets as usize,
            )
        }
    }
}

/// Wrapper around D3D12_QUERY_HEAP_DESC structure
#[repr(transparent)]
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Debug, Copy, Clone)]
pub struct QueryHeapDesc(pub(crate) D3D12_QUERY_HEAP_DESC);

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
    pub fn set_heap_type(&mut self, heap_type: QueryHeapType) -> &mut Self {
        self.0.Type = heap_type as i32;
        self
    }

    pub fn with_heap_type(mut self, heap_type: QueryHeapType) -> Self {
        self.set_heap_type(heap_type);
        self
    }

    pub fn heap_type(&self) -> QueryHeapType {
        unsafe { std::mem::transmute(self.0.Type) }
    }

    pub fn set_count(&mut self, count: u32) -> &mut Self {
        self.0.Count = count;
        self
    }

    pub fn with_count(mut self, count: u32) -> Self {
        self.set_count(count);
        self
    }

    pub fn count(&self) -> u32 {
        self.0.Count
    }

    pub fn set_node_mask(&mut self, node_mask: u32) -> &mut Self {
        self.0.NodeMask = node_mask;
        self
    }

    pub fn with_node_mask(mut self, node_mask: u32) -> Self {
        self.set_node_mask(node_mask);
        self
    }

    pub fn node_mask(&self) -> u32 {
        self.0.NodeMask
    }
}

/// Wrapper around D3D12_FEATURE_DATA_D3D12_OPTIONS structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug, Copy, Clone)]
#[repr(transparent)]
pub struct FeatureDataOptions(pub(crate) D3D12_FEATURE_DATA_D3D12_OPTIONS);

// ToDo: remove setters from here since they don't make sense?
impl FeatureDataOptions {
    pub fn set_double_precision_float_shader_ops(
        &mut self,
        double_precision_float_shader_ops: bool,
    ) -> &mut Self {
        self.0.DoublePrecisionFloatShaderOps =
            double_precision_float_shader_ops as i32;
        self
    }

    pub fn with_double_precision_float_shader_ops(
        mut self,
        double_precision_float_shader_ops: bool,
    ) -> Self {
        self.set_double_precision_float_shader_ops(
            double_precision_float_shader_ops,
        );
        self
    }

    pub fn double_precision_float_shader_ops(&self) -> bool {
        self.0.DoublePrecisionFloatShaderOps != 0
    }

    pub fn set_output_merger_logic_op(
        &mut self,
        output_merger_logic_op: bool,
    ) -> &mut Self {
        self.0.OutputMergerLogicOp = output_merger_logic_op as i32;
        self
    }

    pub fn with_output_merger_logic_op(
        mut self,
        output_merger_logic_op: bool,
    ) -> Self {
        self.set_output_merger_logic_op(output_merger_logic_op);
        self
    }

    pub fn output_merger_logic_op(&self) -> bool {
        self.0.OutputMergerLogicOp != 0
    }

    pub fn set_min_precision_support(
        &mut self,
        min_precision_support: ShaderMinPrecisionSupport,
    ) -> &mut Self {
        self.0.MinPrecisionSupport = min_precision_support as i32;
        self
    }

    pub fn with_min_precision_support(
        mut self,
        min_precision_support: ShaderMinPrecisionSupport,
    ) -> Self {
        self.set_min_precision_support(min_precision_support);
        self
    }

    pub fn min_precision_support(&self) -> ShaderMinPrecisionSupport {
        unsafe { std::mem::transmute(self.0.MinPrecisionSupport) }
    }

    pub fn set_tiled_resources_tier(
        &mut self,
        tiled_resources_tier: TiledResourcesTier,
    ) -> &mut Self {
        self.0.TiledResourcesTier = tiled_resources_tier as i32;
        self
    }

    pub fn with_tiled_resources_tier(
        mut self,
        tiled_resources_tier: TiledResourcesTier,
    ) -> Self {
        self.set_tiled_resources_tier(tiled_resources_tier);
        self
    }

    pub fn tiled_resources_tier(&self) -> TiledResourcesTier {
        unsafe { std::mem::transmute(self.0.TiledResourcesTier) }
    }

    pub fn set_resource_binding_tier(
        &mut self,
        resource_binding_tier: ResourceBindingTier,
    ) -> &mut Self {
        self.0.ResourceBindingTier = resource_binding_tier as i32;
        self
    }

    pub fn with_resource_binding_tier(
        mut self,
        resource_binding_tier: ResourceBindingTier,
    ) -> Self {
        self.set_resource_binding_tier(resource_binding_tier);
        self
    }

    pub fn resource_binding_tier(&self) -> ResourceBindingTier {
        unsafe { std::mem::transmute(self.0.ResourceBindingTier) }
    }

    pub fn set_ps_specified_stencil_ref_supported(
        &mut self,
        ps_specified_stencil_ref_supported: bool,
    ) -> &mut Self {
        self.0.PSSpecifiedStencilRefSupported =
            ps_specified_stencil_ref_supported as i32;
        self
    }

    pub fn with_ps_specified_stencil_ref_supported(
        mut self,
        ps_specified_stencil_ref_supported: bool,
    ) -> Self {
        self.set_ps_specified_stencil_ref_supported(
            ps_specified_stencil_ref_supported,
        );
        self
    }

    pub fn ps_specified_stencil_ref_supported(&self) -> bool {
        self.0.PSSpecifiedStencilRefSupported != 0
    }

    pub fn set_typed_uav_load_additional_formats(
        &mut self,
        typed_uav_load_additional_formats: bool,
    ) -> &mut Self {
        self.0.TypedUAVLoadAdditionalFormats =
            typed_uav_load_additional_formats as i32;
        self
    }

    pub fn with_typed_uav_load_additional_formats(
        mut self,
        typed_uav_load_additional_formats: bool,
    ) -> Self {
        self.set_typed_uav_load_additional_formats(
            typed_uav_load_additional_formats,
        );
        self
    }

    pub fn typed_uav_load_additional_formats(&self) -> bool {
        self.0.TypedUAVLoadAdditionalFormats != 0
    }

    pub fn set_rovs_supported(&mut self, rovs_supported: bool) -> &mut Self {
        self.0.ROVsSupported = rovs_supported as i32;
        self
    }

    pub fn with_rovs_supported(mut self, rovs_supported: bool) -> Self {
        self.set_rovs_supported(rovs_supported);
        self
    }

    pub fn rovs_supported(&self) -> bool {
        self.0.ROVsSupported != 0
    }

    pub fn set_conservative_rasterization_tier(
        &mut self,
        conservative_rasterization_tier: ConservativeRasterizationTier,
    ) -> &mut Self {
        self.0.ConservativeRasterizationTier =
            conservative_rasterization_tier as i32;
        self
    }

    pub fn with_conservative_rasterization_tier(
        mut self,
        conservative_rasterization_tier: ConservativeRasterizationTier,
    ) -> Self {
        self.set_conservative_rasterization_tier(
            conservative_rasterization_tier,
        );
        self
    }

    pub fn conservative_rasterization_tier(
        &self,
    ) -> ConservativeRasterizationTier {
        unsafe { std::mem::transmute(self.0.ConservativeRasterizationTier) }
    }

    pub fn set_max_gpu_virtual_address_bits_per_resource(
        &mut self,
        max_gpu_virtual_address_bits_per_resource: u32,
    ) -> &mut Self {
        self.0.MaxGPUVirtualAddressBitsPerResource =
            max_gpu_virtual_address_bits_per_resource;
        self
    }

    pub fn with_max_gpu_virtual_address_bits_per_resource(
        mut self,
        max_gpu_virtual_address_bits_per_resource: u32,
    ) -> Self {
        self.set_max_gpu_virtual_address_bits_per_resource(
            max_gpu_virtual_address_bits_per_resource,
        );
        self
    }

    pub fn max_gpu_virtual_address_bits_per_resource(&self) -> u32 {
        self.0.MaxGPUVirtualAddressBitsPerResource
    }

    pub fn set_standard_swizzle_64_kb_supported(
        &mut self,
        standard_swizzle_64_kb_supported: bool,
    ) -> &mut Self {
        self.0.StandardSwizzle64KBSupported =
            standard_swizzle_64_kb_supported as i32;
        self
    }

    pub fn with_standard_swizzle_64_kb_supported(
        mut self,
        standard_swizzle_64_kb_supported: bool,
    ) -> Self {
        self.set_standard_swizzle_64_kb_supported(
            standard_swizzle_64_kb_supported,
        );
        self
    }

    pub fn standard_swizzle_64_kb_supported(&self) -> bool {
        self.0.StandardSwizzle64KBSupported != 0
    }

    pub fn set_cross_node_sharing_tier(
        &mut self,
        cross_node_sharing_tier: CrossNodeSharingTier,
    ) -> &mut Self {
        self.0.CrossNodeSharingTier = cross_node_sharing_tier as i32;
        self
    }

    pub fn with_cross_node_sharing_tier(
        mut self,
        cross_node_sharing_tier: CrossNodeSharingTier,
    ) -> Self {
        self.set_cross_node_sharing_tier(cross_node_sharing_tier);
        self
    }

    pub fn cross_node_sharing_tier(&self) -> CrossNodeSharingTier {
        unsafe { std::mem::transmute(self.0.CrossNodeSharingTier) }
    }

    pub fn set_cross_adapter_row_major_texture_supported(
        &mut self,
        cross_adapter_row_major_texture_supported: bool,
    ) -> &mut Self {
        self.0.CrossAdapterRowMajorTextureSupported =
            cross_adapter_row_major_texture_supported as i32;
        self
    }

    pub fn with_cross_adapter_row_major_texture_supported(
        mut self,
        cross_adapter_row_major_texture_supported: bool,
    ) -> Self {
        self.set_cross_adapter_row_major_texture_supported(
            cross_adapter_row_major_texture_supported,
        );
        self
    }

    pub fn cross_adapter_row_major_texture_supported(&self) -> bool {
        self.0.CrossAdapterRowMajorTextureSupported != 0
    }

    pub fn set_vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation(
        &mut self,
        vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation: bool,
    ) -> &mut Self {
        self.0.VPAndRTArrayIndexFromAnyShaderFeedingRasterizerSupportedWithoutGSEmulation = vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation as i32;
        self
    }

    pub fn with_vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation(
        mut self,
        vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation: bool,
    ) -> Self {
        self.set_vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation(vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation);
        self
    }

    pub fn vp_and_rt_array_index_from_any_shader_feeding_rasterizer_supported_without_gs_emulation(
        &self,
    ) -> bool {
        self.0.VPAndRTArrayIndexFromAnyShaderFeedingRasterizerSupportedWithoutGSEmulation != 0
    }

    pub fn set_resource_heap_tier(
        &mut self,
        resource_heap_tier: ResourceHeapTier,
    ) -> &mut Self {
        self.0.ResourceHeapTier = resource_heap_tier as i32;
        self
    }

    pub fn with_resource_heap_tier(
        mut self,
        resource_heap_tier: ResourceHeapTier,
    ) -> Self {
        self.set_resource_heap_tier(resource_heap_tier);
        self
    }

    pub fn resource_heap_tier(&self) -> ResourceHeapTier {
        unsafe { std::mem::transmute(self.0.ResourceHeapTier) }
    }
}

/// Wrapper around D3D12_RESOURCE_ALLOCATION_INFO structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug, Copy, Clone)]
#[repr(transparent)]
pub struct ResourceAllocationInfo(pub(crate) D3D12_RESOURCE_ALLOCATION_INFO);

impl ResourceAllocationInfo {
    pub fn set_size_in_bytes(&mut self, size_in_bytes: ByteCount) -> &mut Self {
        self.0.SizeInBytes = size_in_bytes.0;
        self
    }

    pub fn with_size_in_bytes(mut self, size_in_bytes: ByteCount) -> Self {
        self.set_size_in_bytes(size_in_bytes);
        self
    }

    pub fn size_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.SizeInBytes)
    }

    pub fn set_alignment(&mut self, alignment: ByteCount) -> &mut Self {
        self.0.Alignment = alignment.0;
        self
    }

    pub fn with_alignment(mut self, alignment: ByteCount) -> Self {
        self.set_alignment(alignment);
        self
    }

    pub fn alignment(&self) -> ByteCount {
        ByteCount::from(self.0.Alignment)
    }
}

/// Wrapper around D3D12_HEAP_DESC structure
#[derive(Hash, PartialOrd, Ord, PartialEq, Eq, Default, Debug, Copy, Clone)]
#[repr(transparent)]
pub struct HeapDesc(pub(crate) D3D12_HEAP_DESC);

impl HeapDesc {
    pub fn set_size_in_bytes(&mut self, size_in_bytes: ByteCount) -> &mut Self {
        self.0.SizeInBytes = size_in_bytes.0;
        self
    }

    pub fn with_size_in_bytes(mut self, size_in_bytes: ByteCount) -> Self {
        self.set_size_in_bytes(size_in_bytes);
        self
    }

    pub fn size_in_bytes(&self) -> ByteCount {
        ByteCount::from(self.0.SizeInBytes)
    }

    pub fn set_properties(&mut self, properties: HeapProperties) -> &mut Self {
        self.0.Properties = properties.0;
        self
    }

    pub fn with_properties(mut self, properties: HeapProperties) -> Self {
        self.set_properties(properties);
        self
    }

    pub fn properties(&self) -> HeapProperties {
        HeapProperties(self.0.Properties)
    }

    pub fn set_alignment(&mut self, alignment: ByteCount) -> &mut Self {
        self.0.Alignment = alignment.0;
        self
    }

    pub fn with_alignment(mut self, alignment: ByteCount) -> Self {
        self.set_alignment(alignment);
        self
    }

    pub fn alignment(&self) -> ByteCount {
        ByteCount::from(self.0.Alignment)
    }

    pub fn set_flags(&mut self, flags: HeapFlags) -> &mut Self {
        self.0.Flags = flags.bits();
        self
    }

    pub fn with_flags(mut self, flags: HeapFlags) -> Self {
        self.set_flags(flags);
        self
    }

    pub fn flags(&self) -> HeapFlags {
        unsafe { HeapFlags::from_bits_unchecked(self.0.Flags) }
    }
}
