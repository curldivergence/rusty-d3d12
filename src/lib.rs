use log::{trace, warn};
use std::default::Default;
use std::ffi::{c_void, CString};
use std::os::raw::c_char;
use std::{slice, str};
use winapi::shared::winerror;

#[macro_use]
extern crate static_assertions;

mod raw_bindings;
pub use raw_bindings::d3d12::*;

#[macro_use]
mod utils;
pub use utils::*;

mod const_wrappers;
pub use const_wrappers::*;

mod struct_wrappers;
pub use struct_wrappers::*;

mod enum_wrappers;
pub use enum_wrappers::*;

// ToDo: macro?
fn cast_to_ppv<T>(pointer: &mut *mut T) -> *mut *mut std::ffi::c_void {
    pointer as *mut *mut T as *mut *mut std::ffi::c_void
}

// Behold! This macro and the one below now can accept both methods and
// plain functions.
// ToDo: is there a way to fix the trailing comma issue?
macro_rules! dx_call {
    ($object_ptr:expr, $method_name:ident, $($args:expr),*) => {{
        let vtbl = (*$object_ptr).lpVtbl;
        let raw_func = (*vtbl).$method_name.unwrap();
        raw_func($object_ptr, $($args),*)
    }};
    ($fn_name:ident $args:tt) => {$fn_name $args;}
}

// ToDo: better name?
macro_rules! dx_try {
    ($object_ptr:expr, $method_name:ident, $($args:expr),*) => {{
        let vtbl = (*$object_ptr).lpVtbl;
        let raw_func = (*vtbl).$method_name.unwrap();
        let ret_code =  raw_func($object_ptr, $($args),*);
        if fail!(ret_code) {
            return Err(DxError::new(
                stringify!($method_name),
                ret_code,
            ));
        }
    }};
    ($fn_name:ident $args:tt) => {{
        let ret_code = $fn_name $args;
        if fail!(ret_code) {
            return Err(DxError::new(
                stringify!($fn_name),
                ret_code,
            ));
        }
    }}
}

const MAX_FUNC_NAME_LEN: usize = 64;
const MAX_ERROR_MSG_LEN: usize = 512;

pub struct DxError([u8; MAX_FUNC_NAME_LEN], HRESULT);

impl DxError {
    pub fn new(func_name: &str, err_code: HRESULT) -> Self {
        use std::io::Write;
        let mut func_name_owned = [0; MAX_FUNC_NAME_LEN];
        write!(&mut func_name_owned[..], "{}", func_name,)
            .expect("Ironically, DxError creation has failed");
        Self(func_name_owned, err_code)
    }

    fn write_as_str(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        unsafe {
            use winapi::um::winbase::{
                FormatMessageA, FORMAT_MESSAGE_FROM_SYSTEM,
                FORMAT_MESSAGE_IGNORE_INSERTS,
            };
            let mut error_message = [0; MAX_ERROR_MSG_LEN];
            let _char_count = FormatMessageA(
                FORMAT_MESSAGE_FROM_SYSTEM | FORMAT_MESSAGE_IGNORE_INSERTS,
                std::ptr::null(),
                self.1 as u32,
                0,
                &mut error_message as *mut _ as *mut i8,
                MAX_ERROR_MSG_LEN as u32,
                std::ptr::null_mut(),
            );

            // FormatMessage shoves in new line symbols for some reason
            for char in &mut error_message {
                if *char == 0xA || *char == 0xD {
                    *char = 0x20;
                }
            }

            write!(
                f,
                "{} failed: [{:#010x}] {}",
                std::str::from_utf8(&self.0)
                    .expect("Cannot format error message: function name is not valid utf-8"),
                self.1,
                std::str::from_utf8(&error_message)
                    .expect("Cannot format error message: error description is not valid utf-8"),
            )
        }
    }
}

impl std::error::Error for DxError {}

impl std::fmt::Display for DxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_as_str(f)
    }
}

impl std::fmt::Debug for DxError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.write_as_str(f)
    }
}

pub type DxResult<T> = Result<T, DxError>;

macro_rules! success {
    ($ret_code:expr) => {
        $ret_code >= winerror::S_OK
    };
}

macro_rules! fail {
    ($ret_code:expr) => {
        $ret_code < winerror::S_OK
    };
}

macro_rules! cast_to_iunknown {
    ($pointer:expr) => {{
        let mut result: *mut IUnknown = std::ptr::null_mut();
        dx_try!(
            $pointer,
            QueryInterface,
            &IID_IUnknown,
            cast_to_ppv(&mut result)
        );

        dx_call!($pointer, Release,);
        result
    }};
}

// Of course these macros should be traits instead, but there is no way
// to refer to fields in a trait :(
macro_rules! impl_com_object_clone_drop{
    ($struct_type:ty
        $(, $extra_member:ident)*
    ) => {
        impl Clone for $struct_type {
            fn clone(&self) -> Self {
                self.add_ref();
                Self {
                    this: self.this,
                    $(
                        $extra_member: self.$extra_member,
                    )*
                }
            }
        }

        impl Drop for $struct_type {
            fn drop(&mut self) {
                self.release();
            }
        }
    };
}

macro_rules! impl_com_object_refcount_unnamed {
    ($struct_type:ty
        $(, $extra_member:ident)*
    ) => {
        impl $struct_type {
            pub fn add_ref(&self) -> u64 {
                unsafe {
                    let live_ref_count: ULONG = dx_call!(self.this, AddRef,);

                    #[cfg(feature = "log_ref_counting")]
                    trace!(
                        "Increased refcount for {}, live reference count: {}",
                        stringify!($struct_type),
                        live_ref_count
                    );

                    live_ref_count as u64
                }
            }

            pub fn release(&self) -> u64 {
                unsafe {
                    let live_ref_count: ULONG = dx_call!(self.this, Release,);

                    #[cfg(feature = "log_ref_counting")]
                    trace!(
                        "Released {}, live reference count: {}",
                        stringify!($struct_type),
                        live_ref_count
                    );

                    live_ref_count as u64
                }
            }
        }
    };
}

macro_rules! impl_com_object_refcount_named {
    ($struct_type:ty
        $(, $extra_member:ident)*
        ) => {
        impl $struct_type {
            pub fn add_ref(&self) -> u64 {
                unsafe {
                    let live_ref_count: ULONG = dx_call!(self.this, AddRef,);
                    #[cfg(feature = "log_ref_counting")]
                    {
                        let name =   self.get_name();
                        trace!(
                                "Increased refcount for {} '{}', live reference count: {}",
                                stringify!($struct_type),
                                match name.as_ref() {
                                    Ok(name) => name,
                                    Err(_) => "unnamed object"
                                },
                            live_ref_count
                        )
                    }
                    live_ref_count as u64
                }
            }

            pub fn release(&self) -> u64 {
                unsafe {
                    #[cfg(feature = "log_ref_counting")]
                    let name = self.get_name();
                    let live_ref_count: ULONG = dx_call!(self.this, Release,);
                    #[cfg(feature = "log_ref_counting")]
                    {
                        trace!(
                            "Released {} '{}', live reference count: {}",
                            stringify!($struct_type),
                            match name.as_ref() {
                                Ok(name) => name,
                                Err(_) => "unnamed object",
                            },
                            live_ref_count
                        );
                    }
                    live_ref_count as u64
                }
            }
        }
    }
}

macro_rules! impl_com_object_set_get_name {
    ($struct_type:ty
        $(, $extra_member:ident)*
    ) => {
        impl $struct_type {
            pub fn set_name(&self, name: &str) -> DxResult<()> {
                let name_wstr = widestring::U16CString::from_str(name)
                    .expect("Cannot convert object name to utf-16");
                unsafe {
                    dx_try!(self.this, SetName, name_wstr.as_ptr());
                }
                Ok(())
            }

            pub fn get_name(&self) -> DxResult<String> {
                let mut buffer_size = 128u32;
                let buffer = vec![0; buffer_size as usize];
                unsafe {
                    dx_try!(
                        self.this,
                        GetPrivateData,
                        &WKPDID_D3DDebugObjectNameW,
                        &mut buffer_size,
                        buffer.as_ptr() as *mut std::ffi::c_void
                    );
                }

                widestring::U16CString::from_vec_with_nul(buffer).map_or_else(
                    |_| Err(DxError::new("U16CString::from_vec_with_nul", -1)),
                    |name_wstr| {
                        name_wstr
                            .to_string()
                            .and_then(|name_string| Ok(name_string))
                            .or_else(|_| {
                                Err(DxError::new("U16CString::to_string", -1))
                            })
                    },
                )
            }
        }
    };
}

