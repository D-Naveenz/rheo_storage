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
- Native Rust logs can be forwarded into a host `ILoggerFactory` through
  `RheoStorage.UseLoggerFactory`.

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
- NuGet packaging stages both native assets:
  - `runtimes/win-x64/native/rheo_storage_ffi.dll`
  - `runtimes/win-arm64/native/rheo_storage_ffi.dll`
- The NuGet package supports Windows `x64` and `arm64` only
- Package consumption fails clearly for explicit unsupported RIDs such as `win-x86`
- Runtime usage outside Windows `x64` or `arm64` throws `PlatformNotSupportedException`

## Logging
- Call `RheoStorage.UseLoggerFactory(loggerFactory)` to forward native and managed
  logs into the host logging pipeline
- Native log records include level, category, message, timestamps, and structured
  fields captured from Rust `tracing`
- Async operation handles emit managed logs for wait, completion, cancellation,
  and failure transitions

## Tests
- `bindings/dotnet/Rheo.Storage.Tests` provides xUnit v3 end-to-end coverage for
  file handling, directory handling, analysis, and watching
