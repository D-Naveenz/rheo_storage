use std::io::{Cursor, Read, Write};

use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

pub const RPKG_MAGIC: &[u8; 4] = b"RPKG";
pub const RPKG_WIRE_VERSION: u8 = 2;
const HEADER_LEN: usize = 28;

/// Supported payload serialization methods for an `RPKG` container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SerializationKind {
    MessagePack = 1,
}

impl TryFrom<u8> for SerializationKind {
    type Error = RpkgDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::MessagePack),
            _ => Err(RpkgDecodeError::UnsupportedSerialization(value)),
        }
    }
}

/// Supported payload compression methods for an `RPKG` container.
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

/// Declares whether a package is intended for external distribution or embedding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackagePurpose {
    External = 0,
    Embedded = 1,
}

impl TryFrom<u8> for PackagePurpose {
    type Error = RpkgDecodeError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::External),
            1 => Ok(Self::Embedded),
            _ => Err(RpkgDecodeError::UnsupportedPurpose(value)),
        }
    }
}

/// Controls whether package checksum verification runs while decoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerificationMode {
    Default,
    Always,
    Skip,
}

impl VerificationMode {
    fn should_verify(self, purpose: PackagePurpose) -> bool {
        match self {
            Self::Always => true,
            Self::Skip => false,
            Self::Default => purpose == PackagePurpose::External,
        }
    }
}

/// Metadata stored alongside an `RPKG` payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackageMetadata {
    pub package_version: String,
    pub source_version: String,
    pub package_revision: u16,
    pub checksum_sha256: [u8; 32],
}

impl PackageMetadata {
    pub fn new(
        package_version: impl Into<String>,
        source_version: impl Into<String>,
        package_revision: u16,
    ) -> Self {
        Self {
            package_version: package_version.into(),
            source_version: source_version.into(),
            package_revision,
            checksum_sha256: [0; 32],
        }
    }
}

/// Options used while encoding a payload into `RPKG` bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RpkgEncodeOptions {
    pub package_id: [u8; 4],
    pub serialization: SerializationKind,
    pub compression: CompressionKind,
    pub purpose: PackagePurpose,
    pub metadata: PackageMetadata,
}

/// Fully decoded `RPKG` data, including container metadata and payload bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedPackage {
    pub package_id: [u8; 4],
    pub serialization: SerializationKind,
    pub compression: CompressionKind,
    pub purpose: PackagePurpose,
    pub metadata: PackageMetadata,
    pub payload: Vec<u8>,
}

impl DecodedPackage {
    pub fn package_id_string(&self) -> String {
        String::from_utf8_lossy(&self.package_id).into_owned()
    }
}

/// Errors that can occur while decoding an `RPKG` container.
#[derive(Debug, Error)]
pub enum RpkgDecodeError {
    #[error("payload is not an RPKG v2 container")]
    UnsupportedFormat,

    #[error("payload is too short for an RPKG header")]
    TruncatedHeader,

    #[error("unsupported RPKG wire version: {0}")]
    UnsupportedWireVersion(u8),

    #[error("unsupported RPKG serialization kind: {0}")]
    UnsupportedSerialization(u8),

    #[error("unsupported RPKG compression kind: {0}")]
    UnsupportedCompression(u8),

    #[error("unsupported RPKG purpose: {0}")]
    UnsupportedPurpose(u8),

    #[error("invalid RPKG reserved field: expected 0, found {0}")]
    InvalidReserved(u32),

    #[error("RPKG payload length does not match the available bytes")]
    InvalidPayloadLength,

    #[error("RPKG metadata length does not match the available bytes")]
    InvalidMetadataLength,

    #[error("RPKG payload checksum does not match metadata")]
    ChecksumMismatch,

