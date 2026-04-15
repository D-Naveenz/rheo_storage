# Releasing Rheo.Storage for .NET

`Rheo.Storage` is delivered through a split workflow model so package validation
and package publication are treated as separate concerns.

## Source of truth

- Shared package metadata lives in [`rheo.config.toml`](../../rheo.config.toml).
- Local developer secrets live in `.env.local`, created from
  [`.env.example`](../../.env.example).
- [`rheo_repo_tool`](../../rheo_repo_tool) is the supported way to inspect and
  synchronize repository configuration.

Useful commands:

```powershell
cargo run -p rheo_repo_tool -- show
cargo run -p rheo_repo_tool -- verify release
cargo run -p rheo_repo_tool -- sync
cargo run -p rheo_repo_tool -- version set --channel nuget 2.0.0
```

## Workflow lanes

- `ci.yml`
  - pull-request validation only
  - runs formatting, clippy, Rust tests, repo-tool tests, and
    `Rheo.Storage.Tests`
- `package-verify.yml`
  - runs on `main` and manual dispatch
  - packs `Rheo.Storage`
  - inspects the `.nupkg`
  - restores and runs the committed smoke consumer from the produced package
  - publishes and runs the same smoke consumer with Native AOT
- `publish-nuget.yml`
  - manual only
  - repeats package verification
  - optionally overrides the NuGet version
  - publishes to nuget.org using the `nuget-production` environment secret
  - restores the published package from nuget.org and reruns the smoke consumer

## Versioning

- Rust crate versioning and NuGet package versioning are stored separately under
  `[versions]` in `rheo.config.toml`.
- The first Rust-backed NuGet release is `2.0.0`.
- After changing versions in the config, run `cargo run -p rheo_repo_tool -- sync`
  so `Cargo.toml` and `bindings/dotnet/Rheo.Storage/Rheo.Storage.csproj` stay aligned.

## Secrets

- Commit only `.env.example`.
- Keep `.env.local` untracked.
- Store the real `NUGET_API_KEY` in the GitHub Environment configured by
  `rheo.config.toml`, currently `nuget-production`.
- Rotate any older credentials that were ever stored in legacy local files.
