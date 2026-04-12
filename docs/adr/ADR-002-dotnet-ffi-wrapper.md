# ADR-002: .NET Wrapper via FFI Instead of WinRT

## Status
Accepted

## Context
The Rust core now covers the desired runtime behavior for analysis, metadata,
operations, navigation, and watching. The remaining problem is exposing that
behavior to the `Rheo.Storage` .NET package without forcing WinRT constraints
onto the Rust core or onto unpackaged desktop consumers.

The previous repo direction reserved `rheo_storage_winrt` as the wrapper layer,
but packaged Windows Runtime components add activation and environment
constraints that are not a good fit for the target NuGet audience.

## Decision
- Remove `rheo_storage_winrt` from the workspace.
- Expose the Rust runtime to .NET through a dedicated `rheo_storage_ffi` crate
  that publishes a Windows-first C ABI.
- Keep the native ABI path-based and UTF-8 oriented.
- Return rich data to .NET as JSON payloads in v1 and raw file reads as byte
  buffers with explicit free functions.
- Keep the ergonomic public .NET API in a managed wrapper project under
  `bindings/dotnet/Rheo.Storage`.
- Keep watchers, async exports, and progress callbacks out of the initial FFI
  surface.

## Consequences
- `rheo_storage` remains Rust-native and free of WinRT-specific type shaping.
- The managed package can target normal desktop .NET consumers without packaged
  app requirements.
- The ABI stays small and explicit, which makes testing, packaging, and
  exception mapping easier.
- Future interop work can add lower-level ABI structs or callback contracts only
  where a concrete need justifies the extra complexity.
