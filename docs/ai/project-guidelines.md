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
- Exclude:
  - mutating file operations
  - directory recursion APIs
  - watchers and debouncing
  - Win32 shell metadata
  - .NET interop and exported DLL design

## Windows-First Guardrails
- Use shared file-opening behavior on Windows so analysis does not create avoidable locking failures.
- Keep OS-specific code isolated at the boundary. The detection engine itself should stay portable.

## Documentation Split
- Use rustdoc comments for public Rust items.
- Use `docs/` for AI guidance, ADRs, and migration notes.
- Avoid duplicating the same guidance in both places.

## Testing Expectations
- Prefer small, deterministic fixtures.
- Test behavior that matters to API consumers: ranking, fallback, MIME/extension selection, and Windows sharing behavior.
- Add new fixture files only when they improve clarity more than in-test byte literals would.
