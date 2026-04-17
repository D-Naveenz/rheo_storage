#![deny(missing_docs)]
#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::result_large_err)]

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
mod logging;
mod marshal;
mod models;
mod operations;
mod queries;
mod sessions;
mod watching;

pub use abi::{RheoOperationSnapshot, RheoOperationState, RheoStatus};
pub use logging::*;
pub use operations::*;
pub use queries::*;
pub use sessions::*;
pub use watching::*;

/// Frees a UTF-8 string buffer previously returned by the native ABI.
///
/// # Arguments
///
/// - `ptr` (`*mut u8`) - Pointer to the start of the owned UTF-8 buffer.
/// - `len` (`usize`) - Length of the buffer in bytes.
///
/// # Safety
///
/// `ptr` and `len` must come from a Rheo Storage FFI function that transferred ownership of a
/// heap-allocated UTF-8 buffer to the caller. Passing any other pointer, length, or allocation
/// provenance results in undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_string_free(ptr: *mut u8, len: usize) {
    marshal::free_boxed_bytes(ptr, len);
}

/// Frees a byte buffer previously returned by the native ABI.
///
/// # Arguments
///
/// - `ptr` (`*mut u8`) - Pointer to the start of the owned byte buffer.
/// - `len` (`usize`) - Length of the buffer in bytes.
///
/// # Safety
///
/// `ptr` and `len` must come from a Rheo Storage FFI function that transferred ownership of a
/// heap-allocated byte buffer to the caller. Passing any other pointer, length, or allocation
/// provenance results in undefined behavior.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_bytes_free(ptr: *mut u8, len: usize) {
    marshal::free_boxed_bytes(ptr, len);
}
