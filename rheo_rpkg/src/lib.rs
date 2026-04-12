use std::io::{Cursor, Read, Write};

use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use serde::Serialize;
use serde::de::DeserializeOwned;
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const RPKG_MAGIC: &[u8; 4] = b"RPKG";
pub const RPKG_WIRE_VERSION: u8 = 2;
const HEADER_LEN: usize = 28;
const SECTION_DESCRIPTOR_LEN: usize = 24;
const PAYLOAD_FORMAT_MESSAGEPACK: u8 = 1;
const METADATA_FORMAT_MESSAGEPACK: u8 = 1;
const INTEGRITY_FORMAT_SHA256: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackagePurpose {
    Standard = 0,
    FastPayload = 1,
    Embedded = 2,
}

impl PackagePurpose {
    pub fn default_read_options(self) -> RpkgReadOptions {
        match self {
            Self::Standard => RpkgReadOptions::default(),
            Self::FastPayload | Self::Embedded => RpkgReadOptions {
                verify_integrity: false,
                load_metadata: false,
            },
        }
    }
}

impl TryFrom<u8> for PackagePurpose {
    type Error = RpkgDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Standard),
            1 => Ok(Self::FastPayload),
            2 => Ok(Self::Embedded),
            _ => Err(RpkgDecodeError::UnsupportedPurpose(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompressionKind {
    None = 0,
    Lz4Frame = 1,
}

impl TryFrom<u8> for CompressionKind {
    type Error = RpkgDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Lz4Frame),
            _ => Err(RpkgDecodeError::UnsupportedCompression(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrityKind {
    None = 0,
    Sha256 = 1,
}

impl TryFrom<u8> for IntegrityKind {
    type Error = RpkgDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::None),
            1 => Ok(Self::Sha256),
            _ => Err(RpkgDecodeError::UnsupportedIntegrity(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SectionKind {
    Payload = 1,
    Metadata = 2,
    Integrity = 3,
    ChunkIndex = 4,
}

impl TryFrom<u8> for SectionKind {
    type Error = RpkgDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Payload),
            2 => Ok(Self::Metadata),
            3 => Ok(Self::Integrity),
            4 => Ok(Self::ChunkIndex),
            _ => Err(RpkgDecodeError::UnsupportedSectionKind(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageHeader {
    pub wire_version: u8,
    pub package_id: [u8; 4],
    pub purpose: PackagePurpose,
    pub compression: CompressionKind,
    pub flags: u8,
    pub section_count: u16,
    pub section_table_offset: u32,
    pub section_table_len: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionDescriptor {
    pub kind: SectionKind,
    pub format: u8,
    pub offset: u64,
    pub length: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpkgReadOptions {
    pub verify_integrity: bool,
    pub load_metadata: bool,
}

impl Default for RpkgReadOptions {
    fn default() -> Self {
        Self {
            verify_integrity: true,
            load_metadata: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpkgWriteOptions {
    pub package_id: [u8; 4],
    pub purpose: PackagePurpose,
    pub compression: CompressionKind,
    pub flags: u8,
    pub metadata: Option<Vec<u8>>,
    pub integrity: IntegrityKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPackage {
    pub header: PackageHeader,
    pub sections: Vec<SectionDescriptor>,
    pub payload: Vec<u8>,
    pub metadata: Option<Vec<u8>>,
    pub integrity: IntegrityKind,
    pub integrity_verified: bool,
}

#[derive(Debug, Error)]
pub enum RpkgDecodeError {
    #[error("payload is not an RPKG v2 container")]
    UnsupportedFormat,
    #[error("payload is too short for an RPKG header")]
    TruncatedHeader,
    #[error("unsupported RPKG wire version: {0}")]
    UnsupportedWireVersion(u8),
    #[error("unsupported RPKG purpose: {0}")]
    UnsupportedPurpose(u8),
    #[error("unsupported RPKG compression kind: {0}")]
    UnsupportedCompression(u8),
    #[error("unsupported RPKG integrity kind: {0}")]
    UnsupportedIntegrity(u8),
    #[error("unsupported RPKG section kind: {0}")]
    UnsupportedSectionKind(u8),
    #[error("RPKG section table is malformed")]
    InvalidSectionTable,
    #[error("RPKG payload section is missing")]
    MissingPayloadSection,
    #[error("RPKG metadata section is malformed")]
    InvalidMetadataSection,
    #[error("RPKG integrity section is malformed")]
    InvalidIntegritySection,
    #[error("RPKG section bytes are out of range")]
    InvalidSectionRange,
    #[error("RPKG payload section does not use MessagePack format")]
    InvalidPayloadFormat,
    #[error("RPKG metadata section does not use MessagePack format")]
    InvalidMetadataFormat,
    #[error("RPKG payload checksum does not match integrity section")]
    ChecksumMismatch,
    #[error("failed to decompress RPKG payload: {0}")]
    Compression(#[from] std::io::Error),
    #[error("failed to decode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::decode::Error),
}

#[derive(Debug, Error)]
pub enum RpkgEncodeError {
    #[error("failed to encode MessagePack payload: {0}")]
    MessagePack(#[from] rmp_serde::encode::Error),
    #[error("failed to compress RPKG payload: {0}")]
    Compression(#[from] std::io::Error),
    #[error("failed to finalize RPKG compression frame: {0}")]
    CompressionFrame(#[from] lz4_flex::frame::Error),
}

pub struct RpkgReader;

impl RpkgReader {
    pub fn read_header(bytes: &[u8]) -> Result<PackageHeader, RpkgDecodeError> {
        if bytes.len() < HEADER_LEN {
            if bytes.starts_with(RPKG_MAGIC) {
                return Err(RpkgDecodeError::TruncatedHeader);
            }
            return Err(RpkgDecodeError::UnsupportedFormat);
        }
        if &bytes[..4] != RPKG_MAGIC {
            return Err(RpkgDecodeError::UnsupportedFormat);
        }
        let wire_version = bytes[4];
        if wire_version != RPKG_WIRE_VERSION {
            return Err(RpkgDecodeError::UnsupportedWireVersion(wire_version));
        }
        Ok(PackageHeader {
            wire_version,
            purpose: PackagePurpose::try_from(bytes[5])?,
            compression: CompressionKind::try_from(bytes[6])?,
            flags: bytes[7],
            package_id: [bytes[8], bytes[9], bytes[10], bytes[11]],
            section_count: u16::from_le_bytes(bytes[12..14].try_into().unwrap()),
            section_table_offset: u32::from_le_bytes(bytes[16..20].try_into().unwrap()),
            section_table_len: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
        })
    }

    pub fn read_sections(
        bytes: &[u8],
        header: &PackageHeader,
    ) -> Result<Vec<SectionDescriptor>, RpkgDecodeError> {
        let section_table_offset = usize::try_from(header.section_table_offset)
            .map_err(|_| RpkgDecodeError::InvalidSectionTable)?;
        let section_table_len = usize::try_from(header.section_table_len)
            .map_err(|_| RpkgDecodeError::InvalidSectionTable)?;
        let table_end = section_table_offset
            .checked_add(section_table_len)
            .ok_or(RpkgDecodeError::InvalidSectionTable)?;
        if table_end > bytes.len() || section_table_offset < HEADER_LEN {
            return Err(RpkgDecodeError::InvalidSectionTable);
        }
        if section_table_len != header.section_count as usize * SECTION_DESCRIPTOR_LEN {
            return Err(RpkgDecodeError::InvalidSectionTable);
        }

        let mut sections = Vec::with_capacity(header.section_count as usize);
        let mut cursor = section_table_offset;
        for _ in 0..header.section_count {
            let entry = &bytes[cursor..cursor + SECTION_DESCRIPTOR_LEN];
            sections.push(SectionDescriptor {
                kind: SectionKind::try_from(entry[0])?,
                format: entry[1],
                offset: u64::from_le_bytes(entry[4..12].try_into().unwrap()),
                length: u64::from_le_bytes(entry[12..20].try_into().unwrap()),
            });
            cursor += SECTION_DESCRIPTOR_LEN;
        }
        Ok(sections)
    }

    pub fn read_payload_bytes(bytes: &[u8]) -> Result<Vec<u8>, RpkgDecodeError> {
        let header = Self::read_header(bytes)?;
        let sections = Self::read_sections(bytes, &header)?;
        let payload = find_section(&sections, SectionKind::Payload)
            .ok_or(RpkgDecodeError::MissingPayloadSection)?;
        if payload.format != PAYLOAD_FORMAT_MESSAGEPACK {
            return Err(RpkgDecodeError::InvalidPayloadFormat);
        }
        let payload_slice = read_section_bytes(bytes, payload)?;
        decompress_payload(payload_slice, header.compression)
    }

    pub fn read_package(
        bytes: &[u8],
        options: &RpkgReadOptions,
    ) -> Result<DecodedPackage, RpkgDecodeError> {
        let header = Self::read_header(bytes)?;
        let sections = Self::read_sections(bytes, &header)?;
        let payload_desc = find_section(&sections, SectionKind::Payload)
            .ok_or(RpkgDecodeError::MissingPayloadSection)?;
        if payload_desc.format != PAYLOAD_FORMAT_MESSAGEPACK {
            return Err(RpkgDecodeError::InvalidPayloadFormat);
        }

        let effective_load_metadata = options.load_metadata;
        let effective_verify_integrity = options.verify_integrity;

        let payload_slice = read_section_bytes(bytes, payload_desc)?;
        let integrity_desc = find_section(&sections, SectionKind::Integrity);
        let integrity = match integrity_desc {
            Some(section) => IntegrityKind::try_from(section.format)?,
            None => IntegrityKind::None,
        };

        if effective_verify_integrity && integrity == IntegrityKind::Sha256 {
            verify_integrity(bytes, &header, &sections, payload_desc, integrity_desc)?;
        }

        let metadata = if effective_load_metadata {
            if let Some(section) = find_section(&sections, SectionKind::Metadata) {
                if section.format != METADATA_FORMAT_MESSAGEPACK {
                    return Err(RpkgDecodeError::InvalidMetadataFormat);
                }
                Some(read_section_bytes(bytes, section)?.to_vec())
            } else {
                None
            }
        } else {
            None
        };

        let compression = header.compression;
        Ok(DecodedPackage {
            header,
            sections,
            payload: decompress_payload(payload_slice, compression)?,
            metadata,
            integrity,
            integrity_verified: effective_verify_integrity && integrity == IntegrityKind::Sha256,
        })
    }

    pub fn read_package_default(bytes: &[u8]) -> Result<DecodedPackage, RpkgDecodeError> {
        let header = Self::read_header(bytes)?;
        let options = header.purpose.default_read_options();
        Self::read_package(bytes, &options)
    }

    pub fn decode_payload<T: DeserializeOwned>(bytes: &[u8]) -> Result<T, RpkgDecodeError> {
        let payload = Self::read_payload_bytes(bytes)?;
        Ok(rmp_serde::from_slice(&payload)?)
    }

    pub fn decode_payload_with_options<T: DeserializeOwned>(
        bytes: &[u8],
        options: &RpkgReadOptions,
    ) -> Result<T, RpkgDecodeError> {
        let package = Self::read_package(bytes, options)?;
        Ok(rmp_serde::from_slice(&package.payload)?)
    }
}

pub struct RpkgWriter;

impl RpkgWriter {
    pub fn write_payload_bytes(
        payload: &[u8],
        options: &RpkgWriteOptions,
    ) -> Result<Vec<u8>, RpkgEncodeError> {
        let payload_section = match options.compression {
            CompressionKind::None => payload.to_vec(),
            CompressionKind::Lz4Frame => {
                let mut encoder = FrameEncoder::new(Vec::new());
                encoder.write_all(payload)?;
                encoder.finish()?
            }
        };

        let mut sections = Vec::with_capacity(3);
        sections.push((
            SectionKind::Payload,
            PAYLOAD_FORMAT_MESSAGEPACK,
            payload_section,
        ));
        if let Some(metadata) = &options.metadata {
            sections.push((
                SectionKind::Metadata,
                METADATA_FORMAT_MESSAGEPACK,
                metadata.clone(),
            ));
        }
        if options.integrity == IntegrityKind::Sha256 {
            sections.push((SectionKind::Integrity, INTEGRITY_FORMAT_SHA256, vec![0; 32]));
        }

        let section_count = sections.len() as u16;
        let section_table_offset = HEADER_LEN as u32;
        let section_table_len = section_count as u32 * SECTION_DESCRIPTOR_LEN as u32;

        let mut current_offset = HEADER_LEN + section_table_len as usize;
        let mut descriptors = Vec::with_capacity(sections.len());
        for (kind, format, data) in &sections {
            descriptors.push(SectionDescriptor {
                kind: *kind,
                format: *format,
                offset: current_offset as u64,
                length: data.len() as u64,
            });
            current_offset += data.len();
        }

        let header = PackageHeader {
            wire_version: RPKG_WIRE_VERSION,
            package_id: options.package_id,
            purpose: options.purpose,
            compression: options.compression,
            flags: options.flags,
            section_count,
            section_table_offset,
            section_table_len,
        };

        let mut bytes = Vec::with_capacity(current_offset);
        bytes.extend_from_slice(&encode_header(&header));
        bytes.extend_from_slice(&encode_sections(&descriptors));
        for (_, _, data) in &sections {
            bytes.extend_from_slice(data);
        }

        if let Some(integrity_index) = descriptors
            .iter()
            .position(|descriptor| descriptor.kind == SectionKind::Integrity)
        {
            let digest = compute_integrity_digest(&bytes, &header, &descriptors, &descriptors[0]);
            let integrity = &descriptors[integrity_index];
            let start = integrity.offset as usize;
            bytes[start..start + 32].copy_from_slice(&digest);
        }

        Ok(bytes)
    }

    pub fn write_payload<T: Serialize>(
        value: &T,
        options: &RpkgWriteOptions,
    ) -> Result<Vec<u8>, RpkgEncodeError> {
        let payload = rmp_serde::to_vec(value)?;
        Self::write_payload_bytes(&payload, options)
    }
}

fn encode_header(header: &PackageHeader) -> [u8; HEADER_LEN] {
    let mut bytes = [0u8; HEADER_LEN];
    bytes[..4].copy_from_slice(RPKG_MAGIC);
    bytes[4] = header.wire_version;
    bytes[5] = header.purpose as u8;
    bytes[6] = header.compression as u8;
    bytes[7] = header.flags;
    bytes[8..12].copy_from_slice(&header.package_id);
    bytes[12..14].copy_from_slice(&header.section_count.to_le_bytes());
    bytes[16..20].copy_from_slice(&header.section_table_offset.to_le_bytes());
    bytes[20..24].copy_from_slice(&header.section_table_len.to_le_bytes());
    bytes
}

fn encode_sections(descriptors: &[SectionDescriptor]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(descriptors.len() * SECTION_DESCRIPTOR_LEN);
    for descriptor in descriptors {
        bytes.push(descriptor.kind as u8);
        bytes.push(descriptor.format);
        bytes.extend_from_slice(&0u16.to_le_bytes());
        bytes.extend_from_slice(&descriptor.offset.to_le_bytes());
        bytes.extend_from_slice(&descriptor.length.to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes
}

fn find_section(sections: &[SectionDescriptor], kind: SectionKind) -> Option<&SectionDescriptor> {
    sections.iter().find(|section| section.kind == kind)
}

fn read_section_bytes<'a>(
    bytes: &'a [u8],
    descriptor: &SectionDescriptor,
) -> Result<&'a [u8], RpkgDecodeError> {
    let start =
        usize::try_from(descriptor.offset).map_err(|_| RpkgDecodeError::InvalidSectionRange)?;
    let len =
        usize::try_from(descriptor.length).map_err(|_| RpkgDecodeError::InvalidSectionRange)?;
    let end = start
        .checked_add(len)
        .ok_or(RpkgDecodeError::InvalidSectionRange)?;
    if end > bytes.len() {
        return Err(RpkgDecodeError::InvalidSectionRange);
    }
    Ok(&bytes[start..end])
}

fn decompress_payload(
    payload: &[u8],
    compression: CompressionKind,
) -> Result<Vec<u8>, RpkgDecodeError> {
    match compression {
        CompressionKind::None => Ok(payload.to_vec()),
        CompressionKind::Lz4Frame => {
            let mut decoder = FrameDecoder::new(Cursor::new(payload));
            let mut output = Vec::new();
            decoder.read_to_end(&mut output)?;
            Ok(output)
        }
    }
}

fn verify_integrity(
    bytes: &[u8],
    header: &PackageHeader,
    sections: &[SectionDescriptor],
    payload: &SectionDescriptor,
    integrity: Option<&SectionDescriptor>,
) -> Result<(), RpkgDecodeError> {
    let Some(integrity) = integrity else {
        return Ok(());
    };
    if integrity.format != INTEGRITY_FORMAT_SHA256 || integrity.length != 32 {
        return Err(RpkgDecodeError::InvalidIntegritySection);
    }
    let expected = read_section_bytes(bytes, integrity)?;
    let actual = compute_integrity_digest(bytes, header, sections, payload);
    if actual.as_slice() != expected {
        return Err(RpkgDecodeError::ChecksumMismatch);
    }
    Ok(())
}

fn compute_integrity_digest(
    bytes: &[u8],
    header: &PackageHeader,
    sections: &[SectionDescriptor],
    payload: &SectionDescriptor,
) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(encode_header(header));
    hasher.update(encode_sections(sections));
    if let Ok(payload_bytes) = read_section_bytes(bytes, payload) {
        hasher.update(payload_bytes);
    }
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{
        CompressionKind, IntegrityKind, METADATA_FORMAT_MESSAGEPACK, PackageHeader, PackagePurpose,
        RpkgDecodeError, RpkgReadOptions, RpkgReader, RpkgWriteOptions, RpkgWriter, SectionKind,
    };

    #[test]
    fn fast_profile_roundtrips_payload_without_metadata() {
        let bytes = RpkgWriter::write_payload(
            &vec!["alpha".to_string(), "beta".to_string()],
            &RpkgWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::FastPayload,
                compression: CompressionKind::Lz4Frame,
                flags: 0,
                metadata: Some(rmp_serde::to_vec(&vec!["ignored"]).unwrap()),
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let payload: Vec<String> =
            RpkgReader::decode_payload(&bytes).expect("payload should decode");
        assert_eq!(payload, vec!["alpha", "beta"]);

        let package = RpkgReader::read_package_default(&bytes).expect("package should load");
        assert!(package.metadata.is_none());
        assert!(!package.integrity_verified);
    }

    #[test]
    fn full_profile_roundtrips_with_metadata_and_checksum() {
        let metadata = rmp_serde::to_vec(&vec![1u8, 2, 3]).unwrap();
        let bytes = RpkgWriter::write_payload(
            &vec!["alpha".to_string()],
            &RpkgWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::Lz4Frame,
                flags: 7,
                metadata: Some(metadata.clone()),
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let package = RpkgReader::read_package_default(&bytes).expect("package should load");
        assert_eq!(package.header.package_id, *b"CONF");
        assert_eq!(package.metadata, Some(metadata));
        assert!(package.integrity_verified);
    }

    #[test]
    fn checksum_mismatch_is_reported_for_full_profile() {
        let mut bytes = RpkgWriter::write_payload(
            &vec!["alpha".to_string()],
            &RpkgWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::None,
                flags: 0,
                metadata: None,
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;
        let error = RpkgReader::read_package_default(&bytes).expect_err("checksum should fail");
        assert!(matches!(error, RpkgDecodeError::ChecksumMismatch));
    }

    #[test]
    fn metadata_section_is_optional() {
        let bytes = RpkgWriter::write_payload(
            &42u32,
            &RpkgWriteOptions {
                package_id: *b"CONF",
                purpose: PackagePurpose::Standard,
                compression: CompressionKind::None,
                flags: 0,
                metadata: None,
                integrity: IntegrityKind::None,
            },
        )
        .expect("package should encode");

        let package = RpkgReader::read_package_default(&bytes).expect("package should load");
        assert_eq!(package.metadata, None);
        let payload: u32 = RpkgReader::decode_payload_with_options(
            &bytes,
            &RpkgReadOptions {
                verify_integrity: false,
                load_metadata: true,
            },
        )
        .expect("payload should decode");
        assert_eq!(payload, 42);
    }

    #[test]
    fn header_and_section_table_can_be_read_separately() {
        let metadata = rmp_serde::to_vec(&"meta").unwrap();
        let bytes = RpkgWriter::write_payload(
            &"payload",
            &RpkgWriteOptions {
                package_id: *b"FDEF",
                purpose: PackagePurpose::Embedded,
                compression: CompressionKind::None,
                flags: 5,
                metadata: Some(metadata),
                integrity: IntegrityKind::Sha256,
            },
        )
        .expect("package should encode");

        let header = RpkgReader::read_header(&bytes).expect("header should load");
        assert_eq!(
            header,
            PackageHeader {
                wire_version: 2,
                package_id: *b"FDEF",
                purpose: PackagePurpose::Embedded,
                compression: CompressionKind::None,
                flags: 5,
                section_count: 3,
                section_table_offset: 28,
                section_table_len: 72,
            }
        );

        let sections = RpkgReader::read_sections(&bytes, &header).expect("sections should load");
        assert_eq!(sections[0].kind, SectionKind::Payload);
        assert_eq!(sections[1].kind, SectionKind::Metadata);
        assert_eq!(sections[1].format, METADATA_FORMAT_MESSAGEPACK);
        assert_eq!(sections[2].kind, SectionKind::Integrity);
    }
}
