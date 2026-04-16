# rheo_tool_rheo_storage

`rheo_tool_rheo_storage` is the repository-specific capability pack that powers
`rheo_tool` for this workspace.

It currently owns:

- building reduced `filedefs.rpkg` packages from TrID XML inputs
- reading `.xml`, extracted directories, and `.7z` archives
- normalizing and verifying existing package files
- refreshing the embedded runtime package with `sync-embedded`
- repository config/version synchronization
- package verification and publish flows for `Rheo.Storage`

## Default Working Folders

When `rheo_tool defs ...` runs against this capability pack, it uses these
repo-relative defaults:

- `package/` for source discovery
- `output/` for generated package files
- `logs/` for dated log files such as `2026-04-10_def_builder.log`

The local `package/` folder inside this crate is excluded from Cargo package
publishing and copied beside the built artifacts during a normal build.

## Repository

- Workspace: <https://github.com/D-Naveenz/rheo_storage>
- Root documentation: <https://github.com/D-Naveenz/rheo_storage#readme>

## License

Licensed under Apache-2.0.
