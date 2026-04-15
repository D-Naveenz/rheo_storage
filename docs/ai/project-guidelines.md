# Project Guidelines

## Rewrite Rules
- Build the library as if Rust were the original implementation language.
- Preserve useful concepts from the legacy project, not its object model.
- Prefer ownership, borrowing, iterators, and explicit result types over class-style wrappers.

## Milestone One Scope
- Include:
  - typed analysis errors
  - legacy definitions package loading
  - signature-based detection
  - text vs binary fallback heuristics
  - immutable file metadata
  - deterministic tests

## Milestone Two Scope
- Include:
  - sync file operations
  - optional-feature async wrappers for file and directory operations
  - progress reporting only when callers opt in
  - path-based storage handles that layer cleanly over the operation core
  - recursive directory copy, move, and delete
  - directory navigation and debounced watching
-  - operation-triggered metadata or analysis loading unless the caller requests it

## .NET Interop Scope
- Use `rheo_storage_ffi` as the only native ABI layer for .NET consumers.
- Keep the FFI surface path-based and UTF-8 oriented instead of mirroring Rust handle types.
- Keep rich FFI payloads JSON-based unless a clear performance bottleneck justifies lower-level ABI structs.
- Keep the managed `bindings/dotnet/Rheo.Storage` package ergonomic and modern without forcing .NET object-shape concerns back into `rheo_storage`.
- Support async workflows, progress reporting, explicit watching, and streamed writes through pull-based native handles rather than native-to-managed callbacks.
- Keep long-lived handle management and .NET-friendly orchestration in the managed wrapper instead of pushing .NET lifecycle rules into `rheo_storage`.
- Keep shared .NET package metadata and workflow settings in `rheo.config.toml`, then synchronize consumer projects through `rheo_repo_tool`.

## Builder Scope
- `rheo_storage_def_builder` should ingest TrID definitions directly from source material instead of assuming a prebuilt intermediate package.
- Support `.xml` files, extracted definition directories, and `.7z` archives without requiring users to manually unpack large definition sets into the repo.
- Preserve the logical `filedefs.rpkg` runtime contract even if the on-disk encoding changes, but keep generic package/container code in a shared crate instead of inside `rheo_storage`.
- Keep MIME validation offline and deterministic by vendoring a normalized IANA snapshot plus custom overlay types into the repo.
- Keep builder output deterministic even when source archives contain inconsistent XML such as repeated optional nodes.
- Keep the builder as a CLI application, not a reusable public library crate. Public API belongs in `rheo_storage`.
- Treat the interactive TUI and one-shot CLI as two front ends over one shared command runner instead of separate execution paths.

## ABI Layer Rules
- Keep ABI crates thin over the core runtime.
- Do not fork file-system behavior between ABI layers.
- If an ABI-specific type restriction appears, solve it in the wrapper crate rather than reshaping the Rust core.

## Windows-First Guardrails
- Use shared file-opening behavior on Windows so analysis does not create avoidable locking failures.
- Keep OS-specific code isolated at the boundary. The detection engine itself should stay portable.
- Prefer same-volume rename moves for the fast path, and only fall back to copy/delete when a cross-volume move requires it.
- Prefer Windows as the authoritative CI and release platform for this repo unless a specific portability goal justifies another runner.
- If a workflow or test depends on large binary assets tracked with Git LFS, explicitly enable LFS checkout in CI instead of assuming the files arrive as normal Git blobs.
- Split CI lanes by purpose: pull requests validate code health, `main` verifies package consumption, and publishing stays manual behind environment-scoped secrets.

## Documentation Split
- Use rustdoc comments for public Rust items.
- Use `docs/` for AI guidance, ADRs, and migration notes.
- Avoid duplicating the same guidance in both places.
- Keep release workflow knowledge in the repo root and `docs/ai` rather than only in GitHub Actions YAML, so future agents understand why the pipeline is shaped the way it is.
- Keep repo tooling usage documented near the workflows so version bumps and secret handling do not become tribal knowledge.

## Testing Expectations
- Prefer small, deterministic fixtures.
- Test behavior that matters to API consumers: ranking, fallback, MIME/extension selection, and Windows sharing behavior.
- Add new fixture files only when they improve clarity more than in-test byte literals would.
- For mutation APIs, verify both the low-overhead default path and the progress-enabled path.
