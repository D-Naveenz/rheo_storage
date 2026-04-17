# Releasing Rheo.Storage for .NET

`Rheo.Storage` is delivered through a split workflow model so package validation
and package publication are treated as separate concerns.

## Source of truth

- Shared package metadata lives in [`rheo.config.toml`](../../rheo.config.toml).
- Local developer secrets live in `.env.local`, created from
  [`.env.example`](../../.env.example).
- [`rheo_tool`](../../tooling/rheo_tool) is the supported way to inspect and
  synchronize repository configuration, run CI-equivalent checks locally, and
  drive NuGet verification and publishing.

Useful commands:

```powershell
cargo run -p rheo_tool -- config show
cargo run -p rheo_tool -- verify release-config
cargo run -p rheo_tool -- config sync
cargo run -p rheo_tool -- version set --channel nuget 2.0.0
cargo run -p rheo_tool -- verify ci
cargo run -p rheo_tool -- verify package
```

## Workflow lanes

- `ci.yml`
  - pull-request validation only
  - runs the thin workflow wrapper around `cargo run -p rheo_tool -- verify ci`
- `package-verify.yml`
  - runs on `main` and manual dispatch
  - runs the thin workflow wrapper around `cargo run -p rheo_tool -- verify package`
  - uploads the produced `.nupkg` and `.snupkg` artifacts
- `publish-nuget.yml`
  - manual only
  - drives `cargo run -p rheo_tool -- release publish`
  - repeats package verification
  - optionally overrides the NuGet version
  - publishes to nuget.org using the `nuget-production` environment secret
  - restores the published package from nuget.org and reruns the smoke consumer
- `release-rust.yml`
  - manual only
  - validates the Rust workspace and managed wrapper tests before release
  - can either bump the Rust workspace version during the release or publish the
    version that is already committed on `main`
  - when the version is already managed locally through `cargo run -p rheo_tool -- version ...`,
    run the workflow with `bump_version=false`
  - runs `cargo release` in isolated mode, so release behavior is defined by the
    workflow inputs and flags instead of a checked-in `release.toml`

## Versioning

- Rust crate versioning and NuGet package versioning are stored separately under
  `[versions]` in `rheo.config.toml`.
- The first Rust-backed NuGet release is `2.0.0`.
- After changing versions in the config, run `cargo run -p rheo_tool -- config sync`
  so `Cargo.toml` and `bindings/dotnet/Rheo.Storage/Rheo.Storage.csproj` stay aligned.

## Secrets

- Commit only `.env.example`.
- Keep `.env.local` untracked.
- Store the real `NUGET_API_KEY` in the GitHub Environment configured by
  `rheo.config.toml`, currently `nuget-production`.
- Rotate any older credentials that were ever stored in legacy local files.
