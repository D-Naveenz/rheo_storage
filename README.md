# Rheo Storage

Rust-first rewrite of `Rheo.Storage`, focused on secure and idiomatic storage
APIs, definition-driven file analysis, and Windows-first integration.

## Workspace
- `rheo_rpkg`: generic MessagePack-based `RPKG` v2 container crate
- `rheo_storage`: core runtime library
- `rheo_storage_ffi`: native C ABI wrapper crate for .NET and other FFI consumers
- `tooling/rheo_tool`: umbrella operator CLI for definitions, verification, packaging, and release flows
- `tooling/rheo_tool_core`: reusable command, registry, process, and report primitives for Rheo toolchains
- `tooling/rheo_tool_ui`: shared interactive shell UI over the tool registry
- `tooling/rheo_tool_rheo_storage`: this repository's defs/config/package/release capability pack

## Crates

### `rheo_rpkg`
- Generic `RPKG` v2 container crate for MessagePack payloads with optional metadata and integrity sections.
- Used by both the runtime and the builder without pulling TrID-specific behavior into `rheo_storage`.

### `rheo_storage`
- Rust-native runtime crate for file analysis, metadata, operations, navigation, and watching.
- Uses the bundled `filedefs.rpkg` runtime package and reads filedefs payloads through `rheo_rpkg`.

### `rheo_storage_ffi`
- Native interop layer over `rheo_storage`.
- Exposes a path-based C ABI with UTF-8 inputs, explicit memory-free helpers, JSON payloads for rich results, operation handles for async/progress workflows, watch handles for directory monitoring, and write sessions for streamed uploads.

### `rheo_tool`
- Primary operator CLI for this repository.
- Owns `defs`, `verify`, `package`, `release`, `config`, and `version` command groups.
- Supports direct commands for automation and an interactive shell when launched without a subcommand in a real terminal.

### `rheo_tool_core`
- Shared registry, command execution context, process helpers, and structured output model.
- Designed for reuse by other Rheo repositories without pulling in repo-specific defs or package logic.

### `rheo_tool_ui`
- Shared interactive shell layer over `rheo_tool_core`.
- Keeps section navigation and prompt handling out of repo-specific capability code.

### `rheo_tool_rheo_storage`
- Repository-specific command capability pack for config sync, defs workflows, CI/package verification, and release packaging.
- Owns the TrID/package assets and defs engine that used to live in the old standalone builder app.

## Builder Package Assets
- `tooling/rheo_tool_rheo_storage/package` is kept in the repo for large local builder inputs such as `triddefs_xml.7z`.
- That folder is excluded from Cargo package publishing, but `rheo_tool_rheo_storage` copies it into the active Cargo output directory during build.
- The copy target mirrors MSBuild-style output behavior, so after building you can expect `target/debug/package` or `target/release/package` beside the builder executable.
- The shared defs workflow now uses executable-relative defaults:
  - `package/` for TrID source discovery
  - `output/` for generated `filedefs.rpkg`
  - `logs/` for dated log files such as `2026-04-10_def_builder.log`
- All three locations can still be overridden from `rheo_tool` with `--package-dir`, `--output-dir`, and `--logs-dir`, or by passing explicit `--input` and `--output` paths on commands that support them.
- Launching `rheo_tool` without a subcommand in a real terminal opens the interactive Rheo shell.
- Explicit subcommands still run directly, so scripting and automation remain compatible.

## Release Metadata

- Repository: <https://github.com/D-Naveenz/rheo_storage>
- License: Apache-2.0
- Rust edition: 2024

## Release Flow

- Shared release metadata lives in [rheo.config.toml](./rheo.config.toml).
- Local developer secrets belong in `.env.local`, created from [.env.example](./.env.example).
- [`rheo_tool`](./tooling/rheo_tool) is the supported operator surface for config sync, version edits, env bootstrapping, verification, package validation, and publish flows.
- GitHub Actions delivery lanes are split by responsibility:
  - [ci.yml](./.github/workflows/ci.yml): pull request validation only
  - [package-verify.yml](./.github/workflows/package-verify.yml): build and verify a consumable NuGet package on `main`
  - [publish-nuget.yml](./.github/workflows/publish-nuget.yml): manual NuGet publish with consumer-side verification
  - [release-rust.yml](./.github/workflows/release-rust.yml): manual crates.io workspace release
- Root [release.toml](./release.toml) still configures shared Rust crate releases and tags in the form `v<version>`.
- See [releasing-rheo-storage-dotnet.md](./docs/reference/releasing-rheo-storage-dotnet.md) for the .NET package flow.

## Consumer Docs
- [Rust consumer](./docs/reference/rust-consumer.md)
- [.NET consumer](./docs/reference/dotnet-consumer.md)
- [.NET release operations](./docs/reference/releasing-rheo-storage-dotnet.md)
