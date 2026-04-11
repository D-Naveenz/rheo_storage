# rheo_storage_winrt

`rheo_storage_winrt` is the WinRT-facing wrapper crate for Rheo Storage.

Its job is to expose a Windows Runtime friendly surface over the Rust core in
`rheo_storage` while keeping business logic in the core crate.

## Status

This crate is intentionally thin and still evolving. The runtime behavior,
analysis rules, and storage operations live in `rheo_storage`.

## Repository

- Workspace: <https://github.com/D-Naveenz/rheo_storage>
- WinRT consumer notes: <https://github.com/D-Naveenz/rheo_storage/tree/main/docs/reference/winrt-consumer.md>

## License

Licensed under Apache-2.0.
