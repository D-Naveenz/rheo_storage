# rheo_tool

`rheo_tool` is the supported operator CLI for this repository.

It acts as the front door for:

- repo config and version synchronization
- local CI-style verification
- multi-runtime NuGet packaging and publish flows
- definitions package workflows
- interactive shell usage for common maintenance paths

## Examples

```powershell
cargo run -p rheo_tool -- verify ci
cargo run -p rheo_tool -- verify package
cargo run -p rheo_tool -- package verify
```

Launching `rheo_tool` without a subcommand in an interactive terminal opens the
Rheo shell.

## Logging

`rheo_tool` now emits richer structured logs for:

- command start and completion
- effective configuration
- spawned external processes
- package verification and publish milestones
- failures and validation details

Repository-specific command implementations live in `tooling/rheo_tool_rheo_storage`.
