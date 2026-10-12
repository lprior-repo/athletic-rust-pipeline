use super::codec::{corrupt, is_digest, FrozenCodecError, FrozenCodecResult};
use super::inventory::{ArtifactRef, StreamInventory};
use super::stream::StreamKind;
use census_store::Table;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestStream {
    pub stream: String,
    pub inventory: ArtifactRef,
    pub rows: u64,
    pub bytes: u64,
}

impl ManifestStream {
    pub fn of(inventory: &StreamInventory, encoded: &[u8]) -> FrozenCodecResult<Self> {
        let stream = Self {
            stream: inventory.kind()?.wire().to_string(),
            inventory: ArtifactRef::of(encoded)?,
            rows: inventory.rows(),
            bytes: inventory.bytes(),
        };
        stream.validate()?;
        Ok(stream)
    }

    pub fn validate(&self) -> FrozenCodecResult<()> {
        if StreamKind::from_wire(&self.stream).is_none() {
            return Err(corrupt(
                "manifest stream",
                &format!("{} is not one of the twelve logical streams", self.stream),
            ));
        }
        self.inventory.validate()?;
        if self.rows > census_store::MAX_ROWS_PER_TABLE {
            return Err(FrozenCodecError::RowTooLarge {
                what: "manifest stream rows",
                bytes: self.rows,
                max: census_store::MAX_ROWS_PER_TABLE,
            });
        }
        if (self.rows == 0) != (self.bytes == 0) {
            return Err(corrupt(
                "manifest stream",
                &format!(
                    "{} declares {} rows against {} bytes",
                    self.stream, self.rows, self.bytes
                ),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManifestFingerprint {
    pub table: String,
    pub digest: String,
}

impl ManifestFingerprint {
    pub fn of(table: Table, digest: &str) -> FrozenCodecResult<Self> {
        let fingerprint = Self {
            table: table.file().to_string(),
            digest: digest.to_string(),
        };
        fingerprint.validate()?;
        Ok(fingerprint)
    }

    pub fn validate(&self) -> FrozenCodecResult<()> {
        if Table::from_wire(&self.table).is_none() {
            return Err(corrupt(
                "source fingerprint",
                &format!("{} is not a store table", self.table),
            ));
        }
        if !is_digest(&self.digest) {
            return Err(corrupt(
                "source fingerprint",
                &format!("{} is not a lowercase sha256 digest", self.digest),
            ));
        }
        Ok(())
    }
}
