# ADR-001: Rust-First Rewrite

## Status
Accepted

## Context
`Rheo.Storage` began as a C# library with deep Windows integration, broad file-system features, and difficult concurrency behavior around file and directory mutation. The Rust rewrite needs an implementation strategy that is safer, easier to reason about, and suitable for incremental learning.

## Decision
- Build a Rust-native public API rather than mirroring the C# object model.
- Optimize for Windows first while keeping the pure analysis engine portable.
- Start with content-based analysis and immutable metadata before any mutating file or directory features.
- Defer FFI and exported DLL concerns until the Rust core is stable and worth wrapping.

## Consequences
- Early milestones can focus on correctness and ergonomics without carrying legacy API constraints.
- The runtime library can reuse the legacy definitions asset immediately, while the Rust definitions builder is postponed.
- Future milestones may add mutation APIs, watchers, and builder tooling, but each must fit the Rust-first architecture rather than force a class-for-class port.
