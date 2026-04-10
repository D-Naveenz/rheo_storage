# WinRT Consumer Guide

Use `rheo_storage_winrt` as the dedicated WinRT-facing wrapper crate for packaged Windows consumers.

## Current Status
- The crate exists as the WinRT-specific layer boundary.
- Its current implementation is a thin Rust wrapper over `rheo_storage`.
- It is ready to host a packaged Windows Runtime component surface without changing the Rust core API shape.

## Intended Consumer
- Packaged WinUI / Windows App SDK desktop applications.

## Design Rule
- Keep WinRT-specific type restrictions and activation concerns in this crate.
- Do not push WinRT constraints down into `rheo_storage`.
