# Rust Consumer Guide

Use `rheo_storage_lib` directly when your application is already written in Rust.

## Typical Flow
- Use `FileStorage` and `DirectoryStorage` for path-based operations.
- Use `FileInfo` and `DirectoryInfo` when you need metadata.
- Use `analyze_path` or `FileInfo::analysis()` when you need content-based type detection.
- Use `DirectoryStorage::watch(...)` when you need debounced change notifications.

## Performance Model
- Plain read/write/copy/move/delete/enumerate calls do not trigger file analysis.
- File analysis is lazy.
- Windows shell enrichment is lazy.
- Directory summaries are lazy unless explicitly preloaded.