    #[error("failed to decode RPKG metadata: {0}")]
    Metadata(#[from] rmp_serde::decode::Error),

    #[error("failed to decompress RPKG payload: {0}")]
    Compression(#[from] std::io::Error),
}

/// Errors that can occur while encoding an `RPKG` container.
#[derive(Debug, Error)]
pub enum RpkgEncodeError {
    #[error("failed to encode RPKG metadata: {0}")]
    Metadata(#[from] rmp_serde::encode::Error),

    #[error("failed to compress RPKG payload: {0}")]
    Compression(#[from] std::io::Error),

    #[error("failed to finalize RPKG compression frame: {0}")]
    CompressionFrame(#[from] lz4_flex::frame::Error),
}

pub fn encode(payload: &[u8], options: &RpkgEncodeOptions) -> Result<Vec<u8>, RpkgEncodeError> {
    let encoded_payload = match options.compression {
        CompressionKind::None => payload.to_vec(),
        CompressionKind::Lz4Frame => {
            let mut encoder = FrameEncoder::new(Vec::new());
            encoder.write_all(payload)?;
            encoder.finish()?
        }
    };

    let mut metadata = options.metadata.clone();
    let mut metadata_len = rmp_serde::to_vec(&metadata)?.len();
    let (header, metadata_bytes) = loop {
        let header = build_header(
            options.serialization,
            options.compression,
            options.purpose,
            options.package_id,
            encoded_payload.len(),
            metadata_len,
        );

        metadata.checksum_sha256 = compute_checksum(&header, &encoded_payload);
        let metadata_bytes = rmp_serde::to_vec(&metadata)?;
        let new_len = metadata_bytes.len();
        if new_len == metadata_len {
            break (header, metadata_bytes);
        }

        metadata_len = new_len;
    };

    let mut bytes = Vec::with_capacity(header.len() + encoded_payload.len() + metadata_bytes.len());
    bytes.extend_from_slice(&header);
    bytes.extend_from_slice(&encoded_payload);
    bytes.extend_from_slice(&metadata_bytes);
    Ok(bytes)
}

pub fn decode(
    bytes: &[u8],
    verification_mode: VerificationMode,
) -> Result<DecodedPackage, RpkgDecodeError> {
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

    let serialization = SerializationKind::try_from(bytes[5])?;
    let compression = CompressionKind::try_from(bytes[6])?;
    let purpose = PackagePurpose::try_from(bytes[7])?;
    let package_id = [bytes[8], bytes[9], bytes[10], bytes[11]];
    let payload_len = u64::from_le_bytes(bytes[12..20].try_into().unwrap());
    let metadata_len = u32::from_le_bytes(bytes[20..24].try_into().unwrap());
    let reserved = u32::from_le_bytes(bytes[24..28].try_into().unwrap());
    if reserved != 0 {
        return Err(RpkgDecodeError::InvalidReserved(reserved));
    }

    let payload_len = usize::try_from(payload_len).map_err(|_| RpkgDecodeError::InvalidPayloadLength)?;
    let metadata_len =
        usize::try_from(metadata_len).map_err(|_| RpkgDecodeError::InvalidMetadataLength)?;
    let payload_end = HEADER_LEN
        .checked_add(payload_len)
        .ok_or(RpkgDecodeError::InvalidPayloadLength)?;
    if payload_end > bytes.len() {
        return Err(RpkgDecodeError::InvalidPayloadLength);
    }
    let metadata_end = payload_end
        .checked_add(metadata_len)
        .ok_or(RpkgDecodeError::InvalidMetadataLength)?;
    if metadata_end != bytes.len() {
        return Err(RpkgDecodeError::InvalidMetadataLength);
    }

    let metadata: PackageMetadata = rmp_serde::from_slice(&bytes[payload_end..metadata_end])?;
    let payload_slice = &bytes[HEADER_LEN..payload_end];
    if verification_mode.should_verify(purpose) {
        let header = build_header(
            serialization,
            compression,
            purpose,
            package_id,
            payload_len,
            metadata_len,
        );
        let checksum = compute_checksum(&header, payload_slice);
        if checksum != metadata.checksum_sha256 {
            return Err(RpkgDecodeError::ChecksumMismatch);
        }
    }

    let payload = match compression {
        CompressionKind::None => payload_slice.to_vec(),
        CompressionKind::Lz4Frame => {
            let mut decoder = FrameDecoder::new(Cursor::new(payload_slice));
            let mut payload = Vec::new();
            decoder.read_to_end(&mut payload)?;
            payload
        }
    };

    Ok(DecodedPackage {
        package_id,
        serialization,
        compression,
        purpose,
        metadata,
        payload,
    })
}

fn build_header(
    serialization: SerializationKind,
    compression: CompressionKind,
    purpose: PackagePurpose,
    package_id: [u8; 4],
    payload_len: usize,
    metadata_len: usize,
) -> Vec<u8> {
    let mut header = Vec::with_capacity(HEADER_LEN);
    header.extend_from_slice(RPKG_MAGIC);
    header.push(RPKG_WIRE_VERSION);
    header.push(serialization as u8);
    header.push(compression as u8);
    header.push(purpose as u8);
    header.extend_from_slice(&package_id);
    header.extend_from_slice(&(payload_len as u64).to_le_bytes());
    header.extend_from_slice(&(metadata_len as u32).to_le_bytes());
    header.extend_from_slice(&0u32.to_le_bytes());
    header
}

fn compute_checksum(header: &[u8], payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(header);
    hasher.update(payload);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{
        CompressionKind, PackageMetadata, PackagePurpose, RpkgDecodeError, RpkgEncodeOptions,
        SerializationKind, VerificationMode, decode, encode,
    };

    fn sample_options() -> RpkgEncodeOptions {
        RpkgEncodeOptions {
            package_id: *b"TEST",
            serialization: SerializationKind::MessagePack,
            compression: CompressionKind::Lz4Frame,
            purpose: PackagePurpose::External,
            metadata: PackageMetadata::new("1.0.0", "source-1", 1),
        }
    }

    #[test]
    fn external_package_verifies_checksum_by_default() {
        let bytes = encode(b"hello", &sample_options()).expect("package should encode");
        let decoded = decode(&bytes, VerificationMode::Default).expect("package should decode");

        assert_eq!(decoded.package_id, *b"TEST");
        assert_eq!(decoded.payload, b"hello");
    }

    #[test]
    fn embedded_package_skips_checksum_by_default() {
        let mut options = sample_options();
        options.purpose = PackagePurpose::Embedded;
        let mut bytes = encode(b"hello", &options).expect("package should encode");
        let last = bytes.len() - 1;
        bytes[last] ^= 0x01;

        let decoded = decode(&bytes, VerificationMode::Default).expect("package should decode");
        assert_eq!(decoded.purpose, PackagePurpose::Embedded);
    }

    #[test]
    fn forced_verification_catches_embedded_checksum_mismatch() {
        let mut options = sample_options();
        options.purpose = PackagePurpose::Embedded;
        let mut bytes = encode(b"hello", &options).expect("package should encode");
        let index = 8;
        bytes[index] ^= 0x01;

        let error = decode(&bytes, VerificationMode::Always).expect_err("checksum should fail");
        assert!(matches!(error, RpkgDecodeError::ChecksumMismatch));
    }

    #[test]
    fn rejects_non_rpkg_bytes() {
        let error = decode(b"not-rpkg", VerificationMode::Default)
            .expect_err("non-RPKG bytes should fail");
        assert!(matches!(error, RpkgDecodeError::UnsupportedFormat));
    }
}