pub fn d3d_enable_experimental_shader_models() -> DxResult<()> {
    unsafe {
        let guid = GUID {
            Data1: 0x76f5573e,
            Data2: 0xf13a,
            Data3: 0x40f5,
            Data4: [0xb2, 0x97, 0x81, 0xce, 0x9e, 0x18, 0x93, 0x3f],
        };

        dx_try!(D3D12EnableExperimentalFeatures(
            1,
            &guid,
            std::ptr::null_mut(),
            std::ptr::null_mut()
        ));

        Ok(())
    }
}

#[derive(Debug)]
pub struct Debug {
    pub this: *mut ID3D12Debug5,
}
impl_com_object_refcount_unnamed!(Debug);
impl_com_object_clone_drop!(Debug);

impl Debug {
    pub fn new() -> DxResult<Self> {
        let mut debug_interface: *mut ID3D12Debug5 = std::ptr::null_mut();
        unsafe {
            dx_try!(D3D12GetDebugInterface(
                &IID_ID3D12Debug5,
                cast_to_ppv(&mut debug_interface),
            ));

            Ok(Debug {
                this: debug_interface,
            })
        }
    }

    pub fn enable_debug_layer(&self) {
        unsafe { dx_call!(self.this, EnableDebugLayer,) }
    }

    pub fn enable_gpu_based_validation(&self) {
        unsafe { dx_call!(self.this, SetEnableGPUBasedValidation, 1) }
    }

    pub fn enable_object_auto_name(&self) {
        unsafe { dx_call!(self.this, SetEnableAutoName, 1) }
    }
}

#[derive(Debug)]
pub struct InfoQueue {
    pub this: *mut ID3D12InfoQueue1,
}
impl_com_object_refcount_unnamed!(InfoQueue);
impl_com_object_clone_drop!(InfoQueue);

impl InfoQueue {
    pub fn new(
        device: &Device,
        break_flags: Option<&[MessageSeverity]>,
    ) -> DxResult<Self> {
        let mut info_queue: *mut ID3D12InfoQueue1 = std::ptr::null_mut();
        unsafe {
            dx_try!(
                device.this,
                QueryInterface,
                &IID_ID3D12InfoQueue1,
                cast_to_ppv(&mut info_queue)
            );
            // ToDo: do we need it? It leads to refcount-related exceptions
            // under certain circumstances (see commit a738100)
            // device.release();

            if let Some(break_flags) = break_flags {
                for flag in break_flags {
                    dx_try!(info_queue, SetBreakOnSeverity, *flag as i32, 1);
                }
            }
        }
        Ok(InfoQueue { this: info_queue })
    }

    pub fn get_messages(&self) -> DxResult<Vec<String>> {
        let mut messages: Vec<String> = Vec::new();
        unsafe {
            let message_count = dx_call!(self.this, GetNumStoredMessages,);

            for message_index in 0..message_count {
                let mut message_size: SIZE_T = 0;
                dx_try!(
                    self.this,
                    GetMessageA,
                    message_index,
                    std::ptr::null_mut(),
                    &mut message_size
                );

                let allocation_layout = std::alloc::Layout::from_size_align(
                    message_size as usize,
                    8,
                )
                .expect("Wrong allocation layout");
                let message_struct =
                    std::alloc::alloc(allocation_layout) as *mut D3D12_MESSAGE;
                dx_try!(
                    self.this,
                    GetMessageA,
                    message_index,
                    message_struct,
                    &mut message_size
                );

                let message_string =
                    str::from_utf8_unchecked(slice::from_raw_parts(
                        (*message_struct).pDescription as *const u8,
                        (*message_struct).DescriptionByteLength as usize,
                    ));
                messages.push(message_string.to_string());
                std::alloc::dealloc(
                    message_struct as *mut u8,
                    allocation_layout,
                )
            }
            dx_call!(self.this, ClearStoredMessages,);
        }
        Ok(messages)
    }

    pub fn print_messages(&self) -> DxResult<()> {
        let messages = self.get_messages()?;
        for message in messages {
            warn!("{}", message);
        }

        Ok(())
    }

