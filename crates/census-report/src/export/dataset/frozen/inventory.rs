use super::budget::check_json_budget;
use super::codec::{
    check_codec_head, check_entity_id, checked_add, count, encoding_error, is_digest, sha256_hex,
    FrozenCodecError, FrozenCodecResult, CHUNK_BYTES_LIMIT, CODEC_REVISION, INVENTORY_BYTES_LIMIT,
    INVENTORY_CODEC_ID, MAX_CHUNKS_PER_STREAM, MAX_JSON_NODES,
};
use super::stream::StreamKind;
use census_domain::model::serialized_bytes;
use serde::{Deserialize, Serialize};

pub const NODES_PER_CHUNK_DESCRIPTOR: u64 = 12;
pub const MAX_INVENTORY_JSON_NODES: u64 =
    MAX_JSON_NODES + MAX_CHUNKS_PER_STREAM * NODES_PER_CHUNK_DESCRIPTOR;

struct ChunkTotals {
    rows: u64,
    bytes: u64,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactRef {
    pub digest: String,
    pub bytes: u64,
    pub codec_id: String,
    pub codec_revision: u32,
}

impl ArtifactRef {
    pub fn of(object: &[u8]) -> FrozenCodecResult<Self> {
        let reference = Self {
            digest: sha256_hex(object),
            bytes: count(object.len())?,
            codec_id: INVENTORY_CODEC_ID.to_string(),
            codec_revision: CODEC_REVISION,
        };
        reference.validate()?;
        Ok(reference)
    }

