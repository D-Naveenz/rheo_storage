#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc, clippy::result_large_err)]

//! Native C ABI wrapper for the Rheo Storage Rust core.
//!
//! The exported surface is intentionally small and path-based so higher-level
//! bindings such as the .NET package can provide the ergonomic object model.

use std::ffi::{CStr, c_char};
use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::ptr;
use std::slice;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rheo_storage::{
    AnalysisReport, ContentKind, DetectedDefinition, DirectoryDeleteOptions, DirectoryInfo,
    DirectoryStorage, FileInfo, FileStorage, ReadOptions, SearchScope,
    StorageCancellationToken, StorageChangeEvent, StorageChangeType, StorageEntry, StorageError,
    StorageProgress, StorageWatchConfig, TransferOptions, WriteOptions, analyze_path,
    copy_directory, copy_directory_with_options, copy_file, copy_file_with_options,
    create_directory, create_directory_all, delete_directory, delete_directory_with_options,
    delete_file, move_directory, move_directory_with_options, move_file,
    move_file_with_options, read_file, read_file_to_string, rename_directory, rename_file,
    write_file, write_file_string,
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

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RheoOperationState {
    Running = 0,
    Completed = 1,
    Failed = 2,
    Cancelled = 3,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct RheoOperationSnapshot {
    pub state: RheoOperationState,
    pub has_total_bytes: u8,
    pub total_bytes: u64,
    pub bytes_transferred: u64,
    pub bytes_per_second: f64,
}

#[derive(Debug, Clone)]
struct SharedProgressState {
    total_bytes: Option<u64>,
    bytes_transferred: u64,
    bytes_per_second: f64,
}

#[derive(Debug)]
enum OperationResult {
    None,
    String(String),
    Bytes(Vec<u8>),
}

#[derive(Debug)]
pub struct NativeOperationHandle {
    state: AtomicU8,
    progress: Mutex<SharedProgressState>,
    result: Mutex<Option<OperationResult>>,
    error: Mutex<Option<FfiFailure>>,
    cancellation_token: StorageCancellationToken,
    worker: Mutex<Option<JoinHandle<()>>>,
}

struct NativeProgressReporter {
    state: Arc<NativeOperationHandle>,
}

pub struct NativeWatchHandle {
    handle: Mutex<Option<rheo_storage::DirectoryWatchHandle>>,
}

#[derive(Debug)]
pub struct NativeWriteSession {
    path: PathBuf,
    file: Mutex<Option<File>>,
    completed: AtomicBool,
}

#[derive(Debug, Clone, Copy)]
enum EntryKind {
    Files,
    Directories,
    All,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
struct StorageChangeEventDto {
    change_type: &'static str,
    path: String,
    previous_path: Option<String>,
    observed_at_utc_ms: u64,
}

#[derive(Debug, Clone)]
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

impl rheo_storage::ProgressReporter for NativeProgressReporter {
    fn report(&self, progress: StorageProgress) {
        if let Ok(mut state) = self.state.progress.lock() {
            state.total_bytes = progress.total_bytes;
            state.bytes_transferred = progress.bytes_transferred;
            state.bytes_per_second = progress.bytes_per_second;
        }
    }
}

impl NativeOperationHandle {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: AtomicU8::new(RheoOperationState::Running as u8),
            progress: Mutex::new(SharedProgressState {
                total_bytes: None,
                bytes_transferred: 0,
                bytes_per_second: 0.0,
            }),
            result: Mutex::new(None),
            error: Mutex::new(None),
            cancellation_token: StorageCancellationToken::new(),
            worker: Mutex::new(None),
        })
    }

    fn state(&self) -> RheoOperationState {
        match self.state.load(Ordering::SeqCst) {
            0 => RheoOperationState::Running,
            1 => RheoOperationState::Completed,
            2 => RheoOperationState::Failed,
            3 => RheoOperationState::Cancelled,
            _ => RheoOperationState::Failed,
        }
    }

    fn set_completed(&self, result: OperationResult) {
        if let Ok(mut slot) = self.result.lock() {
            *slot = Some(result);
        }
        self.state
            .store(RheoOperationState::Completed as u8, Ordering::SeqCst);
    }

    fn set_failure(&self, failure: FfiFailure) {
        let state = if failure.payload.code == "cancelled" {
            RheoOperationState::Cancelled
        } else {
            RheoOperationState::Failed
        };
        if let Ok(mut slot) = self.error.lock() {
            *slot = Some(failure);
        }
        self.state.store(state as u8, Ordering::SeqCst);
    }

    fn snapshot(&self) -> RheoOperationSnapshot {
        let progress = self.progress.lock().unwrap_or_else(|p| p.into_inner());
        RheoOperationSnapshot {
            state: self.state(),
            has_total_bytes: u8::from(progress.total_bytes.is_some()),
            total_bytes: progress.total_bytes.unwrap_or(0),
            bytes_transferred: progress.bytes_transferred,
            bytes_per_second: progress.bytes_per_second,
        }
    }

    fn take_string_result(&self) -> Result<Option<String>, FfiFailure> {
        let mut slot = self.result.lock().unwrap_or_else(|p| p.into_inner());
        match slot.take() {
            Some(OperationResult::String(value)) => Ok(Some(value)),
            Some(OperationResult::None) | None => Ok(None),
            Some(OperationResult::Bytes(_)) => Err(FfiFailure::error(
                "operation result contained bytes, not a string",
            )),
        }
    }

    fn take_bytes_result(&self) -> Result<Option<Vec<u8>>, FfiFailure> {
        let mut slot = self.result.lock().unwrap_or_else(|p| p.into_inner());
        match slot.take() {
            Some(OperationResult::Bytes(value)) => Ok(Some(value)),
            Some(OperationResult::None) | None => Ok(None),
            Some(OperationResult::String(_)) => Err(FfiFailure::error(
                "operation result contained a string, not bytes",
            )),
        }
    }

    fn clone_error(&self) -> Option<FfiFailure> {
        self.error
            .lock()
            .unwrap_or_else(|p| p.into_inner())
            .as_ref()
            .cloned()
    }
}