    pub fn register_callback(
        &self,
        callback: unsafe extern "C" fn(
            i32,
            i32,
            i32,
            *const c_char,
            *mut c_void,
        ) -> (),
        filter_flags: MessageCallbackFlags,
        // ToDo: context and cookie
    ) -> DxResult<()> {
        unsafe {
            let mut cookie = 0u32;
            dx_try!(
                self.this,
                RegisterMessageCallback,
                Some(callback),
                filter_flags as i32,
                std::ptr::null_mut(),
                &mut cookie
            );
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct DebugDevice {
    pub this: *mut ID3D12DebugDevice,
}
impl_com_object_refcount_unnamed!(DebugDevice);
impl_com_object_clone_drop!(DebugDevice);

impl DebugDevice {
    pub fn new(device: &Device) -> DxResult<Self> {
        let mut debug_device: *mut ID3D12DebugDevice = std::ptr::null_mut();
        unsafe {
            dx_try!(
                device.this,
                QueryInterface,
                &IID_ID3D12DebugDevice,
                cast_to_ppv(&mut debug_device)
            );

            // dx_call!(
            //     info_queue,
            //     SetBreakOnSeverity,
            //     D3D12_MESSAGE_SEVERITY_D3D12_MESSAGE_SEVERITY_WARNING,
            //     1
            // );
        }

        Ok(Self { this: debug_device })
    }

    pub fn report_live_device_objects(&self) -> DxResult<()> {
        unsafe {
            dx_try!(
                self.this,
                ReportLiveDeviceObjects,
                D3D12_RLDO_FLAGS_D3D12_RLDO_DETAIL
            )
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Factory {
    pub this: *mut IDXGIFactory6,
}
impl_com_object_refcount_unnamed!(Factory);
impl_com_object_clone_drop!(Factory);

impl Factory {
    pub fn new(flags: CreateFactoryFlags) -> DxResult<Self> {
        let mut factory: *mut IDXGIFactory6 = std::ptr::null_mut();
        unsafe {
            dx_try!(CreateDXGIFactory2(
                flags.bits(),
                &IID_IDXGIFactory6,
                cast_to_ppv(&mut factory),
            ));
        }
        Ok(Factory { this: factory })
    }

    pub fn enum_adapters(&self) -> DxResult<Vec<Adapter>> {
        let mut result: Vec<Adapter> = vec![];

        unsafe {
            let mut adapter_index = 0;
            loop {
                let mut temp_adapter: *mut IDXGIAdapter1 = std::ptr::null_mut();

                let ret_code = dx_call!(
                    self.this,
                    EnumAdapters1,
                    adapter_index,
                    &mut temp_adapter
                );
                if ret_code == winerror::DXGI_ERROR_NOT_FOUND {
                    break;
                } else if ret_code != winerror::S_OK {
                    return Err(DxError::new("EnumAdapters1", ret_code));
                }

                let mut real_adapter: *mut IDXGIAdapter3 = std::ptr::null_mut();
                dx_try!(
                    temp_adapter,
                    QueryInterface,
                    &IID_IDXGIAdapter3,
                    cast_to_ppv(&mut real_adapter)
                );

                // Apparently QueryInterface increases ref count?
                dx_call!(temp_adapter, Release,);

                result.push(Adapter { this: real_adapter });
                adapter_index += 1;
            }
        }
        Ok(result)
    }

    pub fn enum_adapters_by_gpu_preference(
        &self,
        preference: GpuPreference,
    ) -> DxResult<Vec<Adapter>> {
        let mut result: Vec<Adapter> = vec![];

        unsafe {
            let mut adapter_index = 0;
            loop {
                let mut adapter: *mut IDXGIAdapter3 = std::ptr::null_mut();

                let ret_code = dx_call!(
                    self.this,
                    EnumAdapterByGpuPreference,
                    adapter_index,
                    preference as i32,
                    &IID_IDXGIAdapter3,
                    cast_to_ppv(&mut adapter)
                );
                if ret_code == winerror::DXGI_ERROR_NOT_FOUND {
                    break;
                } else if ret_code != winerror::S_OK {
                    return Err(DxError::new(
                        "EnumAdapterByGpuPreference",
                        ret_code,
                    ));
                }

                result.push(Adapter { this: adapter });
                adapter_index += 1;
            }
        }
        Ok(result)
    }

    pub fn enum_warp_adapter(&self) -> DxResult<Adapter> {
        let mut hw_adapter: *mut IDXGIAdapter3 = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                EnumWarpAdapter,
                &IID_IDXGIAdapter3,
                cast_to_ppv(&mut hw_adapter)
            );
        }

        Ok(Adapter { this: hw_adapter })
    }

    pub fn create_swapchain(
        &self,
        command_queue: &CommandQueue,
        window_handle: HWND,
        desc: &SwapchainDesc,
    ) -> DxResult<Swapchain> {
        let mut temp_hw_swapchain: *mut IDXGISwapChain1 = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreateSwapChainForHwnd,
                cast_to_iunknown!(command_queue.this),
                window_handle,
                &desc.0,
                std::ptr::null(),
                std::ptr::null_mut(),
                &mut temp_hw_swapchain
            );
        }

        let mut hw_swapchain: *mut IDXGISwapChain4 = std::ptr::null_mut();
        unsafe {
            dx_try!(
                temp_hw_swapchain,
                QueryInterface,
                &IID_IDXGISwapChain4,
                cast_to_ppv(&mut hw_swapchain)
            );
        }
        Ok(Swapchain { this: hw_swapchain })
    }

    pub fn make_window_association(
        &self,
        hwnd: *mut std::ffi::c_void,
        flags: MakeWindowAssociationFlags,
    ) -> DxResult<()> {
        unsafe {
            dx_try!(
                self.this,
                MakeWindowAssociation,
                hwnd as HWND,
                flags.bits()
            )
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Adapter {
    pub this: *mut IDXGIAdapter3,
}
impl_com_object_refcount_unnamed!(Adapter);
impl_com_object_clone_drop!(Adapter);

impl Adapter {
    pub fn get_desc(&self) -> DxResult<AdapterDesc> {
        let mut hw_adapter_desc = AdapterDesc::default();
        unsafe {
            dx_try!(self.this, GetDesc1, &mut hw_adapter_desc.0);
        }
        Ok(hw_adapter_desc)
    }
}

#[derive(Debug)]
pub struct Device {
    pub this: *mut ID3D12Device2,
}
impl_com_object_refcount_unnamed!(Device);
impl_com_object_clone_drop!(Device);

// ToDo: clean up Send and Sync implementations
unsafe impl Send for Device {}
// unsafe impl Sync for Device {}

impl Device {
    pub fn check_feature_support<T>(
        &self,
        feature: Feature,
        feature_support_data: &mut T,
    ) -> DxResult<()> {
        unsafe {
            let data = feature_support_data as *mut _ as *mut std::ffi::c_void;
            let data_size = std::mem::size_of::<T>() as u32;

            dx_try!(
                self.this,
                CheckFeatureSupport,
                feature as i32,
                data,
                data_size
            );
        }

        Ok(())
    }

    pub fn create_command_allocator(
        &self,
        command_list_type: CommandListType,
    ) -> DxResult<CommandAllocator> {
        let mut hw_command_allocator: *mut ID3D12CommandAllocator =
            std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateCommandAllocator,
                command_list_type as i32,
                &IID_ID3D12CommandAllocator,
                cast_to_ppv(&mut hw_command_allocator)
            )
        }

        Ok(CommandAllocator {
            this: hw_command_allocator,
        })
    }

    pub fn create_command_list(
        &self,
        command_list_type: CommandListType,
        command_allocator: &CommandAllocator,
        initial_state: Option<&PipelineState>,
    ) -> DxResult<CommandList> {
        let mut hw_command_list: *mut ID3D12GraphicsCommandList6 =
            std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateCommandList,
                0,
                command_list_type as i32,
                command_allocator.this,
                match initial_state {
                    Some(state) => state.this,
                    None => std::ptr::null_mut(),
                },
                &IID_ID3D12CommandList,
                cast_to_ppv(&mut hw_command_list)
            )
        }

        Ok(CommandList {
            this: hw_command_list,
        })
    }

    pub fn create_command_queue(
        &self,
        desc: &CommandQueueDesc,
    ) -> DxResult<CommandQueue> {
        let mut hw_queue: *mut ID3D12CommandQueue = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreateCommandQueue,
                &desc.0,
                &IID_ID3D12CommandQueue,
                cast_to_ppv(&mut hw_queue)
            );
        }

