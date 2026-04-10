# AI Knowledge Index

## Primary Guides
- [`project-guidelines.md`](./project-guidelines.md): implementation rules for the Rust rewrite.
- [`continuous-learning.md`](./continuous-learning.md): lessons captured while working incrementally.

## Architecture Decisions
- [`../adr/ADR-001-rust-first-rewrite.md`](../adr/ADR-001-rust-first-rewrite.md): Rust-native API, Windows-first scope, no immediate FFI.

## Legacy Context
- [`../reference/legacy-rheo-storage.md`](../reference/legacy-rheo-storage.md): summary of the original C# feature families and migration boundaries.

## Current Focus
- Keep `rheo_storage_lib` as the source of truth for analysis, metadata, operations, enumeration, and watching.
- Use `rheo_storage_def_builder` to inspect and normalize the runtime package format.
- Keep ABI layers thin: `rheo_storage_com` for classic COM consumers and `rheo_storage_winrt` for WinRT-facing packaging work.
