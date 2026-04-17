# Rheo Storage

Rheo Storage is a Rust-first storage runtime with a Windows-first delivery story.
It combines definition-driven file analysis, path-based file and directory operations,
debounced watching, a reusable `RPKG` package format, and a managed .NET wrapper over
the native core.

## Workspace

| Project | Purpose |
| --- | --- |
| `rheo_rpkg` | Shared `RPKG` v2 container crate for MessagePack payloads, optional metadata, and integrity sections |
| `rheo_storage` | Rust-native runtime for analysis, metadata, operations, navigation, and watching |
| `rheo_storage_ffi` | Thin C ABI over `rheo_storage` for managed and native hosts |
| `bindings/dotnet/Rheo.Storage` | `net10.0` wrapper over `rheo_storage_ffi` |
| `tooling/rheo_tool` | Operator CLI for verification, packaging, release, and defs workflows |
| `tooling/rheo_tool_rheo_storage` | Repository-specific capability pack used by `rheo_tool` |

## Highlights

- Rust-native public API in `rheo_storage`, not a class-for-class port of the legacy C# model
- Bundled `filedefs.rpkg` runtime package for content-based file analysis
- File and directory operations that keep the simple path fast and opt into progress only when needed
- Debounced directory watching for stable change notifications
- Structured logging with `tracing` in Rust, native log forwarding through `rheo_storage_ffi`, and host integration through `Microsoft.Extensions.Logging`
- Multi-runtime NuGet packaging for Windows `win-x64` and `win-arm64`

## Quick Start

Rust runtime:

```rust
use rheo_storage::{FileStorage, analyze_path};

let report = analyze_path("sample.pdf")?;
let bytes = FileStorage::from_existing("sample.pdf")?.read()?;
# Ok::<(), rheo_storage::StorageError>(())
```

.NET wrapper:

```csharp
using Microsoft.Extensions.Logging;
using Rheo.Storage;

using var loggerFactory = LoggerFactory.Create(builder => builder.AddConsole());
RheoStorage.UseLoggerFactory(loggerFactory);

var file = RheoStorage.File(@"C:\data\sample.pdf");
var analysis = file.Analyze();
var bytes = await file.ReadBytesAsync();
```

Tooling:

```powershell
cargo run -p rheo_tool -- verify ci
cargo run -p rheo_tool -- package verify
```

## Support Matrix

| Surface | Status |
| --- | --- |
| `rheo_rpkg` | Portable Rust crate |
| `rheo_storage` | Windows-first runtime; portable where the underlying functionality naturally is |
| `rheo_storage_ffi` | Windows-first native ABI |
| `Rheo.Storage` NuGet package | Windows `win-x64` and `win-arm64` only |

The NuGet package now fails clearly during package consumption for unsupported RIDs such as `win-x86`,
and the managed wrapper also throws a `PlatformNotSupportedException` when loaded outside Windows `x64` or `arm64`.

## Logging

- Rust crates emit structured `tracing` events for analysis, metadata loading, operations, watching, package verification, and release flows.
- `rheo_storage_ffi` exposes a native logger registration API that forwards JSON log records across the ABI.
- `Rheo.Storage` forwards both managed wrapper logs and native runtime logs into a host `ILoggerFactory`.
- `rheo_tool` and `rheo_tool_rheo_storage` now emit richer command, configuration, transfer, and verification logs for release diagnostics.

## Release Flow

- Shared release metadata lives in [rheo.config.toml](./rheo.config.toml).
- Local secrets belong in [.env.local](./.env.example), created from the example file.
- [tooling/rheo_tool](./tooling/rheo_tool/README.md) is the supported operator surface for config sync, verification, packaging, and publish flows.
- NuGet verification checks that both `runtimes/win-x64/native/rheo_storage_ffi.dll` and `runtimes/win-arm64/native/rheo_storage_ffi.dll` are present in the package.

## Docs

- [Rust consumer guide](./docs/reference/rust-consumer.md)
- [.NET consumer guide](./docs/reference/dotnet-consumer.md)
- [.NET release guide](./docs/reference/releasing-rheo-storage-dotnet.md)
- [rheo_rpkg README](./rheo_rpkg/README.md)
- [rheo_storage README](./rheo_storage/README.md)
- [rheo_storage_ffi README](./rheo_storage_ffi/README.md)
- [Rheo.Storage README](./bindings/dotnet/Rheo.Storage/README.md)
