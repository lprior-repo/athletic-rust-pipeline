use census_domain::model::CanonicalJsonError;
use serde::Deserialize;
use sha2::{Digest, Sha256};

pub const MANIFEST_CODEC_ID: &str = "census-frozen-jsonl-v3";
pub const INVENTORY_CODEC_ID: &str = "census-frozen-inventory-v3";
pub const CODEC_REVISION: u32 = 3;
pub const TOKEN_PROTOCOL: u32 = 3;
pub const MANIFEST_BYTES_LIMIT: u64 = 64 * 1024;
pub const INVENTORY_BYTES_LIMIT: u64 = 4 * 1024 * 1024;
pub const CHUNK_BYTES_LIMIT: u64 = 4 * 1024 * 1024;
pub const LINE_BYTES_LIMIT: u64 = 1024 * 1024;
pub const MAX_CHUNKS_PER_STREAM: u64 = 16_384;
pub const MAX_INVENTORY_ROOTS: usize = 64;
pub const MAX_JSON_DEPTH: u64 = 64;
pub const MAX_JSON_NODES: u64 = 65_536;
pub const MAX_JSON_STRING_BYTES: u64 = 256 * 1024;
pub const MAX_ENTITY_ID_BYTES: usize = census_store::MAX_ID_BYTES;
pub const MAX_TOKEN_TEXT_BYTES: usize = 128;
pub const MAX_ROOT_TEXT_BYTES: usize = 4096;

#[derive(Debug, thiserror::Error)]
pub enum FrozenCodecError {
    #[error("{what} of {bytes} bytes exceeds the {max}-byte ceiling")]
    RowTooLarge {
        what: &'static str,
        bytes: u64,
        max: u64,
    },
    #[error("{what} needs {needed} against an available {available}")]
    BudgetExceeded {
        what: &'static str,
        needed: u64,
        available: u64,
    },
    #[error("{what} of {bytes} bytes needs another partition beyond the {max}-byte envelope")]
    NeedsPartition {
        what: &'static str,
        bytes: u64,
        max: u64,
    },
    #[error("unsupported {what}: {found}; this codec writes {supported}")]
    UnsupportedVersion {
        what: &'static str,
        found: String,
        supported: String,
    },
    #[error("corrupt {what}: {detail}")]
    CorruptArtifact { what: &'static str, detail: String },
    #[error("serializing a frozen record: {detail}")]
    Encoding { detail: String },
}

pub type FrozenCodecResult<T> = Result<T, FrozenCodecError>;

pub fn corrupt(what: &'static str, detail: &str) -> FrozenCodecError {
    FrozenCodecError::CorruptArtifact {
        what,
        detail: detail.to_string(),
    }
}

pub fn count(value: usize) -> FrozenCodecResult<u64> {
    u64::try_from(value)
        .map_err(|_| corrupt("frozen codec counter", "a byte or row count exceeded u64"))
}

pub fn checked_add(what: &'static str, left: u64, right: u64) -> FrozenCodecResult<u64> {
    left.checked_add(right)
        .ok_or_else(|| corrupt(what, "a running total overflowed u64"))
}

pub fn encoding_error(error: CanonicalJsonError) -> FrozenCodecError {
    FrozenCodecError::Encoding {
        detail: error.to_string(),
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn is_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

pub fn check_entity_id(entity_id: &str) -> FrozenCodecResult<()> {
    if entity_id.is_empty() {
        return Err(corrupt("entity id", "an entity id must not be empty"));
    }
    if entity_id.len() > MAX_ENTITY_ID_BYTES {
        return Err(FrozenCodecError::RowTooLarge {
            what: "entity id",
            bytes: count(entity_id.len())?,
            max: count(MAX_ENTITY_ID_BYTES)?,
        });
    }
    Ok(())
}

pub fn check_text(what: &'static str, value: &str, max: usize) -> FrozenCodecResult<()> {
    if value.len() > max {
        return Err(FrozenCodecError::RowTooLarge {
            what,
            bytes: count(value.len())?,
            max: count(max)?,
        });
    }
    Ok(())
}

pub fn check_codec_head(
    bytes: &[u8],
    supported: &str,
    what: &'static str,
) -> FrozenCodecResult<()> {
    let head: CodecHead =
        serde_json::from_slice(bytes).map_err(|source| corrupt(what, &source.to_string()))?;
    if head.codec == supported && head.codec_revision == CODEC_REVISION {
        return Ok(());
    }
    Err(FrozenCodecError::UnsupportedVersion {
        what,
        found: format!("{} revision {}", head.codec, head.codec_revision),
        supported: format!("{supported} revision {CODEC_REVISION}"),
    })
}

#[derive(Deserialize)]
struct CodecHead {
    codec: String,
    codec_revision: u32,
}

#[cfg(test)]
#[path = "codec_tests.rs"]
mod codec_tests;

#[cfg(test)]
#[path = "codec_adversarial_tests.rs"]
mod codec_adversarial_tests;