        Ok(CommandQueue { this: hw_queue })
    }

    pub fn create_committed_resource(
        &self,
        heap_props: &HeapProperties,
        heap_flags: HeapFlags,
        resource_desc: &ResourceDesc,
        initial_state: ResourceStates,
        optimized_clear_value: Option<&ClearValue>,
    ) -> DxResult<Resource> {
        let mut hw_resource: *mut ID3D12Resource = std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateCommittedResource,
                &heap_props.0,
                heap_flags.bits(),
                &resource_desc.0,
                initial_state.bits(),
                match optimized_clear_value {
                    Some(clear_value) => {
                        &clear_value.0
                    }
                    None => std::ptr::null(),
                },
                &IID_ID3D12Resource,
                cast_to_ppv(&mut hw_resource)
            )
        }

        Ok(Resource { this: hw_resource })
    }

    pub fn create_compute_pipeline_state(
        &self,
        pso_desc: &ComputePipelineStateDesc,
    ) -> DxResult<PipelineState> {
        let mut hw_pipeline_state: *mut ID3D12PipelineState =
            std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreateComputePipelineState,
                &pso_desc.0,
                &IID_ID3D12PipelineState,
                cast_to_ppv(&mut hw_pipeline_state)
            );
        }
        Ok(PipelineState {
            this: hw_pipeline_state,
        })
    }

    pub fn create_constant_buffer_view(
        &self,
        desc: &ConstantBufferViewDesc,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CreateConstantBufferView,
                &desc.0,
                dest_descriptor.hw_handle
            )
        }
    }

    pub fn create_depth_stencil_view(
        &self,
        resource: &Resource,
        desc: &DepthStencilViewDesc,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CreateDepthStencilView,
                resource.this,
                &desc.0,
                dest_descriptor.hw_handle
            )
        }
    }

    pub fn create_descriptor_heap(
        &self,
        desc: &DescriptorHeapDesc,
    ) -> DxResult<DescriptorHeap> {
        let mut hw_descriptor_heap: *mut ID3D12DescriptorHeap =
            std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreateDescriptorHeap,
                &desc.0,
                &IID_ID3D12DescriptorHeap,
                cast_to_ppv(&mut hw_descriptor_heap)
            );
        }
        Ok(DescriptorHeap {
            this: hw_descriptor_heap,
            handle_size: self.get_descriptor_handle_increment_size(unsafe {
                std::mem::transmute(desc.0.Type)
            }),
        })
    }

    pub fn create_fence(
        &self,
        initial_value: u64,
        flags: FenceFlags,
    ) -> DxResult<Fence> {
        let mut hw_fence: *mut ID3D12Fence = std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateFence,
                initial_value,
                flags.bits(),
                &IID_ID3D12Fence,
                cast_to_ppv(&mut hw_fence)
            )
        }

        Ok(Fence { this: hw_fence })
    }

    pub fn create_graphics_pipeline_state(
        &self,
        pso_desc: &GraphicsPipelineStateDesc,
    ) -> DxResult<PipelineState> {
        let mut hw_pipeline_state: *mut ID3D12PipelineState =
            std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreateGraphicsPipelineState,
                &pso_desc.0,
                &IID_ID3D12PipelineState,
                cast_to_ppv(&mut hw_pipeline_state)
            );
        }
        Ok(PipelineState {
            this: hw_pipeline_state,
        })
    }

    pub fn create_heap(&self, heap_desc: &HeapDesc) -> DxResult<Heap> {
        let mut hw_heap: *mut ID3D12Heap = std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateHeap,
                &heap_desc.0,
                &IID_ID3D12Heap,
                cast_to_ppv(&mut hw_heap)
            )
        }

        Ok(Heap { this: hw_heap })
    }

    pub fn create_pipeline_state(
        &self,
        pso_desc: &PipelineStateStreamDesc,
    ) -> DxResult<PipelineState> {
        let mut hw_pipeline_state: *mut ID3D12PipelineState =
            std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreatePipelineState,
                &pso_desc.0,
                &IID_ID3D12PipelineState,
                cast_to_ppv(&mut hw_pipeline_state)
            );
        }
        Ok(PipelineState {
            this: hw_pipeline_state,
        })
    }

    pub fn create_placed_resource(
        &self,
        heap: &Heap,
        heap_offset: Bytes,
        resource_desc: &ResourceDesc,
        initial_state: ResourceStates,
        optimized_clear_value: Option<&ClearValue>,
    ) -> DxResult<Resource> {
        let mut hw_resource: *mut ID3D12Resource = std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreatePlacedResource,
                heap.this,
                heap_offset.0,
                &resource_desc.0,
                initial_state.bits(),
                match optimized_clear_value {
                    Some(clear_value) => {
                        &clear_value.0
                    }
                    None => std::ptr::null(),
                },
                &IID_ID3D12Resource,
                cast_to_ppv(&mut hw_resource)
            )
        }

        Ok(Resource { this: hw_resource })
    }

    pub fn create_query_heap(
        &self,
        heap_desc: &QueryHeapDesc,
    ) -> DxResult<QueryHeap> {
        let mut hw_query_heap: *mut ID3D12QueryHeap = std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateQueryHeap,
                &heap_desc.0 as *const D3D12_QUERY_HEAP_DESC,
                &IID_ID3D12QueryHeap,
                cast_to_ppv(&mut hw_query_heap)
            )
        }

        Ok(QueryHeap {
            this: hw_query_heap,
        })
    }

    pub fn create_render_target_view(
        &self,
        resource: &Resource,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CreateRenderTargetView,
                resource.this,
                std::ptr::null(),
                dest_descriptor.hw_handle
            )
        }
    }

    pub fn create_reserved_resource(
        &self,
        resource_desc: &ResourceDesc,
        initial_state: ResourceStates,
        optimized_clear_value: Option<&ClearValue>,
    ) -> DxResult<Resource> {
        let mut hw_resource: *mut ID3D12Resource = std::ptr::null_mut();

        unsafe {
            dx_try!(
                self.this,
                CreateReservedResource,
                &resource_desc.0,
                initial_state.bits(),
                match optimized_clear_value {
                    Some(clear_value) => {
                        &clear_value.0
                    }
                    None => std::ptr::null(),
                },
                &IID_ID3D12Resource,
                cast_to_ppv(&mut hw_resource)
            )
        }

        Ok(Resource { this: hw_resource })
    }

    pub fn create_root_signature(
        &self,
        node_mask: UINT,
        bytecode: &ShaderBytecode,
    ) -> DxResult<RootSignature> {
        let mut hw_root_signature: *mut ID3D12RootSignature =
            std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                CreateRootSignature,
                node_mask,
                bytecode.0.pShaderBytecode,
                bytecode.0.BytecodeLength,
                &IID_ID3D12RootSignature,
                cast_to_ppv(&mut hw_root_signature)
            );
        }
        Ok(RootSignature {
            this: hw_root_signature,
        })
    }

    pub fn create_sampler(
        &self,
        desc: &SamplerDesc,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CreateSampler,
                &desc.0 as *const D3D12_SAMPLER_DESC,
                dest_descriptor.hw_handle
            )
        }
    }

    pub fn create_shader_resource_view(
        &self,
        resource: &Resource,
        desc: Option<&ShaderResourceViewDesc>,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CreateShaderResourceView,
                resource.this,
                match desc {
                    Some(d) => &d.0,
                    None => std::ptr::null(),
                },
                dest_descriptor.hw_handle
            )
        }
    }

    pub fn create_shared_handle(
        &self,
        object: &DeviceChild,
        name: &str,
    ) -> DxResult<Handle> {
        let mut hw_handle = std::ptr::null_mut();
        let hw_device_child = object.this;
        let name = widestring::U16CString::from_str(name)
            .expect("Cannot convert handle name");
        unsafe {
            dx_try!(
                self.this,
                CreateSharedHandle,
                hw_device_child,
                std::ptr::null_mut(),
                0x10000000, // GENERIC_ALL from winnt.h
                name.as_ptr(),
                &mut hw_handle
            );
        }

        Ok(Handle(hw_handle))
    }

    pub fn create_unordered_access_view(
        &self,
        resource: &Resource,
        counter_resource: Option<&Resource>,
        desc: Option<&UnorderedAccessViewDesc>,
        dest_descriptor: CpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CreateUnorderedAccessView,
                resource.this,
                match counter_resource {
                    Some(res) => res.this,
                    None => std::ptr::null_mut(),
                },
                match desc {
                    Some(d) => &d.0,
                    None => std::ptr::null(),
                },
                dest_descriptor.hw_handle
            )
        }
    }

    pub fn get_copyable_footprints(
        &self,
        resource_desc: &ResourceDesc,
        first_subresouce: u32,
        num_subresources: u32,
        base_offset: Bytes,
    ) -> (Vec<PlacedSubresourceFootprint>, Vec<u32>, Vec<Bytes>, Bytes) {
        let mut placed_subresource_footprints: Vec<PlacedSubresourceFootprint> =
            Vec::with_capacity(num_subresources as usize);
        unsafe {
            placed_subresource_footprints.set_len(num_subresources as usize)
        }

        let mut num_rows: Vec<u32> =
            Vec::with_capacity(num_subresources as usize);
        unsafe { num_rows.set_len(num_subresources as usize) }

        let mut row_sizes: Vec<Bytes> =
            Vec::with_capacity(num_subresources as usize);
        unsafe { row_sizes.set_len(num_subresources as usize) }

        let mut total_bytes = 0u64;

        unsafe {
            dx_call!(
                self.this,
                GetCopyableFootprints,
                &resource_desc.0 as *const D3D12_RESOURCE_DESC,
                first_subresouce,
                num_subresources,
                base_offset.0,
                placed_subresource_footprints.as_mut_ptr()
                    as *mut D3D12_PLACED_SUBRESOURCE_FOOTPRINT,
                num_rows.as_mut_ptr(),
                row_sizes.as_mut_ptr() as *mut u64,
                &mut total_bytes
            )
        }

        (
            placed_subresource_footprints,
            num_rows,
            row_sizes,
            Bytes(total_bytes),
        )
    }

    pub fn get_descriptor_handle_increment_size(
        &self,
        heap_type: DescriptorHeapType,
    ) -> u32 {
        unsafe {
            dx_call!(
                self.this,
                GetDescriptorHandleIncrementSize,
                heap_type as i32
            )
        }
    }

    pub fn get_device_removed_reason(&self) -> DxError {
        unsafe {
            let result = dx_call!(self.this, GetDeviceRemovedReason,);
            DxError::new("GetDeviceRemovedReason", result)
        }
    }

    pub fn get_resource_allocation_info(
        &self,
        visible_mask: u32,
        resource_descs: &[ResourceDesc],
    ) -> ResourceAllocationInfo {
        let mut hw_allocation_info = D3D12_RESOURCE_ALLOCATION_INFO::default();
        unsafe {
            dx_call!(
                self.this,
                GetResourceAllocationInfo,
                &mut hw_allocation_info,
                visible_mask,
                resource_descs.len() as u32,
                resource_descs.as_ptr() as *const D3D12_RESOURCE_DESC
            );
        }

        ResourceAllocationInfo(hw_allocation_info)
    }

    pub fn new(adapter: &Adapter) -> DxResult<Self> {
        let mut hw_device: *mut ID3D12Device2 = std::ptr::null_mut();
        unsafe {
            dx_try!(D3D12CreateDevice(
                cast_to_iunknown!(adapter.this),
                D3D_FEATURE_LEVEL_D3D_FEATURE_LEVEL_12_0,
                &IID_ID3D12Device2,
                cast_to_ppv(&mut hw_device),
            ));
        }

        Ok(Device { this: hw_device })
    }

    pub fn open_shared_fence_handle(&self, handle: Handle) -> DxResult<Fence> {
        let mut hw_fence = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                OpenSharedHandle,
                handle.0,
                &IID_ID3D12Fence,
                &mut hw_fence
            );
        }

        Ok(Fence {
            this: hw_fence as *mut ID3D12Fence,
        })
    }

    pub fn open_shared_handle_by_name(&self, name: &str) -> DxResult<Handle> {
        let mut hw_handle = std::ptr::null_mut();
        let name = widestring::U16CString::from_str(name)
            .expect("Cannot convert handle name");
        unsafe {
            dx_try!(
                self.this,
                OpenSharedHandleByName,
                name.as_ptr(),
                0x10000000, // GENERIC_ALL from winnt.h
                &mut hw_handle
            );
        }

        Ok(Handle(hw_handle))
    }

    pub fn open_shared_heap_handle(&self, handle: Handle) -> DxResult<Heap> {
        let mut hw_heap = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                OpenSharedHandle,
                handle.0,
                &IID_ID3D12Heap,
                &mut hw_heap
            );
        }

        Ok(Heap {
            this: hw_heap as *mut ID3D12Heap,
        })
    }

    pub fn open_shared_resource_handle(
        &self,
        handle: Handle,
    ) -> DxResult<Resource> {
        let mut hw_resource = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                OpenSharedHandle,
                handle.0,
                &IID_ID3D12Resource,
                &mut hw_resource
            );
        }

        Ok(Resource {
            this: hw_resource as *mut ID3D12Resource,
        })
    }
}

