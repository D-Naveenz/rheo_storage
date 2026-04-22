# Dhara Storage Agents

## Purpose
- `dhara_storage` is the Rust-first rewrite of `Dhara.Storage`.
- The workspace now includes the core runtime, the reusable `dhara_tool` platform, and the active .NET/FFI delivery layers.
- Canonical AI-operational knowledge for this workspace lives in MindVault, not in repo-local notes.

## Vault Location
- `%USERPROFILE%\OneDrive\Documents\MindVault`

## Read Order
1. `%USERPROFILE%\OneDrive\Documents\MindVault\AI\Workspaces\rheo-storage\Home.md`
2. `%USERPROFILE%\OneDrive\Documents\MindVault\AI\Workspaces\rheo-storage\Overview.md`
3. `%USERPROFILE%\OneDrive\Documents\MindVault\AI\Workspaces\rheo-storage\Guardrails.md`

## Local Caveats
- Treat repo code, manifests, tests, and workflow files as the source of truth if a vault note drifts.
- Keep `dhara_storage` Rust-native; solve .NET interop constraints in `dhara_storage_native` and `bindings/dotnet/Dhara.Storage`.
- Keep `dhara_tool` and `dhara.config.toml` as the supported operator surface for config sync, verification, packaging, and publishing flows.
- Treat Windows as the primary runtime and CI target unless a concrete portability goal says otherwise.
- Avoid rebuilding repo-local AI docs unless a non-AI publishing or tooling requirement explicitly depends on them.
