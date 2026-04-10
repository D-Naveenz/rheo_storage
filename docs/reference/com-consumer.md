# COM Consumer Guide

Use `rheo_storage_com` from ordinary C#/.NET desktop projects that do not need a WinRT component model.

## Current Surface
- `FileObject`
- `DirectoryObject`

## Consumption Model
- Instantiate the COM class.
- Call `Open(path)` first.
- Use the object methods to query or mutate the bound file-system path.

## Notes
- The COM layer is intentionally thin over `rheo_storage_lib`.
- The current list methods return newline-separated paths for simple interop.
- Long-running behavior, validation, and file semantics come from the Rust core.
