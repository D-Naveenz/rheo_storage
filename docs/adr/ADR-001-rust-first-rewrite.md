# ADR-001: Rust-First Rewrite

## Status
Accepted

## Context
`Rheo.Storage` began as a C# library with deep Windows integration, broad file-system features, and difficult concurrency behavior around file and directory mutation. The Rust rewrite needs an implementation strategy that is safer, easier to reason about, and suitable for incremental learning.

## Decision
- Build a Rust-native public API rather than mirroring the C# object model.
- Optimize for Windows first while keeping the pure analysis engine portable.
- Start with content-based analysis and immutable metadata before any mutating file or directory features.
- Once mutation APIs are added, keep a single synchronous core and layer optional async wrappers over it instead of maintaining duplicate behavior trees.
- Defer FFI and exported DLL concerns until the Rust core is stable and worth wrapping.
- Keep the generic `RPKG` v2 container in a shared crate, `rheo_rpkg`, while letting `rheo_storage` remain a read-side consumer of filedefs packages and `rheo_storage_def_builder` remain the write-side producer from TrID XML sources.

## Consequences
- Early milestones can focus on correctness and ergonomics without carrying legacy API constraints.
- The runtime library can reuse the legacy definitions asset immediately, and the builder can evolve independently as long as it continues emitting the same package contract.
- Mutation APIs should keep the simple path cheap by avoiding implicit info or analysis loading.
- Future milestones may add watchers and builder tooling, but each must fit the Rust-first architecture rather than force a class-for-class port.
