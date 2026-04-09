# Legacy Rheo.Storage Summary

## Major Feature Families
- File operations: copy, move, rename, delete, write, sync/async progress reporting.
- Directory operations: enumeration, recursive copy/move/delete, change watching, debouncing.
- Information layer: storage metadata, formatted sizes, platform-specific details.
- Analysis layer: signature-based file type detection, MIME inference, extension ranking, text/binary fallback.
- Definitions builder: ETL pipeline that produces `filedefs.rpkg`.

## What Carries Forward
- Signature-based file analysis.
- Security-oriented actual-type detection.
- Progressively richer metadata, but only after the stable analysis core exists.

## What Does Not Carry Forward 1:1
- The C# `FileObject` and `DirectoryObject` surface.
- Implicit coupling between runtime library behavior and .NET-specific conventions.
- Early reliance on watchers or mutation-heavy concurrency patterns.

## Migration Boundary for Milestone One
- The Rust rewrite uses the legacy `filedefs.rpkg` package as input only.
- The C# implementation remains the reference for behavior ideas, not the public API shape.
