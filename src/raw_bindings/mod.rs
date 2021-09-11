#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(unused_variables)]
#![allow(dead_code)]

#[cfg(feature = "devel")]
pub mod d3d12 {
    include!(concat!(env!("OUT_DIR"), "/d3d12_bindings.rs"));
}

#[cfg(not(feature = "devel"))]
pub mod d3d12;

#[cfg(feature = "devel")]
pub mod pix {
    include!(concat!(env!("OUT_DIR"), "/pix_bindings.rs"));
}

#[cfg(not(feature = "devel"))]
pub mod pix;