#[derive(Debug)]
pub struct DeviceChild {
    pub this: *mut ID3D12DeviceChild,
}
impl_com_object_refcount_unnamed!(DeviceChild);
impl_com_object_clone_drop!(DeviceChild);

impl From<Heap> for DeviceChild {
    fn from(heap: Heap) -> Self {
        let hw_ptr: *mut ID3D12DeviceChild =
            heap.this as *mut ID3D12DeviceChild;
        unsafe { dx_call!(hw_ptr, AddRef,) };

        Self { this: hw_ptr }
    }
}

impl From<Resource> for DeviceChild {
    fn from(heap: Resource) -> Self {
        let hw_ptr: *mut ID3D12DeviceChild =
            heap.this as *mut ID3D12DeviceChild;
        unsafe { dx_call!(hw_ptr, AddRef,) };

        Self { this: hw_ptr }
    }
}

impl From<Fence> for DeviceChild {
    fn from(heap: Fence) -> Self {
        let hw_ptr: *mut ID3D12DeviceChild =
            heap.this as *mut ID3D12DeviceChild;
        unsafe { dx_call!(hw_ptr, AddRef,) };

        Self { this: hw_ptr }
    }
}

#[derive(Debug)]
pub struct CommandQueue {
    pub this: *mut ID3D12CommandQueue,
}
impl_com_object_refcount_unnamed!(CommandQueue);
impl_com_object_clone_drop!(CommandQueue);

unsafe impl Send for CommandQueue {}

impl CommandQueue {
    pub fn execute_command_lists(&self, command_lists: &[CommandList]) {
        unsafe {
            dx_call!(
                self.this,
                ExecuteCommandLists,
                command_lists.len() as std::os::raw::c_uint,
                command_lists.as_ptr() as *const *mut ID3D12CommandList
            );
        }
    }

    pub fn get_timestamp_frequency(&self) -> DxResult<u64> {
        let mut frequency = 0u64;
        unsafe {
            dx_try!(self.this, GetTimestampFrequency, &mut frequency);

            Ok(frequency)
        }
    }

    pub fn signal(&self, fence: &Fence, value: u64) -> DxResult<()> {
        unsafe { dx_try!(self.this, Signal, fence.this, value) };
        Ok(())
    }

    pub fn wait(&self, fence: &Fence, value: u64) -> DxResult<()> {
        unsafe { dx_try!(self.this, Wait, fence.this, value) };
        Ok(())
    }
}

#[derive(Debug)]
pub struct Swapchain {
    pub this: *mut IDXGISwapChain4,
}
impl_com_object_refcount_unnamed!(Swapchain);
impl_com_object_clone_drop!(Swapchain);

impl Swapchain {
    pub fn get_buffer(&self, index: u32) -> DxResult<Resource> {
        let mut buffer: *mut ID3D12Resource = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                GetBuffer,
                index,
                &IID_ID3D12Resource,
                cast_to_ppv(&mut buffer)
            )
        }

        Ok(Resource { this: buffer })
    }

    pub fn get_frame_latency_waitable_object(&self) -> Win32Event {
        Win32Event {
            handle: unsafe {
                dx_call!(self.this, GetFrameLatencyWaitableObject,)
            },
        }
    }

    pub fn get_current_back_buffer_index(&self) -> u32 {
        unsafe { dx_call!(self.this, GetCurrentBackBufferIndex,) }
    }

    pub fn present(
        &self,
        sync_interval: u32,
        flags: PresentFlags,
    ) -> DxResult<()> {
        unsafe { dx_try!(self.this, Present, sync_interval, flags.bits()) };
        Ok(())
    }
}

#[derive(Debug)]
pub struct DescriptorHeap {
    pub this: *mut ID3D12DescriptorHeap,
    handle_size: u32, // it could be Bytes, but the latter is 64-bit, and
                      // since it doesn't leak into public interface, there isn't much sense in it
}

impl_com_object_set_get_name!(DescriptorHeap, handle_size);
impl_com_object_refcount_unnamed!(DescriptorHeap, handle_size);
impl_com_object_clone_drop!(DescriptorHeap, handle_size);

unsafe impl Send for DescriptorHeap {}

impl DescriptorHeap {
    pub fn get_cpu_descriptor_handle_for_heap_start(
        &self,
    ) -> CpuDescriptorHandle {
        let mut hw_handle = D3D12_CPU_DESCRIPTOR_HANDLE { ptr: 0 };
        unsafe {
            dx_call!(
                self.this,
                GetCPUDescriptorHandleForHeapStart,
                &mut hw_handle
            );
        }
        CpuDescriptorHandle {
            hw_handle,
            handle_size: self.handle_size,
        }
    }

