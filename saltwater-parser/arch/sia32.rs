//! Cosmic OS SIA32 C data model.
#![allow(missing_docs)]

#[allow(non_camel_case_types)]
pub type SIZE_T = u64;
#[allow(dead_code)]
pub const SIZE_MAX: SIZE_T = u32::MAX as SIZE_T;

pub const FLOAT_SIZE: u16 = 4;
pub const DOUBLE_SIZE: u16 = 8;

// SIA32 uses the ILP32 C data model.
pub const LONG_SIZE: u16 = 4;
pub const INT_SIZE: u16 = 4;
pub const SHORT_SIZE: u16 = 2;
pub const BOOL_SIZE: u16 = 1;
pub const PTR_SIZE: u16 = 4;

pub const CHAR_BIT: u16 = 8;
