# Rheo.Storage for .NET

`Rheo.Storage` is the managed .NET wrapper over the native `rheo_storage_ffi`
runtime.

It targets `net10.0` on Windows and packages the native Rust DLL alongside the
managed API.

The public surface is object-oriented and path-based:

- `StorageFile` for file analysis, read/write, copy, move, rename, and delete
- `StorageDirectory` for enumeration, directory operations, and explicit
  watching
- sync and async APIs with cancellation support for long-running native
  operations
- `LibraryImport`-based interop with JSON contracts for rich results and native
  write sessions for streamed uploads