    pub fn get_gpu_descriptor_handle_for_heap_start(
        &self,
    ) -> GpuDescriptorHandle {
        let mut hw_handle = D3D12_GPU_DESCRIPTOR_HANDLE { ptr: 0 };
        unsafe {
            dx_call!(
                self.this,
                GetGPUDescriptorHandleForHeapStart,
                &mut hw_handle
            );
        }
        GpuDescriptorHandle {
            hw_handle,
            handle_size: self.handle_size,
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct CpuDescriptorHandle {
    pub hw_handle: D3D12_CPU_DESCRIPTOR_HANDLE,
    pub handle_size: u32,
}

impl CpuDescriptorHandle {
    #[must_use]
    pub fn advance(self, distance: u32) -> Self {
        CpuDescriptorHandle {
            hw_handle: D3D12_CPU_DESCRIPTOR_HANDLE {
                ptr: self.hw_handle.ptr + (distance * self.handle_size) as u64,
            },
            handle_size: self.handle_size,
        }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct GpuDescriptorHandle {
    pub hw_handle: D3D12_GPU_DESCRIPTOR_HANDLE,
    pub handle_size: u32,
}

impl GpuDescriptorHandle {
    pub fn advance(self, distance: u32) -> Self {
        GpuDescriptorHandle {
            hw_handle: D3D12_GPU_DESCRIPTOR_HANDLE {
                ptr: self.hw_handle.ptr + (distance * self.handle_size) as u64,
            },
            handle_size: self.handle_size,
        }
    }
}

#[derive(Debug)]
pub struct Resource {
    pub this: *mut ID3D12Resource,
}
impl_com_object_clone_drop!(Resource);
impl_com_object_refcount_named!(Resource);
impl_com_object_set_get_name!(Resource);

unsafe impl Send for Resource {}

impl Resource {
    pub fn get_desc(&self) -> ResourceDesc {
        unsafe {
            let mut hw_desc: D3D12_RESOURCE_DESC = std::mem::zeroed();
            dx_call!(self.this, GetDesc, &mut hw_desc);
            ResourceDesc(hw_desc)
        }
    }

    pub fn get_device(&self) -> DxResult<Device> {
        let mut hw_device: *mut ID3D12Device2 = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                GetDevice,
                &IID_ID3D12Device2,
                cast_to_ppv(&mut hw_device)
            );
        }
        Ok(Device { this: hw_device })
    }

    pub fn get_gpu_virtual_address(&self) -> GpuVirtualAddress {
        unsafe { GpuVirtualAddress(dx_call!(self.this, GetGPUVirtualAddress,)) }
    }

    // from d3dx12.h
    pub fn get_required_intermediate_size(
        &self,
        first_subresouce: u32,
        num_subresources: u32,
    ) -> DxResult<Bytes> {
        let resource_desc = self.get_desc();

        let device = self.get_device()?;
        let (_, _, _, total_size) = device.get_copyable_footprints(
            &resource_desc,
            first_subresouce,
            num_subresources,
            Bytes(0),
        );
        device.release();

        Ok(total_size)
    }

    pub fn map(
        &self,
        subresource: u32,
        range: Option<&Range>,
    ) -> DxResult<*mut u8> {
        let mut data: *mut u8 = std::ptr::null_mut();
        unsafe {
            dx_try!(
                self.this,
                Map,
                subresource,
                match range {
                    Some(rng) => &rng.0,
                    None => std::ptr::null(),
                },
                cast_to_ppv(&mut data)
            )
        };
        Ok(data)
    }

    pub fn unmap(&self, subresource: UINT, range: Option<&Range>) {
        unsafe {
            dx_call!(
                self.this,
                Unmap,
                subresource,
                match range {
                    Some(rng) => &rng.0,
                    None => std::ptr::null(),
                }
            )
        }
    }
}

#[derive(Debug)]
pub struct CommandAllocator {
    pub this: *mut ID3D12CommandAllocator,
}
impl_com_object_set_get_name!(CommandAllocator);
impl_com_object_refcount_named!(CommandAllocator);
impl_com_object_clone_drop!(CommandAllocator);

impl CommandAllocator {
    pub fn reset(&self) -> DxResult<()> {
        unsafe { dx_try!(self.this, Reset,) };
        Ok(())
    }
}

assert_eq_size!(CommandList, *mut ID3D12GraphicsCommandList6);

#[derive(Debug)]
pub struct CommandList {
    pub this: *mut ID3D12GraphicsCommandList6,
}
impl_com_object_set_get_name!(CommandList);
impl_com_object_refcount_named!(CommandList);
impl_com_object_clone_drop!(CommandList);

impl CommandList {
    pub fn begin_query(
        &self,
        query_heap: &QueryHeap,
        query_type: QueryType,
        index: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                BeginQuery,
                query_heap.this,
                query_type as i32,
                index
            );
        }
    }

    pub fn clear_depth_stencil_view(
        &self,
        descriptor: CpuDescriptorHandle,
        clear_flags: ClearFlags,
        depth: f32,
        stencil: u8,
        rects: &[Rect],
    ) {
        unsafe {
            dx_call!(
                self.this,
                ClearDepthStencilView,
                descriptor.hw_handle,
                clear_flags.bits(),
                depth,
                stencil,
                rects.len() as u32,
                rects.as_ptr() as *const D3D12_RECT
            )
        }
    }

    pub fn clear_render_target_view(
        &self,
        descriptor: CpuDescriptorHandle,
        color: [f32; 4],
        rects: &[Rect],
    ) {
        unsafe {
            dx_call!(
                self.this,
                ClearRenderTargetView,
                descriptor.hw_handle,
                color.as_ptr(),
                rects.len() as u32,
                rects.as_ptr() as *const D3D12_RECT
            )
        }
    }

    pub fn close(&self) -> DxResult<()> {
        unsafe { dx_try!(self.this, Close,) };
        Ok(())
    }

    pub fn copy_buffer_region(
        &self,
        dest: &Resource,
        dest_offset: Bytes,
        source: &Resource,
        source_offset: Bytes,
        span: Bytes,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CopyBufferRegion,
                dest.this,
                dest_offset.0,
                source.this,
                source_offset.0,
                span.0 as u64
            );
        }
    }

    pub fn copy_resource(&self, dest: &Resource, source: &Resource) {
        unsafe { dx_call!(self.this, CopyResource, dest.this, source.this) }
    }

    pub fn copy_texture_region(
        &self,
        dest_location: TextureCopyLocation,
        dest_x: u32,
        dest_y: u32,
        dest_z: u32,
        source_location: TextureCopyLocation,
        source_box: Option<&Box>,
    ) {
        unsafe {
            dx_call!(
                self.this,
                CopyTextureRegion,
                &dest_location.0,
                dest_x,
                dest_y,
                dest_z,
                &source_location.0,
                match source_box {
                    Some(b) => &b.0,
                    None => std::ptr::null_mut(),
                }
            )
        }
    }

    pub fn dispatch(
        &self,
        thread_group_count_x: u32,
        thread_group_count_y: u32,
        thread_group_count_z: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                Dispatch,
                thread_group_count_x,
                thread_group_count_y,
                thread_group_count_z
            )
        }
    }

    pub fn dispatch_mesh(
        &self,
        thread_group_count_x: u32,
        thread_group_count_y: u32,
        thread_group_count_z: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                DispatchMesh,
                thread_group_count_x,
                thread_group_count_y,
                thread_group_count_z
            )
        }
    }

    pub fn draw_indexed_instanced(
        &self,
        index_count_per_instance: u32,
        instance_count: u32,
        start_index_location: u32,
        base_vertex_location: i32,
        start_instance_location: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                DrawIndexedInstanced,
                index_count_per_instance,
                instance_count,
                start_index_location,
                base_vertex_location,
                start_instance_location
            )
        }
    }

    pub fn draw_instanced(
        &self,
        vertex_count_per_instance: u32,
        instance_count: u32,
        start_vertex_location: u32,
        start_instance_location: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                DrawInstanced,
                vertex_count_per_instance,
                instance_count,
                start_vertex_location,
                start_instance_location
            )
        }
    }

    pub fn end_query(
        &self,
        query_heap: &QueryHeap,
        query_type: QueryType,
        index: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                EndQuery,
                query_heap.this,
                query_type as i32,
                index
            );
        }
    }

    pub fn execute_bundle(&self, command_list: &CommandList) {
        unsafe {
            dx_call!(
                self.this,
                ExecuteBundle,
                // ToDo: is it 100% safe?
                command_list.this as *mut ID3D12GraphicsCommandList
            );
        }
    }

    pub fn reset(
        &self,
        command_allocator: &CommandAllocator,
        pipeline_state: Option<&PipelineState>,
    ) -> DxResult<()> {
        unsafe {
            dx_try!(
                self.this,
                Reset,
                command_allocator.this,
                match pipeline_state {
                    Some(pso) => pso.this,
                    None => std::ptr::null_mut(),
                }
            )
        };
        Ok(())
    }

    pub fn resolve_query_data(
        &self,
        query_heap: &QueryHeap,
        query_type: QueryType,
        start_index: u32,
        num_queries: u32,
        destination_buffer: &Resource,
        aligned_destination_buffer_offset: Bytes,
    ) {
        unsafe {
            dx_call!(
                self.this,
                ResolveQueryData,
                query_heap.this,
                query_type as i32,
                start_index,
                num_queries,
                destination_buffer.this,
                aligned_destination_buffer_offset.0
            );
        }
    }

    pub fn resource_barrier(&self, barriers: &[ResourceBarrier]) {
        unsafe {
            dx_call!(
                self.this,
                ResourceBarrier,
                barriers.len() as std::os::raw::c_uint,
                barriers.as_ptr() as *const D3D12_RESOURCE_BARRIER
            );
        }
    }

    pub fn set_blend_factor(&self, blend_factor: [f32; 4]) {
        unsafe { dx_call!(self.this, OMSetBlendFactor, blend_factor.as_ptr()) }
    }

    pub fn set_compute_root_32bit_constant(
        &self,
        root_parameter_index: u32,
        src_data: u32,
        dest_offset: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetComputeRoot32BitConstant,
                root_parameter_index,
                src_data,
                dest_offset
            )
        }
    }

    pub fn set_compute_root_32bit_constants(
        &self,
        root_parameter_index: u32,
        src_data: &[u32],
        dest_offset: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetComputeRoot32BitConstants,
                root_parameter_index,
                src_data.len() as u32,
                src_data.as_ptr() as *const std::ffi::c_void,
                dest_offset
            )
        }
    }

    pub fn set_compute_root_constant_buffer_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetComputeRootConstantBufferView,
                root_parameter_index,
                buffer_location.0
            )
        }
    }

    pub fn set_compute_root_descriptor_table(
        &self,
        parameter_index: u32,
        base_descriptor: GpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetComputeRootDescriptorTable,
                parameter_index,
                base_descriptor.hw_handle
            )
        }
    }

    pub fn set_compute_root_shader_resource_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetComputeRootShaderResourceView,
                root_parameter_index,
                buffer_location.0
            )
        }
    }

    pub fn set_compute_root_signature(&self, root_signature: &RootSignature) {
        unsafe {
            dx_call!(self.this, SetComputeRootSignature, root_signature.this)
        }
    }

    pub fn set_compute_root_unordered_access_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetComputeRootUnorderedAccessView,
                root_parameter_index,
                buffer_location.0
            )
        }
    }

    pub fn set_descriptor_heaps(&self, heaps: &[DescriptorHeap]) {
        // since DescriptorHeap object is not just a wrapper around
        // the correspondent COM pointer but also contains another member,
        // we cannot just pass an array of DescriptorHeap's where
        // an array of ID3D12DescriptorHeap's is required
        // one could argue this smells, but it is really convenient
        // to store descriptor size inside descriptor heap object

        const MAX_HEAP_COUNT: usize = 2;
        assert!(
            heaps.len() <= MAX_HEAP_COUNT,
            "Cannot set more than 2 descriptor heaps"
        );

        let mut hw_heaps = [std::ptr::null_mut(); MAX_HEAP_COUNT];
        for i in 0..heaps.len() {
            hw_heaps[i] = heaps[i].this;
        }

        unsafe {
            dx_call!(
                self.this,
                SetDescriptorHeaps,
                heaps.len() as std::os::raw::c_uint,
                hw_heaps.as_mut_ptr() as *const *mut ID3D12DescriptorHeap
            )
        }
    }

    pub fn set_graphics_root_32bit_constant(
        &self,
        root_parameter_index: u32,
        src_data: u32,
        dest_offset: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetGraphicsRoot32BitConstant,
                root_parameter_index,
                src_data,
                dest_offset
            )
        }
    }

    pub fn set_graphics_root_32bit_constants(
        &self,
        root_parameter_index: u32,
        src_data: &[u32],
        dest_offset: u32,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetGraphicsRoot32BitConstants,
                root_parameter_index,
                src_data.len() as u32,
                src_data.as_ptr() as *const std::ffi::c_void,
                dest_offset
            )
        }
    }

    pub fn set_graphics_root_constant_buffer_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetGraphicsRootConstantBufferView,
                root_parameter_index,
                buffer_location.0
            )
        }
    }

    pub fn set_graphics_root_descriptor_table(
        &self,
        parameter_index: u32,
        base_descriptor: GpuDescriptorHandle,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetGraphicsRootDescriptorTable,
                parameter_index,
                base_descriptor.hw_handle
            )
        }
    }

    pub fn set_graphics_root_shader_resource_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetGraphicsRootShaderResourceView,
                root_parameter_index,
                buffer_location.0
            )
        }
    }

    pub fn set_graphics_root_signature(&self, root_signature: &RootSignature) {
        unsafe {
            dx_call!(self.this, SetGraphicsRootSignature, root_signature.this)
        }
    }

    pub fn set_graphics_root_unordered_access_view(
        &self,
        root_parameter_index: u32,
        buffer_location: GpuVirtualAddress,
    ) {
        unsafe {
            dx_call!(
                self.this,
                SetGraphicsRootUnorderedAccessView,
                root_parameter_index,
                buffer_location.0
            )
        }
    }

    pub fn set_index_buffer(&self, view: &IndexBufferView) {
        unsafe { dx_call!(self.this, IASetIndexBuffer, &view.0) }
    }

    pub fn set_pipeline_state(&self, pipeline_state: &PipelineState) {
        unsafe { dx_call!(self.this, SetPipelineState, pipeline_state.this) }
    }

    pub fn set_primitive_topology(&self, topology: PrimitiveTopology) {
        unsafe { dx_call!(self.this, IASetPrimitiveTopology, topology as i32) }
    }

    pub fn set_render_targets(
        &self,
        descriptors: &[CpuDescriptorHandle],
        single_handle_to_descriptor_range: bool,
        depth_stencil: Option<CpuDescriptorHandle>,
    ) {
        // since CPUDescriptorHandle object is not just a wrapper around
        // the correspondent COM pointer but also contains another member,
        // we cannot just pass an array of CPUDescriptorHandle's where
        // an array of D3D12_CPU_DESCRIPTOR_HANDLE's is required
        // one could argue this smells, but it is really convenient
        // to store descriptor size inside descriptor heap handle object
        const MAX_RT_COUNT: usize =
            D3D12_SIMULTANEOUS_RENDER_TARGET_COUNT as usize;

        assert!(
            descriptors.len() <= MAX_RT_COUNT,
            "Cannot set more than {} descriptor heaps",
            MAX_RT_COUNT
        );

        let mut hw_descriptors = [0u64; MAX_RT_COUNT];
        for i in 0..descriptors.len() {
            hw_descriptors[i] = descriptors[i].hw_handle.ptr;
        }

        unsafe {
            dx_call!(
                self.this,
                OMSetRenderTargets,
                descriptors.len() as std::os::raw::c_uint,
                hw_descriptors.as_mut_ptr() as *mut D3D12_CPU_DESCRIPTOR_HANDLE,
                match single_handle_to_descriptor_range {
                    true => 1,
                    false => 0,
                },
                match depth_stencil {
                    Some(ref depth_desc) => &depth_desc.hw_handle,
                    None => std::ptr::null_mut(),
                }
            )
        }
    }

    pub fn set_scissor_rects(&self, scissors: &[Rect]) {
        unsafe {
            dx_call!(
                self.this,
                RSSetScissorRects,
                scissors.len() as std::os::raw::c_uint,
                scissors.as_ptr() as *const D3D12_RECT
            );
        }
    }

    pub fn set_vertex_buffers(
        &self,
        start_slot: u32,
        views: &[VertexBufferView],
    ) {
        unsafe {
            dx_call!(
                self.this,
                IASetVertexBuffers,
                start_slot,
                views.len() as UINT,
                views.as_ptr() as *const D3D12_VERTEX_BUFFER_VIEW
            )
        }
    }

    pub fn set_viewports(&self, viewports: &[Viewport]) {
        unsafe {
            dx_call!(
                self.this,
                RSSetViewports,
                viewports.len() as std::os::raw::c_uint,
                viewports.as_ptr() as *const D3D12_VIEWPORT
            );
        }
    }

    // d3dx12.h helper
    #[allow(clippy::too_many_arguments)]
    pub fn update_subresources(
        &self,
        destination_resource: &Resource,
        intermediate_resource: &Resource,
        first_subresouce: u32,
        num_subresources: u32,
        required_size: Bytes,
        layouts: &[PlacedSubresourceFootprint],
        num_rows: &[u32],
        row_sizes_in_bytes: &[Bytes],
        source_data: &[SubresourceData],
    ) -> DxResult<Bytes> {
        // ToDo: implement validation as in the original function

        let data = intermediate_resource.map(0, None)?;

        unsafe {
            for i in 0..num_subresources as usize {
                let dest_data = D3D12_MEMCPY_DEST {
                    pData: data.offset(layouts[i].0.Offset as isize)
                        as *mut std::ffi::c_void,
                    RowPitch: layouts[i].0.Footprint.RowPitch as u64,
                    SlicePitch: (layouts[i].0.Footprint.RowPitch as u64)
                        * num_rows[i] as u64,
                };

                memcpy_subresource(
                    &dest_data,
                    &source_data[i].0,
                    row_sizes_in_bytes[i],
                    num_rows[i],
                    layouts[i].0.Footprint.Depth,
                );
            }
        }
        intermediate_resource.unmap(0, None);

        let destination_desc = destination_resource.get_desc();
        if destination_desc.0.Dimension == ResourceDimension::Buffer as i32 {
            self.copy_buffer_region(
                destination_resource,
                Bytes(0),
                intermediate_resource,
                Bytes(layouts[0].0.Offset),
                Bytes(layouts[0].0.Footprint.Width as u64),
            );
        } else {
            for i in 0..num_subresources as usize {
                let dest_location = TextureCopyLocation::new_subresource_index(
                    destination_resource,
                    i as u32 + first_subresouce,
                );
                let source_location = TextureCopyLocation::new_placed_footprint(
                    intermediate_resource,
                    layouts[i],
                );

                self.copy_texture_region(
                    dest_location,
                    0,
                    0,
                    0,
                    source_location,
                    None,
                );
            }
        }

        Ok(required_size)
    }

    // The stack-allocating version cannot be implemented without changing
    // function signature since it would require function output parameters
    pub fn update_subresources_heap_alloc(
        &self,
        destination_resource: &Resource,
        intermediate_resource: &Resource,
        intermediate_offset: Bytes,
        first_subresouce: u32,
        num_subresources: u32,
        source_data: &[SubresourceData],
    ) -> DxResult<Bytes> {
        let allocation_size = Bytes::from(
            std::mem::size_of::<PlacedSubresourceFootprint>()
                + std::mem::size_of::<u32>()
                + std::mem::size_of::<u64>(),
        ) * num_subresources;

        let mut allocated_memory: Vec<u8> =
            Vec::with_capacity(allocation_size.0 as usize);
        unsafe {
            allocated_memory.set_len(allocation_size.0 as usize);

            // let allocation =
            //     allocated_memory.as_mut_ptr() as *mut std::ffi::c_void;

            let destination_desc = destination_resource.get_desc();
            let device = destination_resource.get_device()?;
            let (layouts, num_rows, row_sizes_in_bytes, required_size) = device
                .get_copyable_footprints(
                    &destination_desc,
                    first_subresouce,
                    num_subresources,
                    intermediate_offset,
                );
            self.update_subresources(
                destination_resource,
                intermediate_resource,
                first_subresouce,
                num_subresources,
                required_size,
                &layouts,
                &num_rows,
                &row_sizes_in_bytes,
                source_data,
            )
        }
    }
}

