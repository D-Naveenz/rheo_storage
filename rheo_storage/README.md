# rheo_storage

`rheo_storage` is the Rust-first core crate for the Rheo Storage rewrite.

It provides:

- content-based file analysis using the bundled `filedefs.rpkg`
- immutable file and directory information models
- file and directory operations with a fast path for simple work
- navigation helpers and debounced watching primitives
- Windows-aware behavior without forcing the whole crate into an ABI-first design
- read-side filedefs package loading through the shared `rheo_rpkg` container crate

## Scope

The crate is the source of truth for runtime behavior in this repository. Higher
level wrappers such as `rheo_storage_winrt` should stay thin and delegate to
this crate.

## Features

- `async-tokio`: enables Tokio-backed async wrappers over the sync operation core

## Repository

- Workspace: <https://github.com/D-Naveenz/rheo_storage>
- Consumer docs: <https://github.com/D-Naveenz/rheo_storage/tree/main/docs/reference/rust-consumer.md>

## License

Licensed under Apache-2.0.
