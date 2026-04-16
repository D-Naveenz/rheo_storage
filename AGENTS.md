# Rheo Storage Agents

## Purpose
- This repository is the Rust-first rewrite of `Rheo.Storage`.
- The rewrite prioritizes secure, idiomatic Rust APIs over C# surface compatibility.
- The current repo already includes the core runtime, the reusable `rheo_tool` platform, and the .NET/native FFI delivery layers.

## Read Order
1. `docs/ai/index.md`
2. `docs/ai/project-guidelines.md`
3. `docs/adr/ADR-001-rust-first-rewrite.md`
4. `docs/reference/legacy-rheo-storage.md`
5. `docs/ai/continuous-learning.md`

## Guardrails
- Keep the public API Rust-native. Do not recreate `FileObject` or `DirectoryObject` class shapes unless an ADR changes that decision.
- Treat Windows as the primary runtime target for now, but keep pure analysis logic portable when it naturally can be.
- Keep ABI-layer constraints out of the Rust core. Solve WinRT-specific shape issues in `rheo_storage_winrt`.
- Prefer typed errors, deterministic test fixtures, and explicit handling of file-sharing semantics on Windows.
- Keep project-specific AI guidance in this repo, not in a global skill.

## Current Milestone
- `rheo_storage` owns analysis, immutable metadata, file and directory operations, navigation, debounced watching, typed errors, and filedefs package deserialization/loading.
- The operations layer should keep the simple path fast: do not force info or analysis loading for plain copy, move, read, write, or delete calls.
- `rheo_rpkg` owns the generic MessagePack-based `RPKG` v2 container format, package purposes, compression, section layout, and integrity framing.
- `rheo_tool_rheo_storage` owns package inspection, normalization, filedefs package serialization, vendored MIME validation, floodgate reduction, interactive shell flows, and TrID XML source ingestion from `.xml`, extracted directories, and `.7z` archives.
- `filedefs.rpkg` is now a single `RPKG` v2 filedefs package; backward-compatibility readers for older ad hoc package formats are intentionally out of scope.
- `rheo_storage_ffi` and `bindings/dotnet/Rheo.Storage` are the active ABI and managed wrapper layers over the Rust core and should keep ABI-specific concerns out of `rheo_storage`.
- CI, docs, and release automation should assume Windows as the primary build/test/doc platform.
- `filedefs.rpkg` and other large runtime fixtures may be tracked with Git LFS, so CI and release workflows must explicitly enable LFS checkout when those assets are needed.

## Working Rhythm
- Update `docs/ai/continuous-learning.md` whenever a repeated implementation lesson appears.
- Record durable architecture decisions under `docs/adr/`.
- If a change shifts scope across milestones, update both `docs/ai/project-guidelines.md` and the relevant ADR in the same change.
