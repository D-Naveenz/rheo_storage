# AI Knowledge Index

## Primary Guides
- [`project-guidelines.md`](./project-guidelines.md): implementation rules for the Rust rewrite.
- [`continuous-learning.md`](./continuous-learning.md): lessons captured while working incrementally.

## Architecture Decisions
- [`../adr/ADR-001-rust-first-rewrite.md`](../adr/ADR-001-rust-first-rewrite.md): Rust-native API, Windows-first scope, no immediate FFI.

## Legacy Context
- [`../reference/legacy-rheo-storage.md`](../reference/legacy-rheo-storage.md): summary of the original C# feature families and migration boundaries.

## Current Focus
- Build out content-based analysis and immutable file metadata in `rheo_storage_lib`.
- Keep the definition builder deferred until the runtime package consumer is proven.
