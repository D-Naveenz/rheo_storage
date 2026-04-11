# Continuous Learning

## 2026-04-09
- The legacy `filedefs.rpkg` package is already sufficient for a real milestone-one implementation, so the rewrite does not need to block on a new builder.
- The sharpest early risk in the old codebase is concurrency around file and directory mutation, not content analysis.
- Keeping AI guidance in `docs/` avoids collisions with rustdoc and gives the repo a clear shared memory path.

## 2026-04-10
- File metadata should stay cheap by default; content analysis belongs behind a lazy cache instead of inside `FileInfo::from_path`.
- A folder-backed `info/` module tree is a better fit than a single `info.rs` file once file and directory information start diverging.
- Progress reporting should stay opt-in and isolated so basic file operations can take the fastest direct path without pre-scans or buffered callbacks.
- A single synchronous operations core plus optional async wrappers is easier to reason about than separate sync and async mutation implementations.
- Directory watching belongs behind a debounced event boundary so callers get stable change notifications without inheriting raw watcher noise.
- Shared package types between the runtime and builder are safer than re-describing the MessagePack contract in multiple crates.
- TrID XML is structurally simple enough to parse directly, but the builder should tolerate messy source data such as repeated optional nodes instead of assuming every file is perfectly normalized.
- On Windows, the system `tar` tool can handle `.7z` archives well enough for builder ingestion, which avoids forcing contributors to manually unpack tens of thousands of TrID XML files into the repo.
- A vendored MIME snapshot keeps transformation runs deterministic and offline while still letting the builder correct broken TrID MIME values back to canonical forms.
- Upgrading `rpkg` to a compressed format is safest when the runtime dual-reads both old and new payloads, so package improvements do not force a flag day migration.

## 2026-04-11
- If a repo is explicitly Windows-first, keep CI and docs generation aligned with that truth instead of preserving cross-platform jobs just for symmetry.
- GitHub Actions must enable Git LFS checkout when tests or embedded assets depend on large binary resources; otherwise LFS pointer text can masquerade as malformed runtime data.
- Builder UI tests should create the files they depend on instead of assuming a local `package/` folder is already populated on CI runners.
- Strict clippy settings in CI are useful, but they surface a lot of small style regressions; running the exact package-level clippy commands locally is the fastest way to stabilize the workflow before pushing.
