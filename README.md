# Rheo Storage

Rust-first rewrite of `Rheo.Storage`, focused on secure and idiomatic storage
APIs, definition-driven file analysis, and Windows-first integration.

## Workspace
- `rheo_rpkg`: generic MessagePack-based `RPKG` v2 container crate
- `rheo_storage`: core runtime library
- `rheo_storage_ffi`: native C ABI wrapper crate for .NET and other FFI consumers
- `rheo_storage_def_builder`: definitions package builder, validator, and normalization tool

## Crates

### `rheo_rpkg`
- Generic `RPKG` v2 container crate for MessagePack payloads with optional metadata and integrity sections.
- Used by both the runtime and the builder without pulling TrID-specific behavior into `rheo_storage`.

### `rheo_storage`
- Rust-native runtime crate for file analysis, metadata, operations, navigation, and watching.
- Uses the bundled `filedefs.rpkg` runtime package and reads filedefs payloads through `rheo_rpkg`.

### `rheo_storage_ffi`
- Thin native interop layer over `rheo_storage`.
- Exposes a stable path-based C ABI with UTF-8 inputs, explicit memory-free helpers, JSON payloads for rich results, and raw byte buffers for file reads.

### `rheo_storage_def_builder`
- CLI application for building and inspecting Rheo definitions packages from TrID XML sources.
- Supports both interactive TUI usage and one-shot command execution.
- Owns filedefs package serialization plus embedded package refresh through `sync-embedded`.

## Builder Package Assets
- `rheo_storage_def_builder/package` is kept in the repo for large local builder inputs such as `triddefs_xml.7z`.
- That folder is excluded from Cargo package publishing, but `rheo_storage_def_builder` copies it into the active Cargo output directory during build.
- The copy target mirrors MSBuild-style output behavior, so after building you can expect `target/debug/package` or `target/release/package` beside the builder executable.
- The builder now uses executable-relative defaults:
  - `package/` for TrID source discovery
  - `output/` for generated `filedefs.rpkg`
  - `logs/` for dated log files such as `2026-04-10_def_builder.log`
- All three locations can still be overridden from the CLI with `--package-dir`, `--output-dir`, and `--logs-dir`, or by passing explicit `--input` and `--output` paths on commands that support them.
- Launching `rheo_storage_def_builder` without a subcommand in a real terminal now opens the interactive Rheo shell.
- Explicit subcommands still run directly, so scripting and automation remain compatible.

## Release Metadata

- Repository: <https://github.com/D-Naveenz/rheo_storage>
- License: Apache-2.0
- Rust edition: 2024

## Release Flow

- Workspace releases are configured through [`cargo release`](https://github.com/crate-ci/cargo-release).
- Root [release.toml](./release.toml) keeps the workspace on a shared version and creates tags in the form `v<version>`.
- GitHub Actions release automation lives in [release.yml](./.github/workflows/release.yml).
- The release workflow is manual by design:
  - run it with `execute = false` for a dry run
  - run it with `execute = true` to publish, tag, and push
- Executed releases require a `CARGO_REGISTRY_TOKEN` repository secret for crates.io publishing.

## Consumer Docs
- [Rust consumer](./docs/reference/rust-consumer.md)
- [.NET consumer](./docs/reference/dotnet-consumer.md)