// this function should not leak to the public API, so
// there is no point in using struct wrappers
unsafe fn memcpy_subresource(
    dest: &D3D12_MEMCPY_DEST,
    src: &D3D12_SUBRESOURCE_DATA,
    row_sizes_in_bytes: Bytes,
    num_rows: u32,
    num_slices: u32,
) {
    for z in 0..num_slices {
        let dest_slice =
            dest.pData.offset((dest.SlicePitch * z as u64) as isize);
        let src_slice = src.pData.offset((src.SlicePitch * z as i64) as isize);

        for y in 0..num_rows {
            std::ptr::copy_nonoverlapping(
                src_slice.offset((src.RowPitch * y as i64) as isize),
                dest_slice.offset((dest.RowPitch * y as u64) as isize),
                row_sizes_in_bytes.0 as usize,
            );
        }
    }
}

#[derive(Debug)]
pub struct Fence {
    pub this: *mut ID3D12Fence,
}

impl_com_object_set_get_name!(Fence);
impl_com_object_refcount_named!(Fence);
impl_com_object_clone_drop!(Fence);

// ToDo: make sure ID3D12Fence is thread-safe
unsafe impl Send for Fence {}

impl Fence {
    pub fn get_completed_value(&self) -> u64 {
        unsafe { dx_call!(self.this, GetCompletedValue,) }
    }

