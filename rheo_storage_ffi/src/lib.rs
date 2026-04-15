#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc, clippy::result_large_err)]

//! Native C ABI wrapper for the Rheo Storage Rust core.
//!
//! The exported surface is intentionally path-based so higher-level bindings
//! such as the .NET package can provide the ergonomic object model.

macro_rules! ffi_fn {
    ($body:expr) => {{
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $body)) {
            Ok(status) => status,
            Err(_) => crate::abi::RheoStatus::Panic,
        }
    }};
}

mod abi;
mod errors;
mod marshal;
mod models;
mod operations;
mod queries;
mod sessions;
mod watching;

pub use abi::{RheoOperationSnapshot, RheoOperationState, RheoStatus};
pub use operations::*;
pub use queries::*;
pub use sessions::*;
pub use watching::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_string_free(ptr: *mut u8, len: usize) {
    marshal::free_boxed_bytes(ptr, len);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_bytes_free(ptr: *mut u8, len: usize) {
    marshal::free_boxed_bytes(ptr, len);
}