impl NativeWriteSession {
    fn write_chunk(&self, bytes: &[u8]) -> Result<(), FfiFailure> {
        if self.completed.load(Ordering::SeqCst) {
            return Err(FfiFailure::error("write session has already completed"));
        }

        let mut slot = self.file.lock().unwrap_or_else(|p| p.into_inner());
        let file = slot
            .as_mut()
            .ok_or_else(|| FfiFailure::error("write session is not open"))?;
        file.write_all(bytes).map_err(|err| {
            FfiFailure::io("write session chunk", &self.path, err)
        })
    }

    fn complete(&self) -> Result<String, FfiFailure> {
        if self.completed.swap(true, Ordering::SeqCst) {
            return Ok(path_to_string(&self.path));
        }

        let mut slot = self.file.lock().unwrap_or_else(|p| p.into_inner());
        let mut file = slot
            .take()
            .ok_or_else(|| FfiFailure::error("write session is not open"))?;
        file.flush()
            .map_err(|err| FfiFailure::io("flush write session", &self.path, err))?;
        Ok(path_to_string(&self.path))
    }

    fn abort(&self) -> Result<(), FfiFailure> {
        let mut slot = self.file.lock().unwrap_or_else(|p| p.into_inner());
        slot.take();
        if !self.completed.load(Ordering::SeqCst) && self.path.exists() {
            fs::remove_file(&self.path)
                .map_err(|err| FfiFailure::io("abort write session", &self.path, err))?;
        }
        Ok(())
    }
}

fn progress_reporter(handle: &Arc<NativeOperationHandle>) -> Arc<dyn rheo_storage::ProgressReporter> {
    Arc::new(NativeProgressReporter {
        state: handle.clone(),
    })
}

fn transfer_options(handle: &Arc<NativeOperationHandle>, overwrite: bool) -> TransferOptions {
    TransferOptions {
        overwrite,
        buffer_size: None,
        progress: Some(progress_reporter(handle)),
        cancellation_token: Some(handle.cancellation_token.clone()),
    }
}

fn write_options(
    handle: &Arc<NativeOperationHandle>,
    overwrite: bool,
    create_parent_directories: bool,
) -> WriteOptions {
    WriteOptions {
        overwrite,
        create_parent_directories,
        buffer_size: None,
        progress: Some(progress_reporter(handle)),
        cancellation_token: Some(handle.cancellation_token.clone()),
    }
}

