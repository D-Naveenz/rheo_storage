# AI Knowledge Index

## Primary Guides
- [`project-guidelines.md`](./project-guidelines.md): implementation rules for the Rust rewrite.
- [`continuous-learning.md`](./continuous-learning.md): lessons captured while working incrementally.
- [`commit-message.md`](./commit-message.md): AI-facing prompt for generating commit messages from diffs.
- [`runbooks/index.md`](./runbooks/index.md): operational runbooks for repeatable repo workflows.

## Architecture Decisions
- [`../adr/ADR-001-rust-first-rewrite.md`](../adr/ADR-001-rust-first-rewrite.md): Rust-native API, Windows-first scope, no immediate FFI.
- [`../adr/ADR-002-dotnet-ffi-wrapper.md`](../adr/ADR-002-dotnet-ffi-wrapper.md): .NET exposure through a dedicated FFI layer instead of WinRT.

## Legacy Context
- [`../reference/legacy-rheo-storage.md`](../reference/legacy-rheo-storage.md): summary of the original C# feature families and migration boundaries.

## Codex Project Setup
- [`../../AGENTS.md`](../../AGENTS.md): durable repo entrypoint and read order for agents.
- [`../../.codex/environments/environment.toml`](../../.codex/environments/environment.toml): Codex app worktree setup, cleanup, and header actions.

## Current Focus
- Keep `rheo_storage` as the source of truth for analysis, metadata, operations, enumeration, and watching.
- Keep ABI concerns isolated in `rheo_storage_ffi` and the managed `bindings/dotnet/Rheo.Storage` wrapper.
- Use `rheo_tool` as the operator surface for defs, verification, package validation, and release workflows.
- Use the defs engine behind `rheo_tool defs` to inspect, normalize, build, and validate the runtime package format.
- Treat release automation and CI as part of project memory: Windows-only pipelines and Git LFS-aware checkout are intentional repo rules, not incidental implementation details.