    pub fn set_event_on_completion(
        &self,
        value: u64,
        event: &Win32Event,
    ) -> DxResult<()> {
        unsafe {
            dx_try!(self.this, SetEventOnCompletion, value, event.handle);
        }
        Ok(())
    }

    pub fn signal(&self, value: u64) -> DxResult<()> {
        unsafe { dx_try!(self.this, Signal, value) }
        Ok(())
    }
}

#[derive(Debug)]
pub struct Win32Event {
    pub handle: HANDLE,
}

unsafe impl Send for Win32Event {}

impl Default for Win32Event {
    fn default() -> Self {
        unsafe {
            Win32Event {
                handle: CreateEventW(
                    std::ptr::null_mut(),
                    0,
                    0,
                    std::ptr::null(),
                ),
            }
        }
    }
}

impl Win32Event {
    pub fn wait(&self, milliseconds: Option<u32>) {
        unsafe {
            WaitForSingleObject(
                self.handle,
                milliseconds.unwrap_or(0xFFFFFFFF),
            );
        }
    }

    pub fn close(&self) {
        unsafe {
            CloseHandle(self.handle);
        }
    }
}

#[derive(Copy, Clone, Debug)]
#[repr(transparent)]
pub struct Handle(pub HANDLE);

impl Handle {
    // ToDo: accept self by value?
    pub fn close(&self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}

#[derive(Debug)]
pub struct RootSignature {
    pub this: *mut ID3D12RootSignature,
}

impl_com_object_set_get_name!(RootSignature);
impl_com_object_refcount_named!(RootSignature);
impl_com_object_clone_drop!(RootSignature);

unsafe impl Send for RootSignature {}

impl RootSignature {
    // ToDo: rename this function or move it elsewhere?
    pub fn serialize_versioned(
        desc: &VersionedRootSignatureDesc,
    ) -> (Blob, DxResult<()>) {
        let mut blob: *mut ID3DBlob = std::ptr::null_mut();
        let mut error_blob: *mut ID3DBlob = std::ptr::null_mut();
        unsafe {
            let ret_code = D3D12SerializeVersionedRootSignature(
                &desc.0,
                &mut blob,
                &mut error_blob,
            );

            if success!(ret_code) {
                (Blob { this: blob }, Ok(()))
            } else {
                (
                    Blob { this: error_blob },
                    Err(DxError::new(
                        "D3D12SerializeVersionedRootSignature",
                        ret_code,
                    )),
                )
            }
        }
    }
}

#[derive(Debug)]
pub struct PipelineState {
    pub this: *mut ID3D12PipelineState,
}
impl_com_object_set_get_name!(PipelineState);
impl_com_object_refcount_named!(PipelineState);
impl_com_object_clone_drop!(PipelineState);

unsafe impl Send for PipelineState {}

#[derive(Debug)]
pub struct Blob {
    pub this: *mut ID3DBlob,
}
impl_com_object_refcount_unnamed!(Blob);
impl_com_object_clone_drop!(Blob);

impl Blob {
    pub fn get_buffer(&self) -> &[u8] {
        unsafe {
            let buffer_pointer: *mut u8 =
                dx_call!(self.this, GetBufferPointer,) as *mut u8;
            let buffer_size: Bytes = Bytes(dx_call!(self.this, GetBufferSize,));
            std::slice::from_raw_parts(buffer_pointer, buffer_size.0 as usize)
        }
    }
}

#[derive(Debug)]
pub struct QueryHeap {
    pub this: *mut ID3D12QueryHeap,
}
impl_com_object_set_get_name!(QueryHeap);
impl_com_object_refcount_named!(QueryHeap);
impl_com_object_clone_drop!(QueryHeap);

#[derive(Debug)]
pub struct Heap {
    pub this: *mut ID3D12Heap,
}
impl_com_object_set_get_name!(Heap);
impl_com_object_refcount_named!(Heap);
impl_com_object_clone_drop!(Heap);

pub struct PIXSupport {}

impl PIXSupport {
    pub fn init() {
        #[cfg(feature = "pix")]
        unsafe {
            raw_bindings::pix::pix_init_analysis();
        }
    }

    pub fn shutdown() {
        #[cfg(feature = "pix")]
        unsafe {
            raw_bindings::pix::pix_shutdown_analysis();
        }
    }

    pub fn begin_capture() {
        #[cfg(feature = "pix")]
        unsafe {
            raw_bindings::pix::pix_begin_capture();
        }
    }

    pub fn end_capture() {
        #[cfg(feature = "pix")]
        unsafe {
            raw_bindings::pix::pix_end_capture();
        }
    }

    pub fn begin_event_cmd_list(
        cmd_list: &CommandList,
        marker: &str,
        color: u64,
    ) {
        #[cfg(feature = "pix")]
        unsafe {
            // ToDo: allocation on every marker call is sad :(
            let marker = CString::new(marker)
                .expect("Cannot convert marker string to C string");
            raw_bindings::pix::pix_begin_event_cmd_list(
                cmd_list.this
                    as *mut raw_bindings::pix::ID3D12GraphicsCommandList6,
                color,
                marker.as_ptr() as *const i8,
            );
        }
    }

    pub fn end_event_cmd_list(cmd_list: &CommandList) {
        #[cfg(feature = "pix")]
        unsafe {
            raw_bindings::pix::pix_end_event_cmd_list(
                cmd_list.this
                    as *mut raw_bindings::pix::ID3D12GraphicsCommandList6,
            );
        }
    }

    pub fn begin_event_cmd_queue(
        cmd_queue: &CommandQueue,
        marker: &str,
        color: u64,
    ) {
        #[cfg(feature = "pix")]
        unsafe {
            let marker = CString::new(marker)
                .expect("Cannot convert marker string to C string");
            raw_bindings::pix::pix_begin_event_cmd_queue(
                cmd_queue.this as *mut raw_bindings::pix::ID3D12CommandQueue,
                color,
                marker.as_ptr() as *const i8,
            );
        }
    }

    pub fn end_event_cmd_queue(cmd_queue: &CommandQueue) {
        #[cfg(feature = "pix")]
        unsafe {
            raw_bindings::pix::pix_end_event_cmd_queue(
                cmd_queue.this as *mut raw_bindings::pix::ID3D12CommandQueue,
            );
        }
    }
}
