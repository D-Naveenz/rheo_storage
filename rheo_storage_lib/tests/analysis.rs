use std::fs::{self, File, OpenOptions};
use std::io::{Cursor, Write};

use rheo_storage_lib::{ContentKind, FileInfo, StorageError, analyze_path, analyze_reader};
use tempfile::tempdir;

#[test]
fn analyze_path_missing_file_returns_not_found() {
    let temp = tempdir().unwrap();
    let missing = temp.path().join("missing.bin");

    let err = analyze_path(&missing).unwrap_err();
    assert!(matches!(err, StorageError::NotFound { .. }));
}

#[test]
fn analyze_path_empty_file_returns_unknown_report() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("empty.bin");
    File::create(&path).unwrap();

    let report = analyze_path(&path).unwrap();
    assert!(report.is_empty());
    assert_eq!(report.content_kind, ContentKind::Unknown);
    assert_eq!(report.top_mime_type, None);
}

#[test]
fn analyze_reader_detects_png_signature() {
    let png_bytes = [
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R',
    ];
    let cursor = Cursor::new(png_bytes);

    let report = analyze_reader(cursor, Some(std::path::Path::new("image.png"))).unwrap();

    assert!(!report.matches.is_empty());
    assert_eq!(report.top_detected_extension.as_deref(), Some("png"));
    assert_eq!(report.content_kind, ContentKind::Binary);
}

#[test]
fn analyze_reader_detects_zip_signature() {
    let zip_bytes = [0x50, 0x4B, 0x03, 0x04, 0x14, 0x00, 0x00, 0x00];
    let cursor = Cursor::new(zip_bytes);

    let report = analyze_reader(cursor, Some(std::path::Path::new("archive.zip"))).unwrap();

    assert!(!report.matches.is_empty());
    assert!(
        report
            .matches
            .iter()
            .flat_map(|item| item.extensions.iter())
            .any(|ext| ext.eq_ignore_ascii_case("zip"))
    );
}

#[test]
fn analyze_reader_detects_pdf_signature() {
    let cursor = Cursor::new(b"%PDF-1.7\n1 0 obj\n<< /Type /Catalog >>\nendobj\n".to_vec());

    let report = analyze_reader(cursor, Some(std::path::Path::new("sample.pdf"))).unwrap();

    assert_eq!(report.top_detected_extension.as_deref(), Some("pdf"));
    assert_eq!(report.content_kind, ContentKind::Binary);
}

#[test]
fn analyze_reader_uses_text_fallback_for_plain_text() {
    let cursor = Cursor::new(b"Hello from Rheo.Storage in Rust.\n".to_vec());

    let report = analyze_reader(cursor, Some(std::path::Path::new("note.txt"))).unwrap();

    assert_eq!(report.content_kind, ContentKind::Text);
    assert_eq!(report.top_mime_type.as_deref(), Some("text/plain"));
    assert_eq!(report.top_detected_extension.as_deref(), Some("txt"));
}

#[test]
fn analyze_reader_detects_utf8_bom_text() {
    let cursor = Cursor::new(vec![0xEF, 0xBB, 0xBF, b'H', b'i', b'!']);

    let report = analyze_reader(cursor, Some(std::path::Path::new("bom.txt"))).unwrap();

    assert_eq!(report.content_kind, ContentKind::Text);
    assert_eq!(report.top_mime_type.as_deref(), Some("text/plain"));
}

#[test]
fn analyze_reader_uses_binary_fallback_for_unknown_bytes() {
    let cursor = Cursor::new(vec![0x00, 0xFF, 0x7A, 0x3C, 0x5D, 0xA1, 0x42, 0x99]);

    let report = analyze_reader(cursor, Some(std::path::Path::new("mystery.bin"))).unwrap();

    assert_eq!(report.content_kind, ContentKind::Binary);
    assert_eq!(
        report.top_mime_type.as_deref(),
        Some("application/octet-stream")
    );
}

#[test]
fn ranked_matches_are_sorted_and_confidence_sums_to_roughly_hundred() {
    let cursor = Cursor::new([
        0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x00, 0x00, 0x0D, b'I', b'H', b'D',
        b'R',
    ]);

    let report = analyze_reader(cursor, Some(std::path::Path::new("image.png"))).unwrap();

    for pair in report.matches.windows(2) {
        assert!(pair[0].score >= pair[1].score);
    }

    let total_confidence = report
        .matches
        .iter()
        .map(|item| item.confidence)
        .sum::<f64>();
    assert!((total_confidence - 100.0).abs() < 0.01);
}

#[cfg(windows)]
#[test]
fn analyze_path_can_read_file_while_another_handle_is_open() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("shared.txt");
    fs::write(&path, b"shared read").unwrap();

    let mut open_handle = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    writeln!(open_handle).unwrap();

    let report = analyze_path(&path).unwrap();
    assert_eq!(report.content_kind, ContentKind::Text);
}

#[test]
fn file_info_from_path_exposes_rust_native_metadata() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("sample.txt");
    fs::write(&path, b"hello file info").unwrap();

    let info = FileInfo::from_path(&path).unwrap();

    assert_eq!(info.filename_extension.as_deref(), Some("txt"));
    assert_eq!(info.mime_type.as_deref(), Some("text/plain"));
    assert_eq!(info.content_kind, ContentKind::Text);
    assert_eq!(info.size, 15);
}
