# Continuous Learning

## 2026-04-09
- The legacy `filedefs.rpkg` package is already sufficient for a real milestone-one implementation, so the rewrite does not need to block on a new builder.
- The sharpest early risk in the old codebase is concurrency around file and directory mutation, not content analysis.
- Keeping AI guidance in `docs/` avoids collisions with rustdoc and gives the repo a clear shared memory path.

## 2026-04-10
- File metadata should stay cheap by default; content analysis belongs behind a lazy cache instead of inside `FileInfo::from_path`.
- A folder-backed `info/` module tree is a better fit than a single `info.rs` file once file and directory information start diverging.