fn delete_options(handle: &Arc<NativeOperationHandle>, recursive: bool) -> DirectoryDeleteOptions {
    DirectoryDeleteOptions {
        recursive,
        cancellation_token: Some(handle.cancellation_token.clone()),
    }
}

fn fail_if_cancelled(handle: &Arc<NativeOperationHandle>) -> Result<(), FfiFailure> {
    if handle.cancellation_token.is_cancelled() {
        return Err(FfiFailure::cancelled("native operation"));
    }

    Ok(())
}

fn spawn_path_operation(
    operation: impl FnOnce(Arc<NativeOperationHandle>) -> Result<OperationResult, FfiFailure>
        + Send
        + 'static,
) -> *mut NativeOperationHandle {
    spawn_operation(operation)
}

fn spawn_operation(
    operation: impl FnOnce(Arc<NativeOperationHandle>) -> Result<OperationResult, FfiFailure>
        + Send
        + 'static,
) -> *mut NativeOperationHandle {
    let handle = NativeOperationHandle::new();
    let worker_state = handle.clone();
    let join = thread::spawn(move || match catch_unwind(AssertUnwindSafe(|| operation(worker_state.clone()))) {
        Ok(Ok(result)) => worker_state.set_completed(result),
        Ok(Err(failure)) => worker_state.set_failure(failure),
        Err(_) => worker_state.set_failure(FfiFailure {
            status: RheoStatus::Panic,
            payload: ErrorPayload {
                code: "panic",
                message: "native operation panicked".to_owned(),
                path: None,
                operation: None,
                kind: None,
                value: None,
            },
        }),
    });

    if let Ok(mut slot) = handle.worker.lock() {
        *slot = Some(join);
    }

    Arc::into_raw(handle) as *mut NativeOperationHandle
}

unsafe fn clone_operation_handle(
    handle: *mut NativeOperationHandle,
) -> Result<Arc<NativeOperationHandle>, FfiFailure> {
    if handle.is_null() {
        return Err(FfiFailure::invalid_argument(
            "handle",
            "operation handle must not be null",
        ));
    }

    let arc = unsafe { Arc::from_raw(handle) };
    let cloned = arc.clone();
    let _ = Arc::into_raw(arc);
    Ok(cloned)
}

unsafe fn with_watch_handle<T>(
    handle: *mut NativeWatchHandle,
    action: impl FnOnce(&NativeWatchHandle) -> Result<T, FfiFailure>,
) -> Result<T, FfiFailure> {
    if handle.is_null() {
        return Err(FfiFailure::invalid_argument(
            "handle",
            "watch handle must not be null",
        ));
    }

    let handle = unsafe { &*handle };
    action(handle)
}

