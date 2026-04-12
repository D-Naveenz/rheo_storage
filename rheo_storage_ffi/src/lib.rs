#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc, clippy::result_large_err)]

//! Native C ABI wrapper for the Rheo Storage Rust core.
//!
//! The exported surface is intentionally small and path-based so higher-level
//! bindings such as the .NET package can provide the ergonomic object model.

use std::ffi::{CStr, c_char};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::time::{SystemTime, UNIX_EPOCH};

use rheo_storage::{
    AnalysisReport, ContentKind, DetectedDefinition, DirectoryInfo, DirectoryStorage, FileInfo,
    SearchScope, StorageEntry, StorageError, analyze_path, copy_directory, copy_file,
    create_directory, create_directory_all, delete_directory, delete_file, move_directory,
    move_file, read_file, read_file_to_string, rename_directory, rename_file, write_file,
    write_file_string,
};
use serde::Serialize;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RheoStatus {
    Ok = 0,
    Error = 1,
    InvalidArgument = 2,
    Panic = 3,
}

#[derive(Debug, Clone, Copy)]
enum EntryKind {
    Files,
    Directories,
    All,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct ErrorPayload {
    code: &'static str,
    message: String,
    path: Option<String>,
    operation: Option<String>,
    kind: Option<String>,
    value: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct StorageMetadataDto {
    path: String,
    name: String,
    is_read_only: bool,
    is_hidden: bool,
    is_system: bool,
    is_temporary: bool,
    is_symbolic_link: bool,
    link_target: Option<String>,
    created_at_utc_ms: Option<u64>,
    modified_at_utc_ms: Option<u64>,
    accessed_at_utc_ms: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct FileInfoDto {
    metadata: StorageMetadataDto,
    display_name: String,
    size: u64,
    formatted_size: String,
    filename_extension: Option<String>,
    analysis: Option<AnalysisReportDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DirectorySummaryDto {
    total_size: u64,
    file_count: u64,
    directory_count: u64,
    formatted_size: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DirectoryInfoDto {
    metadata: StorageMetadataDto,
    display_name: String,
    summary: Option<DirectorySummaryDto>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct DetectedDefinitionDto {
    file_type_label: String,
    mime_type: String,
    extensions: Vec<String>,
    score: u64,
    confidence: f64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct AnalysisReportDto {
    matches: Vec<DetectedDefinitionDto>,
    top_mime_type: Option<String>,
    top_detected_extension: Option<String>,
    content_kind: ContentKindDto,
    bytes_scanned: usize,
    file_size: u64,
    source_extension: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContentKindDto {
    Text,
    Binary,
    Unknown,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct StorageEntryDto {
    kind: &'static str,
    path: String,
    name: String,
}

#[derive(Debug)]
struct FfiFailure {
    status: RheoStatus,
    payload: ErrorPayload,
}

macro_rules! ffi_fn {
    ($body:expr) => {{
        match catch_unwind(AssertUnwindSafe(|| $body)) {
            Ok(status) => status,
            Err(_) => RheoStatus::Panic,
        }
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_string_free(ptr: *mut u8, len: usize) {
    free_boxed_bytes(ptr, len);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_bytes_free(ptr: *mut u8, len: usize) {
    free_boxed_bytes(ptr, len);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_analyze_path(
    path: *const c_char,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let report = analyze_path(&path).map_err(FfiFailure::from)?;
            Ok(AnalysisReportDto::from(report))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_get_file_info(
    path: *const c_char,
    include_analysis: u8,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let info = if include_analysis != 0 {
                FileInfo::from_path_with_analysis(&path)
            } else {
                FileInfo::from_path(&path)
            }
            .map_err(FfiFailure::from)?;

            FileInfoDto::try_from_info(info, include_analysis != 0)
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_get_directory_info(
    path: *const c_char,
    include_summary: u8,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let info = if include_summary != 0 {
                DirectoryInfo::from_path_with_summary(&path)
            } else {
                DirectoryInfo::from_path(&path)
            }
            .map_err(FfiFailure::from)?;

            DirectoryInfoDto::try_from_info(info, include_summary != 0)
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_list_files(
    path: *const c_char,
    recursive: u8,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            list_entries_json(&path, recursive != 0, EntryKind::Files)
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_list_directories(
    path: *const c_char,
    recursive: u8,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            list_entries_json(&path, recursive != 0, EntryKind::Directories)
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_list_entries(
    path: *const c_char,
    recursive: u8,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            list_entries_json(&path, recursive != 0, EntryKind::All)
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_read_file(
    path: *const c_char,
    out_bytes_ptr: *mut *mut u8,
    out_bytes_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_bytes(
        out_bytes_ptr,
        out_bytes_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            read_file(&path).map_err(FfiFailure::from)
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_read_file_text(
    path: *const c_char,
    out_string_ptr: *mut *mut u8,
    out_string_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_string_ptr,
        out_string_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            read_file_to_string(&path).map_err(FfiFailure::from)
        },
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_file(
    path: *const c_char,
    data_ptr: *const u8,
    data_len: usize,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let bytes = parse_bytes_arg(data_ptr, data_len, "data")?;
            let written = write_file(&path, bytes).map_err(FfiFailure::from)?;
            Ok(path_to_string(&written))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_file_text(
    path: *const c_char,
    text: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let text = parse_string_arg(text, "text")?;
            let written = write_file_string(&path, &text).map_err(FfiFailure::from)?;
            Ok(path_to_string(&written))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_copy_file(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| copy_file(source, destination),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_move_file(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| move_file(source, destination),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_rename_file(
    source: *const c_char,
    new_name: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            let renamed = rename_file(&source, &new_name).map_err(FfiFailure::from)?;
            Ok(path_to_string(&renamed))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_delete_file(
    path: *const c_char,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let path = parse_path_arg(path, "path")?;
        delete_file(&path).map_err(FfiFailure::from)
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_create_directory(
    path: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let created = create_directory(&path).map_err(FfiFailure::from)?;
            Ok(path_to_string(&created))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_create_directory_all(
    path: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let created = create_directory_all(&path).map_err(FfiFailure::from)?;
            Ok(path_to_string(&created))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_copy_directory(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| copy_directory(source, destination),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_move_directory(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_path_operation(
        source,
        destination,
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        |source, destination| move_directory(source, destination),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_rename_directory(
    source: *const c_char,
    new_name: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            let renamed = rename_directory(&source, &new_name).map_err(FfiFailure::from)?;
            Ok(path_to_string(&renamed))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_delete_directory(
    path: *const c_char,
    recursive: u8,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let path = parse_path_arg(path, "path")?;
        if recursive == 0 {
            return Err(FfiFailure::invalid_argument(
                "recursive",
                "delete_directory requires recursive=1 in v1",
            ));
        }

        delete_directory(&path).map_err(FfiFailure::from)
    }))
}

unsafe fn execute_path_operation(
    source: *const c_char,
    destination: *const c_char,
    out_path_ptr: *mut *mut u8,
    out_path_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce(&Path, &Path) -> Result<PathBuf, StorageError>,
) -> RheoStatus {
    execute_string(
        out_path_ptr,
        out_path_len,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            let result = operation(&source, &destination).map_err(FfiFailure::from)?;
            Ok(path_to_string(&result))
        },
    )
}

unsafe fn execute_json<T: Serialize>(
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<T, FfiFailure>,
) -> RheoStatus {
    execute_bytes_common(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || {
            let value = operation()?;
            serde_json::to_vec(&value).map_err(|err| FfiFailure::error(err.to_string()))
        },
    )
}

unsafe fn execute_string(
    out_string_ptr: *mut *mut u8,
    out_string_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<String, FfiFailure>,
) -> RheoStatus {
    execute_bytes_common(
        out_string_ptr,
        out_string_len,
        out_error_ptr,
        out_error_len,
        || operation().map(String::into_bytes),
    )
}

unsafe fn execute_bytes(
    out_bytes_ptr: *mut *mut u8,
    out_bytes_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<Vec<u8>, FfiFailure>,
) -> RheoStatus {
    execute_bytes_common(
        out_bytes_ptr,
        out_bytes_len,
        out_error_ptr,
        out_error_len,
        operation,
    )
}

unsafe fn execute_bytes_common(
    out_ptr: *mut *mut u8,
    out_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<Vec<u8>, FfiFailure>,
) -> RheoStatus {
    if let Err(failure) = validate_buffer_out(out_ptr, out_len) {
        return write_error_only(out_error_ptr, out_error_len, failure);
    }

    if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
        return failure.status;
    }

    reset_buffer_out(out_ptr, out_len);
    reset_buffer_out(out_error_ptr, out_error_len);

    match operation() {
        Ok(bytes) => {
            write_buffer(out_ptr, out_len, bytes);
            RheoStatus::Ok
        }
        Err(failure) => {
            write_error_payload(out_error_ptr, out_error_len, &failure);
            failure.status
        }
    }
}

unsafe fn execute_unit(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<(), FfiFailure>,
) -> RheoStatus {
    if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
        return failure.status;
    }

    reset_buffer_out(out_error_ptr, out_error_len);

    match operation() {
        Ok(()) => RheoStatus::Ok,
        Err(failure) => {
            write_error_payload(out_error_ptr, out_error_len, &failure);
            failure.status
        }
    }
}

unsafe fn validate_buffer_out(
    ptr_out: *mut *mut u8,
    len_out: *mut usize,
) -> Result<(), FfiFailure> {
    if ptr_out.is_null() {
        return Err(FfiFailure::invalid_argument(
            "out_ptr",
            "output pointer argument must not be null",
        ));
    }
    if len_out.is_null() {
        return Err(FfiFailure::invalid_argument(
            "out_len",
            "output length argument must not be null",
        ));
    }
    Ok(())
}

unsafe fn reset_buffer_out(ptr_out: *mut *mut u8, len_out: *mut usize) {
    *ptr_out = ptr::null_mut();
    *len_out = 0;
}

unsafe fn write_buffer(ptr_out: *mut *mut u8, len_out: *mut usize, bytes: Vec<u8>) {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed) as *mut u8;
    *ptr_out = ptr;
    *len_out = len;
}

unsafe fn write_error_only(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    failure: FfiFailure,
) -> RheoStatus {
    if out_error_ptr.is_null() || out_error_len.is_null() {
        return failure.status;
    }

    reset_buffer_out(out_error_ptr, out_error_len);
    write_error_payload(out_error_ptr, out_error_len, &failure);
    failure.status
}

unsafe fn write_error_payload(
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    failure: &FfiFailure,
) {
    let payload = serde_json::to_vec(&failure.payload).unwrap_or_else(|_| {
        br#"{"code":"ffi_error","message":"failed to serialize error"}"#.to_vec()
    });
    write_buffer(out_error_ptr, out_error_len, payload);
}

unsafe fn free_boxed_bytes(ptr: *mut u8, len: usize) {
    if ptr.is_null() {
        return;
    }

    let slice_ptr = ptr::slice_from_raw_parts_mut(ptr, len);
    drop(Box::from_raw(slice_ptr));
}

fn list_entries_json(
    path: &Path,
    recursive: bool,
    kind: EntryKind,
) -> Result<Vec<StorageEntryDto>, FfiFailure> {
    let directory = DirectoryStorage::from_existing(path).map_err(FfiFailure::from)?;
    let scope = if recursive {
        SearchScope::AllDirectories
    } else {
        SearchScope::TopDirectoryOnly
    };

    match kind {
        EntryKind::Files => Ok(directory
            .files_matching("*", scope)
            .map_err(FfiFailure::from)?
            .into_iter()
            .map(|file| StorageEntryDto {
                kind: "file",
                path: path_to_string(file.path()),
                name: file.name().unwrap_or_default().to_owned(),
            })
            .collect()),
        EntryKind::Directories => Ok(directory
            .directories_matching("*", scope)
            .map_err(FfiFailure::from)?
            .into_iter()
            .map(|dir| StorageEntryDto {
                kind: "directory",
                path: path_to_string(dir.path()),
                name: dir.name().unwrap_or_default().to_owned(),
            })
            .collect()),
        EntryKind::All => Ok(directory
            .entries_matching("*", scope)
            .map_err(FfiFailure::from)?
            .into_iter()
            .map(StorageEntryDto::from_entry)
            .collect()),
    }
}

fn parse_path_arg(value: *const c_char, field: &'static str) -> Result<PathBuf, FfiFailure> {
    Ok(PathBuf::from(parse_string_arg(value, field)?))
}

fn parse_string_arg(value: *const c_char, field: &'static str) -> Result<String, FfiFailure> {
    if value.is_null() {
        return Err(FfiFailure::invalid_argument(
            field,
            "string pointer must not be null",
        ));
    }

    let value = unsafe { CStr::from_ptr(value) };
    let bytes = value.to_bytes();
    if bytes.is_empty() {
        return Err(FfiFailure::invalid_argument(
            field,
            "string value must not be empty",
        ));
    }

    String::from_utf8(bytes.to_vec())
        .map_err(|_| FfiFailure::invalid_argument(field, "string value must be valid UTF-8"))
}

fn parse_bytes_arg<'a>(
    data_ptr: *const u8,
    data_len: usize,
    field: &'static str,
) -> Result<&'a [u8], FfiFailure> {
    if data_len == 0 {
        return Ok(&[]);
    }
    if data_ptr.is_null() {
        return Err(FfiFailure::invalid_argument(
            field,
            "byte pointer must not be null",
        ));
    }

    Ok(unsafe { slice::from_raw_parts(data_ptr, data_len) })
}

fn path_to_string(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}

fn system_time_to_unix_millis(value: Option<SystemTime>) -> Option<u64> {
    value
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
}

impl StorageMetadataDto {
    fn from_metadata(metadata: &rheo_storage::StorageMetadata) -> Self {
        Self {
            path: path_to_string(metadata.path()),
            name: metadata.name().to_owned(),
            is_read_only: metadata.is_read_only(),
            is_hidden: metadata.is_hidden(),
            is_system: metadata.is_system(),
            is_temporary: metadata.is_temporary(),
            is_symbolic_link: metadata.is_symbolic_link(),
            link_target: metadata.link_target().map(path_to_string),
            created_at_utc_ms: system_time_to_unix_millis(metadata.created_at()),
            modified_at_utc_ms: system_time_to_unix_millis(metadata.modified_at()),
            accessed_at_utc_ms: system_time_to_unix_millis(metadata.accessed_at()),
        }
    }
}

impl FileInfoDto {
    fn try_from_info(info: FileInfo, include_analysis: bool) -> Result<Self, FfiFailure> {
        let analysis = if include_analysis {
            Some(AnalysisReportDto::from(
                info.analysis().map_err(FfiFailure::from)?.clone(),
            ))
        } else {
            None
        };

        Ok(Self {
            metadata: StorageMetadataDto::from_metadata(info.metadata()),
            display_name: info.display_name().to_owned(),
            size: info.size(),
            formatted_size: info.formatted_size(),
            filename_extension: info.filename_extension().map(ToOwned::to_owned),
            analysis,
        })
    }
}

impl DirectoryInfoDto {
    fn try_from_info(info: DirectoryInfo, include_summary: bool) -> Result<Self, FfiFailure> {
        let summary = if include_summary {
            let summary = *info.summary().map_err(FfiFailure::from)?;
            Some(DirectorySummaryDto {
                total_size: summary.total_size,
                file_count: summary.file_count,
                directory_count: summary.directory_count,
                formatted_size: summary.formatted_size(),
            })
        } else {
            None
        };

        Ok(Self {
            metadata: StorageMetadataDto::from_metadata(info.metadata()),
            display_name: info.display_name().to_owned(),
            summary,
        })
    }
}

impl AnalysisReportDto {
    fn from(value: AnalysisReport) -> Self {
        Self {
            matches: value
                .matches
                .into_iter()
                .map(DetectedDefinitionDto::from)
                .collect(),
            top_mime_type: value.top_mime_type,
            top_detected_extension: value.top_detected_extension,
            content_kind: ContentKindDto::from(value.content_kind),
            bytes_scanned: value.bytes_scanned,
            file_size: value.file_size,
            source_extension: value.source_extension,
        }
    }
}

impl From<DetectedDefinition> for DetectedDefinitionDto {
    fn from(value: DetectedDefinition) -> Self {
        Self {
            file_type_label: value.file_type_label,
            mime_type: value.mime_type,
            extensions: value.extensions,
            score: value.score,
            confidence: value.confidence,
        }
    }
}

impl From<ContentKind> for ContentKindDto {
    fn from(value: ContentKind) -> Self {
        match value {
            ContentKind::Text => Self::Text,
            ContentKind::Binary => Self::Binary,
            ContentKind::Unknown => Self::Unknown,
        }
    }
}

impl StorageEntryDto {
    fn from_entry(entry: StorageEntry) -> Self {
        match entry {
            StorageEntry::File(file) => Self {
                kind: "file",
                path: path_to_string(file.path()),
                name: file.name().unwrap_or_default().to_owned(),
            },
            StorageEntry::Directory(directory) => Self {
                kind: "directory",
                path: path_to_string(directory.path()),
                name: directory.name().unwrap_or_default().to_owned(),
            },
        }
    }
}

impl FfiFailure {
    fn error(message: impl Into<String>) -> Self {
        Self {
            status: RheoStatus::Error,
            payload: ErrorPayload {
                code: "storage_error",
                message: message.into(),
                path: None,
                operation: None,
                kind: None,
                value: None,
            },
        }
    }

    fn invalid_argument(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            status: RheoStatus::InvalidArgument,
            payload: ErrorPayload {
                code: "invalid_argument",
                message: message.into(),
                path: None,
                operation: None,
                kind: Some(field.to_owned()),
                value: None,
            },
        }
    }
}

impl From<StorageError> for FfiFailure {
    fn from(value: StorageError) -> Self {
        match value {
            StorageError::NotFound { path } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "not_found",
                    message: format!("path does not exist: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::AlreadyExists { path } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "already_exists",
                    message: format!("path already exists: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::NotAFile { path } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "not_a_file",
                    message: format!("path is not a file: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::NotADirectory { path } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "not_a_directory",
                    message: format!("path is not a directory: {}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::InvalidName { kind, value } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "invalid_name",
                    message: format!("invalid {kind} name: {value}"),
                    path: None,
                    operation: None,
                    kind: Some(kind.to_owned()),
                    value: Some(value),
                },
            },
            StorageError::PathConflict { path, message } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "path_conflict",
                    message: format!("path conflict at '{}': {message}", path_to_string(&path)),
                    path: Some(path_to_string(&path)),
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::Io {
                operation,
                path,
                source,
            } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "io",
                    message: source.to_string(),
                    path: Some(path_to_string(&path)),
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::ReaderIo { operation, source } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "reader_io",
                    message: source.to_string(),
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::DefinitionsLoad { message } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "definitions_load",
                    message,
                    path: None,
                    operation: None,
                    kind: None,
                    value: None,
                },
            },
            StorageError::AsyncRuntime { operation, message } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "async_runtime",
                    message,
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
            StorageError::Watch { operation, message } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "watch",
                    message,
                    path: None,
                    operation: Some(operation.to_owned()),
                    kind: None,
                    value: None,
                },
            },
        }
    }
}
