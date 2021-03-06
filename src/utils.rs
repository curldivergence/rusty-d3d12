use log::{debug, error, info, trace, warn};
use winapi::shared::winerror;

use crate::{DXError, DxResult};

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

macro_rules! impl_mul {
    ($struct_type:ty, $integer_type:ty) => {
        impl std::ops::Mul<$integer_type> for $struct_type {
            type Output = Self;

            fn mul(self, rhs: $integer_type) -> Self {
                Self(self.0 * rhs as u64)
            }
        }
    };
}

// ToDo: let's see how it goes with just two types and if we should go back
// to *Count/*Offset pairs
/// Bytes

#[derive(Copy, Clone, Debug)]
pub struct Bytes(pub u64);

// Bytes + Bytes = Bytes
impl std::ops::Add<Bytes> for Bytes {
    type Output = Self;

    fn add(self, rhs: Bytes) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<Bytes> for Bytes {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0 + rhs.0);
    }
}

impl_mul!(Bytes, u8);
impl_mul!(Bytes, i8);
impl_mul!(Bytes, u16);
impl_mul!(Bytes, i16);
impl_mul!(Bytes, u32);
impl_mul!(Bytes, i32);
impl_mul!(Bytes, u64);
impl_mul!(Bytes, i64);
impl_mul!(Bytes, usize);
impl_mul!(Bytes, isize);

impl std::ops::Mul<Bytes> for usize {
    type Output = Bytes;

    fn mul(self, rhs: Bytes) -> Self::Output {
        Bytes(self as u64 * rhs.0)
    }
}

// Bytes * Elements = Bytes
impl std::ops::Mul<Elements> for Bytes {
    type Output = Self;

    fn mul(self, rhs: Elements) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Into<usize> for Bytes {
    fn into(self) -> usize {
        self.0 as usize
    }
}

impl_from!(Bytes, u8);
impl_from!(Bytes, i8);
impl_from!(Bytes, u16);
impl_from!(Bytes, i16);
impl_from!(Bytes, u32);
impl_from!(Bytes, i32);
impl_from!(Bytes, u64);
impl_from!(Bytes, i64);
impl_from!(Bytes, usize);
impl_from!(Bytes, isize);

/// Elements

#[derive(Copy, Clone, Debug)]
pub struct Elements(pub u64);

// Elements + Elements = Elements
impl std::ops::Add<Elements> for Elements {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

impl std::ops::AddAssign<Elements> for Elements {
    fn add_assign(&mut self, rhs: Self) {
        *self = Self(self.0 + rhs.0);
    }
}

impl_from!(Elements, u8);
impl_from!(Elements, i8);
impl_from!(Elements, u16);
impl_from!(Elements, i16);
impl_from!(Elements, u32);
impl_from!(Elements, i32);
impl_from!(Elements, u64);
impl_from!(Elements, i64);
impl_from!(Elements, usize);
impl_from!(Elements, isize);

impl std::ops::Mul<u64> for Elements {
    type Output = Self;

    fn mul(self, rhs: u64) -> Self::Output {
        Self::from(self.0 * rhs)
    }
}

///

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
            Err(DXError::new(
                "compile_hlsl",
                winapi::shared::winerror::E_FAIL,
            ))
        }
    }
}

///
pub fn align_to_multiple(location: u64, alignment: u64) -> u64 {
    (location + (alignment - 1)) & (!(alignment - 1))
}