    pub fn validate(&self) -> FrozenCodecResult<()> {
        if self.codec_id != INVENTORY_CODEC_ID || self.codec_revision != CODEC_REVISION {
            return Err(FrozenCodecError::UnsupportedVersion {
                what: "artifact reference",
                found: format!("{} revision {}", self.codec_id, self.codec_revision),
                supported: format!("{INVENTORY_CODEC_ID} revision {CODEC_REVISION}"),
            });
        }
        if !is_digest(&self.digest) {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "artifact reference",
                detail: format!("{} is not a lowercase sha256 digest", self.digest),
            });
        }
        if self.bytes == 0 {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "artifact reference",
                detail: "an artifact reference must cover at least one byte".to_string(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChunkDescriptor {
    pub index: u64,
    pub first_ordinal: u64,
    pub rows: u64,
    pub bytes: u64,
    pub sha256: String,
    pub first_key: String,
    pub last_key: String,
    pub codec_revision: u32,
}

impl ChunkDescriptor {
    pub fn validate(&self, expected_index: u64) -> FrozenCodecResult<()> {
        if self.codec_revision != CODEC_REVISION {
            return Err(FrozenCodecError::UnsupportedVersion {
                what: "chunk descriptor",
                found: format!("revision {}", self.codec_revision),
                supported: format!("revision {CODEC_REVISION}"),
            });
        }
        if self.index != expected_index {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "chunk descriptor",
                detail: format!(
                    "chunk index {} appears where {expected_index} was expected",
                    self.index
                ),
            });
        }
        if !is_digest(&self.sha256) {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "chunk descriptor",
                detail: format!("{} is not a lowercase sha256 digest", self.sha256),
            });
        }
        if self.rows == 0 {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "chunk descriptor",
                detail: "a chunk must hold at least one row".to_string(),
            });
        }
        if self.bytes == 0 || self.bytes > CHUNK_BYTES_LIMIT {
            return Err(FrozenCodecError::RowTooLarge {
                what: "frozen chunk",
                bytes: self.bytes,
                max: CHUNK_BYTES_LIMIT,
            });
        }
        check_entity_id(&self.first_key)?;
        check_entity_id(&self.last_key)?;
        if self.last_key < self.first_key {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "chunk descriptor",
                detail: format!(
                    "chunk keys run from {} back to {}",
                    self.first_key, self.last_key
                ),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StreamInventory {
    pub stream: String,
    pub codec: String,
    pub codec_revision: u32,
    pub chunks: Vec<ChunkDescriptor>,
    pub rows: u64,
    pub bytes: u64,
}

impl StreamInventory {
    pub fn new(
        stream: StreamKind,
        chunks: Vec<ChunkDescriptor>,
        rows: u64,
        bytes: u64,
    ) -> FrozenCodecResult<Self> {
        let inventory = Self {
            stream: stream.wire().to_string(),
            codec: INVENTORY_CODEC_ID.to_string(),
            codec_revision: CODEC_REVISION,
            chunks,
            rows,
            bytes,
        };
        inventory.validate()?;
        Ok(inventory)
    }

    pub fn kind(&self) -> FrozenCodecResult<StreamKind> {
        StreamKind::from_wire(&self.stream).ok_or_else(|| FrozenCodecError::CorruptArtifact {
            what: "stream inventory",
            detail: format!("{} is not a supported logical stream", self.stream),
        })
    }

    pub fn rows(&self) -> u64 {
        self.rows
    }

    pub fn bytes(&self) -> u64 {
        self.bytes
    }

    pub fn chunks(&self) -> &[ChunkDescriptor] {
        &self.chunks
    }

    pub fn validate(&self) -> FrozenCodecResult<()> {
        self.kind()?;
        if self.codec != INVENTORY_CODEC_ID || self.codec_revision != CODEC_REVISION {
            return Err(FrozenCodecError::UnsupportedVersion {
                what: "stream inventory",
                found: format!("{} revision {}", self.codec, self.codec_revision),
                supported: format!("{INVENTORY_CODEC_ID} revision {CODEC_REVISION}"),
            });
        }
        let chunk_count = count(self.chunks.len())?;
        if chunk_count > MAX_CHUNKS_PER_STREAM {
            return Err(FrozenCodecError::NeedsPartition {
                what: "stream chunk count",
                bytes: chunk_count,
                max: MAX_CHUNKS_PER_STREAM,
            });
        }
        let totals = self.validate_descriptors()?;
        if totals.rows != self.rows {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "stream inventory",
                detail: format!(
                    "descriptors account for {} rows against the declared {}",
                    totals.rows, self.rows
                ),
            });
        }
        if totals.bytes != self.bytes {
            return Err(FrozenCodecError::CorruptArtifact {
                what: "stream inventory",
                detail: format!(
                    "descriptors account for {} bytes against the declared {}",
                    totals.bytes, self.bytes
                ),
            });
        }
        Ok(())
    }

    fn validate_descriptors(&self) -> FrozenCodecResult<ChunkTotals> {
        let mut expected_index: u64 = 0;
        let mut expected_ordinal: u64 = 0;
        let mut rows: u64 = 0;
        let mut bytes: u64 = 0;
        for descriptor in &self.chunks {
            descriptor.validate(expected_index)?;
            if descriptor.first_ordinal != expected_ordinal {
                return Err(FrozenCodecError::CorruptArtifact {
                    what: "stream inventory",
                    detail: format!(
                        "chunk {expected_index} starts at ordinal {} instead of {expected_ordinal}",
                        descriptor.first_ordinal
                    ),
                });
            }
            expected_ordinal = checked_add(
                "stream inventory ordinals",
                expected_ordinal,
                descriptor.rows,
            )?;
            rows = checked_add("stream inventory rows", rows, descriptor.rows)?;
            bytes = checked_add("stream inventory bytes", bytes, descriptor.bytes)?;
            expected_index = checked_add("stream inventory index", expected_index, 1)?;
        }
        Ok(ChunkTotals { rows, bytes })
    }

    pub fn encode(&self) -> FrozenCodecResult<Vec<u8>> {
        self.validate()?;
        let encoded = serialized_bytes(self).map_err(encoding_error)?;
        let size = count(encoded.len())?;
        if size > INVENTORY_BYTES_LIMIT {
            return Err(FrozenCodecError::NeedsPartition {
                what: "stream inventory",
                bytes: size,
                max: INVENTORY_BYTES_LIMIT,
            });
        }
        Ok(encoded)
    }

    pub fn decode(bytes: &[u8]) -> FrozenCodecResult<Self> {
        let size = count(bytes.len())?;
        if size > INVENTORY_BYTES_LIMIT {
            return Err(FrozenCodecError::RowTooLarge {
                what: "stream inventory",
                bytes: size,
                max: INVENTORY_BYTES_LIMIT,
            });
        }
        check_codec_head(bytes, INVENTORY_CODEC_ID, "stream inventory")?;
        check_json_budget("stream inventory", bytes, MAX_INVENTORY_JSON_NODES)?;
        let inventory: Self =
            serde_json::from_slice(bytes).map_err(|source| FrozenCodecError::CorruptArtifact {
                what: "stream inventory",
                detail: source.to_string(),
            })?;
        inventory.validate()?;
        Ok(inventory)
    }
}
