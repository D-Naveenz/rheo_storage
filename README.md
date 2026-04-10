# Rheo Storage

Rust-first rewrite of `Rheo.Storage`, focused on secure and idiomatic storage
APIs, definition-driven file analysis, and Windows-first integration.

## Workspace
- `rheo_storage`: core runtime library
- `rheo_storage_def_builder`: definitions package builder, validator, and normalization tool
- `rheo_storage_winrt`: WinRT-facing wrapper crate for packaged Windows consumers

## Crates

### `rheo_storage`
- Rust-native runtime crate for file analysis, metadata, operations, navigation, and watching.
- Uses the bundled `filedefs.rpkg` runtime package and supports both legacy plain MessagePack and newer Rheo LZ4-wrapped packages.

### `rheo_storage_def_builder`
- CLI application for building and inspecting Rheo definitions packages from TrID XML sources.
- Supports both interactive TUI usage and one-shot command execution.

### `rheo_storage_winrt`
- Thin ABI wrapper layer for WinRT-facing consumers.
- Delegates runtime behavior to `rheo_storage`.

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

## Consumer Docs
- [Rust consumer](./docs/reference/rust-consumer.md)
- [WinRT consumer](./docs/reference/winrt-consumer.md)
