# .NET Consumer Guide

Use `bindings/dotnet/Rheo.Storage` when consuming Rheo Storage from C# or other
.NET languages.

## Current Shape
- The managed wrapper calls the native `rheo_storage_ffi` DLL through
  `LibraryImport`.
- The native ABI is path-based and UTF-8 oriented.
- Rich results such as analysis, metadata, and directory listings cross the ABI
  as JSON payloads.
- Raw file reads cross the ABI as byte buffers with explicit free helpers on the
  native side.
- Long-running copy, move, delete, read, and write workflows use native
  operation handles so the managed API can expose cancellation and progress.
- Directory watching is explicit from C# through `StorageDirectory.StartWatching`
  and `StopWatching`.
- Stream-based writes use a native write session that accepts chunked uploads
  from managed `Stream` readers.

## Public API
- `RheoStorage.File(path)` creates a `StorageFile`
- `RheoStorage.Directory(path)` creates a `StorageDirectory`
- `RheoStorage.AnalyzePath(path)` runs point-in-time analysis without creating a
  wrapper object
- `StorageFile` exposes sync and async methods for:
  - analysis
  - reading bytes and text
  - writing bytes, text, and streams
  - copy, move, rename, and delete
- `StorageDirectory` exposes sync and async methods for:
  - create and create-all
  - copy, move, rename, and delete
  - child resolution and recursive enumeration
  - explicit debounced watching with a `Changed` event

## Packaging
- The managed project builds `rheo_storage_ffi` during `dotnet build`
- The native DLL is copied to the output folder for local development
- NuGet packaging places the native asset under `runtimes/win-x64/native`

## Tests
- `bindings/dotnet/Rheo.Storage.Tests` provides xUnit v3 end-to-end coverage for
  file handling, directory handling, analysis, and watching
