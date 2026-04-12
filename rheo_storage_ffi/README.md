# rheo_storage_ffi

`rheo_storage_ffi` exposes a Windows-first native C ABI over the Rust core in
`rheo_storage`.

It is the primary interop layer for the managed `Rheo.Storage` .NET wrapper and
keeps ABI concerns out of the Rust-native core crate.
