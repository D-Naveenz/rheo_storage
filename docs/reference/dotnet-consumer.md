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

## Included in v1
- file analysis
- file and directory metadata
- basic file read, write, copy, move, rename, and delete
- directory create, copy, move, rename, delete, and enumeration

## Not Included in v1
- watchers
- native async exports
- progress callbacks
- reader or writer callback interop