unsafe fn with_write_session<T>(
    handle: *mut NativeWriteSession,
    action: impl FnOnce(&NativeWriteSession) -> Result<T, FfiFailure>,
) -> Result<T, FfiFailure> {
    if handle.is_null() {
        return Err(FfiFailure::invalid_argument(
            "handle",
            "write session handle must not be null",
        ));
    }

    let handle = unsafe { &*handle };
    action(handle)
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_copy_file(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                copy_file_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| path_to_string(&path))
                .map(OperationResult::String)
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_move_file(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                move_file_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| path_to_string(&path))
                .map(OperationResult::String)
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_rename_file(
    source: *const c_char,
    new_name: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                rename_file(&source, &new_name)
                    .map(|path| path_to_string(&path))
                    .map(OperationResult::String)
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_delete_file(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                delete_file(&path)
                    .map(|_| OperationResult::None)
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_read_file(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                FileStorage::from_existing(&path)
                    .map_err(FfiFailure::from)?
                    .read_with_options(ReadOptions {
                        buffer_size: None,
                        progress: Some(progress_reporter(&handle)),
                        cancellation_token: Some(handle.cancellation_token.clone()),
                    })
                .map(OperationResult::Bytes)
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_read_file_text(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                let bytes = FileStorage::from_existing(&path)
                    .map_err(FfiFailure::from)?
                    .read_with_options(ReadOptions {
                        buffer_size: None,
                        progress: Some(progress_reporter(&handle)),
                        cancellation_token: Some(handle.cancellation_token.clone()),
                    })
                    .map_err(FfiFailure::from)?;

                let text = String::from_utf8(bytes)
                    .map_err(|_| FfiFailure::error("file contents were not valid UTF-8"))?;

                Ok(OperationResult::String(text))
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_write_file(
    path: *const c_char,
    data_ptr: *const u8,
    data_len: usize,
    overwrite: u8,
    create_parent_directories: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let bytes = parse_bytes_arg(data_ptr, data_len, "data")?.to_vec();
            Ok(spawn_path_operation(move |handle| {
                let file = FileStorage::new(&path).map_err(FfiFailure::from)?;
                let mut cursor = Cursor::new(bytes);
                file.write_from_reader(
                    &mut cursor,
                    write_options(&handle, overwrite != 0, create_parent_directories != 0),
                )
                .map(|file| OperationResult::String(path_to_string(file.path())))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_write_file_text(
    path: *const c_char,
    text: *const c_char,
    overwrite: u8,
    create_parent_directories: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            let text = parse_string_arg(text, "text")?;
            Ok(spawn_path_operation(move |handle| {
                let file = FileStorage::new(&path).map_err(FfiFailure::from)?;
                let mut cursor = Cursor::new(text.into_bytes());
                file.write_from_reader(
                    &mut cursor,
                    write_options(&handle, overwrite != 0, create_parent_directories != 0),
                )
                .map(|file| OperationResult::String(path_to_string(file.path())))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_create_directory(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                create_directory(&path)
                    .map(|path| OperationResult::String(path_to_string(&path)))
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_create_directory_all(
    path: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                create_directory_all(&path)
                    .map(|path| OperationResult::String(path_to_string(&path)))
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_copy_directory(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                copy_directory_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| OperationResult::String(path_to_string(&path)))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_move_directory(
    source: *const c_char,
    destination: *const c_char,
    overwrite: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let destination = parse_path_arg(destination, "destination")?;
            Ok(spawn_path_operation(move |handle| {
                move_directory_with_options(
                    &source,
                    &destination,
                    transfer_options(&handle, overwrite != 0),
                )
                .map(|path| OperationResult::String(path_to_string(&path)))
                .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_rename_directory(
    source: *const c_char,
    new_name: *const c_char,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let source = parse_path_arg(source, "source")?;
            let new_name = parse_string_arg(new_name, "new_name")?;
            Ok(spawn_path_operation(move |handle| {
                fail_if_cancelled(&handle)?;
                rename_directory(&source, &new_name)
                    .map(|path| OperationResult::String(path_to_string(&path)))
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_start_delete_directory(
    path: *const c_char,
    recursive: u8,
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_operation_handle_creation(
        out_handle,
        out_error_ptr,
        out_error_len,
        || {
            let path = parse_path_arg(path, "path")?;
            Ok(spawn_path_operation(move |handle| {
                delete_directory_with_options(&path, delete_options(&handle, recursive != 0))
                    .map(|_| OperationResult::None)
                    .map_err(FfiFailure::from)
            }))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_get_snapshot(
    handle: *mut NativeOperationHandle,
    out_snapshot: *mut RheoOperationSnapshot,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!({
        if out_snapshot.is_null() {
            return write_error_only(
                out_error_ptr,
                out_error_len,
                FfiFailure::invalid_argument("out_snapshot", "snapshot output pointer must not be null"),
            );
        }
        if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
            return failure.status;
        }
        reset_buffer_out(out_error_ptr, out_error_len);

        match clone_operation_handle(handle) {
            Ok(handle) => {
                unsafe { *out_snapshot = handle.snapshot() };
                RheoStatus::Ok
            }
            Err(failure) => {
                write_error_payload(out_error_ptr, out_error_len, &failure);
                failure.status
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_cancel(
    handle: *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let handle = unsafe { clone_operation_handle(handle) }?;
        handle.cancellation_token.cancel();
        Ok(())
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_take_string_result(
    handle: *mut NativeOperationHandle,
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
            let handle = unsafe { clone_operation_handle(handle) }?;
            if let Some(failure) = handle.clone_error() {
                return Err(failure);
            }
            Ok(handle.take_string_result()?.unwrap_or_default())
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_take_bytes_result(
    handle: *mut NativeOperationHandle,
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
            let handle = unsafe { clone_operation_handle(handle) }?;
            if let Some(failure) = handle.clone_error() {
                return Err(failure);
            }
            Ok(handle.take_bytes_result()?.unwrap_or_default())
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_get_error(
    handle: *mut NativeOperationHandle,
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
            let handle = unsafe { clone_operation_handle(handle) }?;
            Ok(handle.clone_error().map(|failure| failure.payload))
        }
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_operation_free(handle: *mut NativeOperationHandle) {
    if handle.is_null() {
        return;
    }

    let handle = unsafe { Arc::from_raw(handle) };
    handle.cancellation_token.cancel();
    if let Ok(mut slot) = handle.worker.lock() {
        if let Some(worker) = slot.take() {
            let _ = worker.join();
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_watch_create(
    path: *const c_char,
    recursive: u8,
    debounce_window_ms: u64,
    out_handle: *mut *mut NativeWatchHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!({
        if out_handle.is_null() {
            return write_error_only(
                out_error_ptr,
                out_error_len,
                FfiFailure::invalid_argument("out_handle", "watch handle output must not be null"),
            );
        }
        if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
            return failure.status;
        }
        unsafe { *out_handle = ptr::null_mut() };
        reset_buffer_out(out_error_ptr, out_error_len);

        let result = (|| {
            let path = parse_path_arg(path, "path")?;
            let config = StorageWatchConfig {
                recursive: recursive != 0,
                debounce_window: Duration::from_millis(debounce_window_ms.max(1)),
            };
            let watch = DirectoryStorage::from_existing(&path)
                .map_err(FfiFailure::from)?
                .watch(config)
                .map_err(FfiFailure::from)?;
            Ok(Box::into_raw(Box::new(NativeWatchHandle {
                handle: Mutex::new(Some(watch)),
            })))
        })();

        match result {
            Ok(handle) => {
                unsafe { *out_handle = handle };
                RheoStatus::Ok
            }
            Err(failure) => {
                write_error_payload(out_error_ptr, out_error_len, &failure);
                failure.status
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_watch_try_recv_json(
    handle: *mut NativeWatchHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_watch_receive(
        handle,
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        |watch| watch.try_recv(),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_watch_recv_json(
    handle: *mut NativeWatchHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_watch_receive(
        handle,
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        |watch| watch.recv().map(Some),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_watch_recv_json_timeout(
    handle: *mut NativeWatchHandle,
    timeout_ms: u64,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_watch_receive(
        handle,
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        |watch| watch.recv_timeout(Duration::from_millis(timeout_ms)),
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_watch_stop(
    handle: *mut NativeWatchHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        unsafe { with_watch_handle(handle, |watch_handle| {
            let mut slot = watch_handle.handle.lock().unwrap_or_else(|p| p.into_inner());
            slot.take();
            Ok(())
        }) }
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_watch_free(handle: *mut NativeWatchHandle) {
    if handle.is_null() {
        return;
    }

    let handle = unsafe { Box::from_raw(handle) };
    if let Ok(mut slot) = handle.handle.lock() {
        slot.take();
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_session_create(
    path: *const c_char,
    overwrite: u8,
    create_parent_directories: u8,
    out_handle: *mut *mut NativeWriteSession,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!({
        if out_handle.is_null() {
            return write_error_only(
                out_error_ptr,
                out_error_len,
                FfiFailure::invalid_argument("out_handle", "write session output must not be null"),
            );
        }
        if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
            return failure.status;
        }
        unsafe { *out_handle = ptr::null_mut() };
        reset_buffer_out(out_error_ptr, out_error_len);

        let result = (|| {
            let path = parse_path_arg(path, "path")?;
            if create_parent_directories != 0 {
                if let Some(parent) = path.parent() {
                    if !parent.as_os_str().is_empty() {
                        fs::create_dir_all(parent)
                            .map_err(|err| FfiFailure::io("create parent directory for", parent, err))?;
                    }
                }
            }

            let mut options = OpenOptions::new();
            options.write(true).create(true);
            if overwrite != 0 {
                options.truncate(true);
            } else {
                options.create_new(true);
            }

            let file = options
                .open(&path)
                .map_err(|err| FfiFailure::io("open write session", &path, err))?;

            Ok(Box::into_raw(Box::new(NativeWriteSession {
                path,
                file: Mutex::new(Some(file)),
                completed: AtomicBool::new(false),
            })))
        })();

        match result {
            Ok(handle) => {
                unsafe { *out_handle = handle };
                RheoStatus::Ok
            }
            Err(failure) => {
                write_error_payload(out_error_ptr, out_error_len, &failure);
                failure.status
            }
        }
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_session_write_chunk(
    handle: *mut NativeWriteSession,
    data_ptr: *const u8,
    data_len: usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || {
        let bytes = parse_bytes_arg(data_ptr, data_len, "data")?;
        unsafe { with_write_session(handle, |session| session.write_chunk(bytes)) }
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_session_complete(
    handle: *mut NativeWriteSession,
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
        || unsafe { with_write_session(handle, |session| session.complete()) },
    ))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_session_abort(
    handle: *mut NativeWriteSession,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
) -> RheoStatus {
    ffi_fn!(execute_unit(out_error_ptr, out_error_len, || unsafe {
        with_write_session(handle, |session| session.abort())
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn rheo_write_session_free(handle: *mut NativeWriteSession) {
    if handle.is_null() {
        return;
    }

    let session = unsafe { Box::from_raw(handle) };
    let _ = session.abort();
}

unsafe fn execute_watch_receive(
    handle: *mut NativeWatchHandle,
    out_json_ptr: *mut *mut u8,
    out_json_len: *mut usize,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    receive: impl FnOnce(&rheo_storage::DirectoryWatchHandle) -> Result<Option<StorageChangeEvent>, StorageError>,
) -> RheoStatus {
    execute_json(
        out_json_ptr,
        out_json_len,
        out_error_ptr,
        out_error_len,
        || unsafe {
            with_watch_handle(handle, |watch_handle| {
                let slot = watch_handle.handle.lock().unwrap_or_else(|p| p.into_inner());
                let watch = slot.as_ref().ok_or_else(|| FfiFailure::error("watch handle has already been stopped"))?;
                Ok(receive(watch)
                    .map_err(FfiFailure::from)?
                    .map(StorageChangeEventDto::from))
            })
        },
    )
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

unsafe fn execute_operation_handle_creation(
    out_handle: *mut *mut NativeOperationHandle,
    out_error_ptr: *mut *mut u8,
    out_error_len: *mut usize,
    operation: impl FnOnce() -> Result<*mut NativeOperationHandle, FfiFailure>,
) -> RheoStatus {
    if out_handle.is_null() {
        return write_error_only(
            out_error_ptr,
            out_error_len,
            FfiFailure::invalid_argument("out_handle", "output handle pointer must not be null"),
        );
    }

    if let Err(failure) = validate_buffer_out(out_error_ptr, out_error_len) {
        return failure.status;
    }

    unsafe { *out_handle = ptr::null_mut() };
    reset_buffer_out(out_error_ptr, out_error_len);

    match operation() {
        Ok(handle) => {
            unsafe { *out_handle = handle };
            RheoStatus::Ok
        }
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

impl From<StorageChangeEvent> for StorageChangeEventDto {
    fn from(value: StorageChangeEvent) -> Self {
        Self {
            change_type: match value.change_type {
                StorageChangeType::Created => "created",
                StorageChangeType::Deleted => "deleted",
                StorageChangeType::Modified => "modified",
                StorageChangeType::Relocated => "relocated",
            },
            path: path_to_string(&value.path),
            previous_path: value.previous_path.as_deref().map(path_to_string),
            observed_at_utc_ms: system_time_to_unix_millis(Some(value.observed_at)).unwrap_or(0),
        }
    }
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

    fn io(operation: &'static str, path: &Path, source: std::io::Error) -> Self {
        Self {
            status: RheoStatus::Error,
            payload: ErrorPayload {
                code: "io",
                message: source.to_string(),
                path: Some(path_to_string(path)),
                operation: Some(operation.to_owned()),
                kind: None,
                value: None,
            },
        }
    }

    fn cancelled(operation: &'static str) -> Self {
        Self {
            status: RheoStatus::Error,
            payload: ErrorPayload {
                code: "cancelled",
                message: format!("operation cancelled while attempting to {operation}"),
                path: None,
                operation: Some(operation.to_owned()),
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
            StorageError::Cancelled { operation } => Self {
                status: RheoStatus::Error,
                payload: ErrorPayload {
                    code: "cancelled",
                    message: format!("operation cancelled while attempting to {operation}"),
                    path: None,
                    operation: Some(operation.to_owned()),
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
