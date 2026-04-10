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
- Exclude:
  - transactional definitions building
  - .NET interop and exported DLL design
  - operation-triggered metadata or analysis loading unless the caller requests it

## ABI Layer Rules
- Keep COM and WinRT crates thin over the core runtime.
- Do not fork file-system behavior between ABI layers.
- If an ABI-specific type restriction appears, solve it in the wrapper crate rather than reshaping the Rust core.

## Windows-First Guardrails
- Use shared file-opening behavior on Windows so analysis does not create avoidable locking failures.
- Keep OS-specific code isolated at the boundary. The detection engine itself should stay portable.
- Prefer same-volume rename moves for the fast path, and only fall back to copy/delete when a cross-volume move requires it.

## Documentation Split
- Use rustdoc comments for public Rust items.
- Use `docs/` for AI guidance, ADRs, and migration notes.
- Avoid duplicating the same guidance in both places.

## Testing Expectations
- Prefer small, deterministic fixtures.
- Test behavior that matters to API consumers: ranking, fallback, MIME/extension selection, and Windows sharing behavior.
- Add new fixture files only when they improve clarity more than in-test byte literals would.
- For mutation APIs, verify both the low-overhead default path and the progress-enabled path.
