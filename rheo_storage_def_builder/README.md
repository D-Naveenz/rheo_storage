# rheo_storage_def_builder

`rheo_storage_def_builder` is the CLI tool that builds and inspects Rheo
definitions packages.

It supports:

- building reduced `filedefs.rpkg` packages from TrID XML inputs
- reading `.xml`, extracted directories, and `.7z` archives
- normalizing and verifying existing package files
- interactive terminal use through the Rheo shell
- script-friendly one-shot CLI commands

## Default Working Folders

When the executable runs from a normal Cargo output folder, it uses
executable-relative defaults:

- `package/` for source discovery
- `output/` for generated package files
- `logs/` for dated log files such as `2026-04-10_def_builder.log`

The local `package/` folder inside this crate is excluded from Cargo package
publishing and copied beside the built executable during a normal build.

## Launch Modes

- Run without a subcommand in a real terminal to open the interactive Rheo shell.
- Run with an explicit command to use the classic non-interactive CLI flow.

## Repository

- Workspace: <https://github.com/D-Naveenz/rheo_storage>
- Root documentation: <https://github.com/D-Naveenz/rheo_storage#readme>

## License

Licensed under Apache-2.0.
