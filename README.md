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
- Native interop layer over `rheo_storage`.
- Exposes a path-based C ABI with UTF-8 inputs, explicit memory-free helpers, JSON payloads for rich results, operation handles for async/progress workflows, watch handles for directory monitoring, and write sessions for streamed uploads.

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

- Shared release metadata lives in [rheo.config.toml](./rheo.config.toml).
- Local developer secrets belong in `.env.local`, created from [.env.example](./.env.example).
- [`rheo_repo_tool`](./rheo_repo_tool) owns config sync, version edits, env bootstrapping, and release verification.
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
