# Rheo Storage Agents

## Purpose
- This repository is the Rust-first rewrite of `Rheo.Storage`.
- The rewrite prioritizes secure, idiomatic Rust APIs over C# surface compatibility.
- Milestone one is limited to content-based file analysis and immutable file metadata.

## Read Order
1. `docs/ai/index.md`
2. `docs/ai/project-guidelines.md`
3. `docs/adr/ADR-001-rust-first-rewrite.md`
4. `docs/reference/legacy-rheo-storage.md`
5. `docs/ai/continuous-learning.md`

## Guardrails
- Keep the public API Rust-native. Do not recreate `FileObject` or `DirectoryObject` class shapes unless an ADR changes that decision.
- Treat Windows as the primary runtime target for now, but keep pure analysis logic portable when it naturally can be.
- Do not add FFI, DLL exports, async runtime choices, or file watcher behavior in milestone one.
- Prefer typed errors, deterministic test fixtures, and explicit handling of file-sharing semantics on Windows.
- Keep project-specific AI guidance in this repo, not in a global skill.

## Current Milestone
- `rheo_storage_lib` owns analysis, immutable metadata, file and directory operations, navigation, debounced watching, typed errors, and legacy definitions loading.
- The operations layer should keep the simple path fast: do not force info or analysis loading for plain copy, move, read, write, or delete calls.
- `rheo_storage_def_builder` owns package inspection, normalization, and TrID XML source ingestion from `.xml`, extracted directories, and `.7z` archives.
- `rheo_storage_com` and `rheo_storage_winrt` are wrapper layers over the Rust core and must stay thin.

## Working Rhythm
- Update `docs/ai/continuous-learning.md` whenever a repeated implementation lesson appears.
- Record durable architecture decisions under `docs/adr/`.
- If a change shifts scope across milestones, update both `docs/ai/project-guidelines.md` and the relevant ADR in the same change.
