use log::{debug, error, info, trace, warn};

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{DxError, DxResult};

// ToDo: impl iterators

macro_rules! impl_from {
    ($struct_type:ty, $integer_type:ty) => {
        impl From<$integer_type> for $struct_type {
            fn from(value: $integer_type) -> Self {
                Self(value as u64)
            }
        }
    };
}

macro_rules! impl_mul_div {
    ($struct_type:tt, $integer_type:ty) => {
        impl std::ops::Mul<$integer_type> for $struct_type {
            type Output = Self;

            fn mul(self, rhs: $integer_type) -> Self {
                Self(self.0 * rhs as u64)
            }
        }

        impl std::ops::Mul<$struct_type> for $integer_type {
            type Output = $struct_type;

            fn mul(self, rhs: $struct_type) -> Self::Output {
                $struct_type(self as u64 * rhs.0)
            }
        }

        impl std::ops::Div<$integer_type> for $struct_type {
            type Output = Self;

            fn div(self, rhs: $integer_type) -> Self {
                Self(self.0 / rhs as u64)
            }
        }

        impl std::ops::Div<$struct_type> for $integer_type {
            type Output = $struct_type;

            fn div(self, rhs: $struct_type) -> Self::Output {
                $struct_type(self as u64 / rhs.0)
            }
        }
    };
}

// ToDo: get rid of it in favor of usize??
/// A newtype around [u64] made to distinguish between element counts and byte sizes in APIs
#[derive(PartialEq, Eq, Hash, Debug, Copy, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ByteCount(pub u64);

// ByteCount + ByteCount = ByteCount
impl std::ops::Add<ByteCount> for ByteCount {
    type Output = Self;

    fn add(self, rhs: ByteCount) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<ByteCount> for ByteCount {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0 + rhs.0);
    }
}

impl_mul_div!(ByteCount, u8);
impl_mul_div!(ByteCount, i8);
impl_mul_div!(ByteCount, u16);
impl_mul_div!(ByteCount, i16);
impl_mul_div!(ByteCount, u32);
impl_mul_div!(ByteCount, i32);
impl_mul_div!(ByteCount, u64);
impl_mul_div!(ByteCount, i64);
impl_mul_div!(ByteCount, usize);
impl_mul_div!(ByteCount, isize);

// // Bytes * Elements = Bytes
// impl std::ops::Mul<Elements> for Bytes {
//     type Output = Self;

//     fn mul(self, rhs: Elements) -> Self::Output {
//         Self(self.0 * rhs.0)
//     }
// }

impl Into<usize> for ByteCount {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl_from!(ByteCount, u8);
impl_from!(ByteCount, i8);
impl_from!(ByteCount, u16);
impl_from!(ByteCount, i16);
impl_from!(ByteCount, u32);
impl_from!(ByteCount, i32);
impl_from!(ByteCount, u64);
impl_from!(ByteCount, i64);
impl_from!(ByteCount, usize);
impl_from!(ByteCount, isize);

pub fn compile_shader(
    name: &str,
    source: &str,
    entry_point: &str,
    shader_model: &str,
    args: &[&str],
    defines: &[(&str, Option<&str>)],
) -> DxResult<Vec<u8>> {
    let result = hassle_rs::utils::compile_hlsl(
        name,
        source,
        entry_point,
        shader_model,
        // &["/Zi", "/Zss", "/Od"],
        args,
        defines,
    );
    match result {
        Ok(bytecode) => {
            info!("Shader {} compiled successfully", name);
            Ok(bytecode)
        }
        Err(error) => {
            error!("Cannot compile shader: {}", &error);
            Err(DxError::new(
                "compile_hlsl",
                winapi::shared::winerror::E_FAIL,
            ))
        }
    }
}

pub fn align_to_multiple(value: u64, alignment: u64) -> u64 {
    (value + (alignment - 1)) & (!(alignment - 1))
}

/// A macro similar to [std::mem::size_of] function, but returns [ByteCount] instead of [usize]
#[macro_export]
macro_rules! size_of {
    ($struct_type:ty) => {
        ByteCount::from(std::mem::size_of::<$struct_type>())
    };
}
