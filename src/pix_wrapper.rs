#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(warnings)]

#[cfg(feature = "pix")]
include!(concat!(env!("OUT_DIR"), "/pix_bindings.rs"));

