# rheo_storage

`rheo_storage` is the Rust-native runtime crate for Rheo Storage.

It owns the core behavior for:

- content-based file analysis using the bundled `filedefs.rpkg`
- immutable file and directory information snapshots
- sync-first file and directory operations with optional progress
- path-based storage handles for ergonomic navigation
- debounced directory watching
- structured `tracing` instrumentation for runtime diagnostics

Higher-level delivery layers such as `rheo_storage_ffi` and `bindings/dotnet/Rheo.Storage`
should stay thin and delegate behavior back to this crate.

## Install

```toml
[dependencies]
rheo_storage = "0.2.0"
```

## Quick Start

```rust
use rheo_storage::{DirectoryStorage, FileStorage, analyze_path};

let report = analyze_path("sample.png")?;
let file = FileStorage::from_existing("sample.png")?;
let bytes = file.read()?;

let directory = DirectoryStorage::from_existing(".")?;
let files = directory.files()?;
# let _ = (report, bytes, files);
# Ok::<(), rheo_storage::StorageError>(())
```

## Progress-Aware Operations

```rust
use std::sync::Arc;
use rheo_storage::{FileStorage, StorageProgress, TransferOptions};

let progress = Arc::new(|update: StorageProgress| {
    println!("{} bytes", update.bytes_transferred);
});

FileStorage::from_existing("input.bin")?.copy_to_with_options(
    "output.bin",
    TransferOptions {
        overwrite: true,
        buffer_size: None,
        progress: Some(progress),
        cancellation_token: None,
    },
)?;
# Ok::<(), rheo_storage::StorageError>(())
```

## Logging

`rheo_storage` uses `tracing` for structured runtime logs. The crate emits events for:

- analysis and definition-package loading
- file and directory operations
- metadata and summary loading
- watcher lifecycle events

Applications can install any standard `tracing` subscriber before calling into the crate.

## Features

- `async-tokio`: enables Tokio-backed async wrappers over the sync operations core

## Related Docs

- Workspace overview: <https://github.com/D-Naveenz/rheo_storage>
- Rust consumer guide: <https://github.com/D-Naveenz/rheo_storage/tree/main/docs/reference/rust-consumer.md>
- Native ABI wrapper: <https://github.com/D-Naveenz/rheo_storage/tree/main/rheo_storage_ffi>
