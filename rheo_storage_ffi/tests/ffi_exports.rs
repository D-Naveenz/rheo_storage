use std::ffi::CString;
use std::ptr;
use std::slice;

use rheo_storage_ffi::{
    RheoStatus, rheo_analyze_path, rheo_bytes_free, rheo_create_directory_all, rheo_delete_file,
    rheo_get_directory_info, rheo_get_file_info, rheo_list_entries, rheo_read_file,
    rheo_rename_file, rheo_string_free, rheo_write_file_text,
};
use tempfile::tempdir;

fn fixture_path() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("rheo_storage")
        .join("tests")
        .join("fixtures")
        .join("sample-2.pdf")
}

fn string_from_output(ptr: *mut u8, len: usize) -> String {
    let bytes = unsafe { slice::from_raw_parts(ptr, len) };
    let value = String::from_utf8(bytes.to_vec()).expect("ffi output should be valid utf-8");
    unsafe { rheo_string_free(ptr, len) };
    value
}

fn bytes_from_output(ptr: *mut u8, len: usize) -> Vec<u8> {
    let bytes = unsafe { slice::from_raw_parts(ptr, len).to_vec() };
    unsafe { rheo_bytes_free(ptr, len) };
    bytes
}

#[test]
fn analyze_path_returns_json() {
    let fixture = std::fs::canonicalize(fixture_path()).unwrap();
    let fixture = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();
    let mut out_ptr: *mut u8 = ptr::null_mut();
    let mut out_len = 0;
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status = unsafe {
        rheo_analyze_path(
            fixture.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };

    assert_eq!(status, RheoStatus::Ok);
    assert!(err_ptr.is_null());
    let json = string_from_output(out_ptr, out_len);
    assert!(json.contains("\"top_mime_type\":\"application/pdf\""));
}

#[test]
fn file_info_and_directory_info_include_optional_payloads() {
    let fixture = std::fs::canonicalize(fixture_path()).unwrap();
    let fixture = CString::new(fixture.to_string_lossy().as_bytes()).unwrap();
    let temp = tempdir().unwrap();
    let temp_c = CString::new(temp.path().to_string_lossy().as_bytes()).unwrap();

    let mut out_ptr: *mut u8 = ptr::null_mut();
    let mut out_len = 0;
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let file_status = unsafe {
        rheo_get_file_info(
            fixture.as_ptr(),
            1,
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(file_status, RheoStatus::Ok);
    let json = string_from_output(out_ptr, out_len);
    assert!(json.contains("\"analysis\""));

    let directory_status = unsafe {
        rheo_get_directory_info(
            temp_c.as_ptr(),
            1,
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(directory_status, RheoStatus::Ok);
    let json = string_from_output(out_ptr, out_len);
    assert!(json.contains("\"summary\""));
}

#[test]
fn write_read_list_and_delete_round_trip_non_ascii_paths() {
    let temp = tempdir().unwrap();
    let nested = temp.path().join("nested").join("inner");
    let nested_c = CString::new(nested.to_string_lossy().as_bytes()).unwrap();
    let file = nested.join("unicodé.txt");
    let file_c = CString::new(file.to_string_lossy().as_bytes()).unwrap();
    let renamed = nested.join("renamed.txt");
    let renamed_name_c = CString::new("renamed.txt").unwrap();
    let mut out_ptr: *mut u8 = ptr::null_mut();
    let mut out_len = 0;
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;

    let status = unsafe {
        rheo_create_directory_all(
            nested_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(status, RheoStatus::Ok);
    let _ = string_from_output(out_ptr, out_len);

    let text = CString::new("hello from ffi").unwrap();
    let write_status = unsafe {
        rheo_write_file_text(
            file_c.as_ptr(),
            text.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(write_status, RheoStatus::Ok);
    let written_path = string_from_output(out_ptr, out_len);
    assert!(written_path.ends_with("unicodé.txt"));

    let read_status = unsafe {
        rheo_read_file(
            file_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(read_status, RheoStatus::Ok);
    let bytes = bytes_from_output(out_ptr, out_len);
    assert_eq!(bytes, b"hello from ffi");

    let list_status = unsafe {
        rheo_list_entries(
            nested_c.as_ptr(),
            0,
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(list_status, RheoStatus::Ok);
    let list_json = string_from_output(out_ptr, out_len);
    assert!(list_json.contains("unicod"));

    let rename_status = unsafe {
        rheo_rename_file(
            file_c.as_ptr(),
            renamed_name_c.as_ptr(),
            &mut out_ptr,
            &mut out_len,
            &mut err_ptr,
            &mut err_len,
        )
    };
    assert_eq!(rename_status, RheoStatus::Ok);
    let renamed_path = string_from_output(out_ptr, out_len);
    assert_eq!(renamed_path, renamed.to_string_lossy());

    let renamed_c = CString::new(renamed.to_string_lossy().as_bytes()).unwrap();
    let delete_status = unsafe { rheo_delete_file(renamed_c.as_ptr(), &mut err_ptr, &mut err_len) };
    assert_eq!(delete_status, RheoStatus::Ok);
}

#[test]
fn invalid_arguments_produce_error_payload() {
    let mut err_ptr: *mut u8 = ptr::null_mut();
    let mut err_len = 0;
    let status = unsafe { rheo_delete_file(ptr::null(), &mut err_ptr, &mut err_len) };

    assert_eq!(status, RheoStatus::InvalidArgument);
    let json = string_from_output(err_ptr, err_len);
    assert!(json.contains("\"code\":\"invalid_argument\""));
}
