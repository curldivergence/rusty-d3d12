#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]

use crate::raw_bindings::*;
use crate::utils::*;
// use crate::{enum_wrappers::*};
// use crate::{struct_wrappers::*};

// ToDo: keep the original name?
pub const CONSTANT_BUFFER_ALIGNMENT: Bytes =
    Bytes(D3D12_CONSTANT_BUFFER_DATA_PLACEMENT_ALIGNMENT as u64);

pub const DEFAULT_RESOURCE_ALIGNMENT: Bytes =
    Bytes(D3D12_DEFAULT_RESOURCE_PLACEMENT_ALIGNMENT as u64);

pub const SIMULTANEOUS_RENDER_TARGET_COUNT: usize = D3D12_SIMULTANEOUS_RENDER_TARGET_COUNT as usize;
