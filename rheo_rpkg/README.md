# rheo_rpkg

`rheo_rpkg` is the shared `RPKG` v2 container crate used across the Rheo workspace.

It stores MessagePack payloads together with:

- optional MessagePack metadata
- optional integrity data
- explicit package purposes that guide default read behavior
- optional payload compression

## Install

```toml
[dependencies]
rheo_rpkg = "0.2.0"
```

## Quick Start

```rust
use rheo_rpkg::{
    CompressionKind, IntegrityKind, PackagePurpose, RpkgReader, RpkgWriteOptions, RpkgWriter,
};

let bytes = RpkgWriter::write_payload(
    &vec!["alpha".to_string(), "beta".to_string()],
    &RpkgWriteOptions {
        package_id: *b"CONF",
        purpose: PackagePurpose::Standard,
        compression: CompressionKind::Lz4Frame,
        flags: 0,
        metadata: None,
        integrity: IntegrityKind::Sha256,
    },
)?;

let decoded: Vec<String> = RpkgReader::decode_payload(&bytes)?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## Read Profiles

- `PackagePurpose::Standard`: verify integrity and load metadata by default
- `PackagePurpose::FastPayload`: favor low-overhead payload access
- `PackagePurpose::Embedded`: favor lightweight embedded runtime reads

## Intended Use

`rheo_rpkg` is intentionally generic. It does not know about TrID definitions,
Rheo metadata models, or any specific runtime payload schema. Those concerns live
in higher-level crates such as `rheo_storage` and `rheo_tool_rheo_storage`.
