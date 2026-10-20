use super::super::DatasetLineage;
use super::budget::check_json_budget;
use super::codec::{
    check_codec_head, check_text, corrupt, count, encoding_error, is_digest, FrozenCodecError,
    FrozenCodecResult, CODEC_REVISION, MANIFEST_BYTES_LIMIT, MANIFEST_CODEC_ID,
    MAX_INVENTORY_ROOTS, MAX_JSON_NODES, MAX_ROOT_TEXT_BYTES,
};
use super::references::{ManifestFingerprint, ManifestStream};
use super::stream::StreamKind;
use super::token::FrozenInputToken;
use super::{INPUT_TABLES, SCHEMA_REVISION};
use census_domain::model::{serialized_bytes, SchoolYear};
use census_store::Table;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenManifest {
    pub codec: String,
    pub codec_revision: u32,
    pub lineage: DatasetLineage,
    pub input_token: FrozenInputToken,
    pub streams: Vec<ManifestStream>,
    pub source_fingerprints: Vec<ManifestFingerprint>,
    #[serde(default)]
    pub legacy_logical_digest: Option<String>,
}

impl FrozenManifest {
    pub fn new(
        lineage: DatasetLineage,
        input_token: FrozenInputToken,
        streams: Vec<ManifestStream>,
        source_fingerprints: Vec<ManifestFingerprint>,
        legacy_logical_digest: Option<String>,
    ) -> FrozenCodecResult<Self> {
        let manifest = Self {
            codec: MANIFEST_CODEC_ID.to_string(),
            codec_revision: CODEC_REVISION,
            lineage,
            input_token,
            streams,
            source_fingerprints,
            legacy_logical_digest,
        };
        manifest.validate()?;
        Ok(manifest)
    }

    pub fn validate(&self) -> FrozenCodecResult<()> {
        if self.codec != MANIFEST_CODEC_ID || self.codec_revision != CODEC_REVISION {
            return Err(FrozenCodecError::UnsupportedVersion {
                what: "frozen manifest",
                found: format!("{} revision {}", self.codec, self.codec_revision),
                supported: format!("{MANIFEST_CODEC_ID} revision {CODEC_REVISION}"),
            });
        }
        check_lineage(&self.lineage)?;
        self.input_token.validate_against(&self.lineage)?;
        let roots = self
            .streams
            .len()
            .checked_add(self.source_fingerprints.len())
            .ok_or_else(|| {
                corrupt(
                    "frozen manifest",
                    "the inventory root count overflowed usize",
                )
            })?;
        if roots > MAX_INVENTORY_ROOTS {
            return Err(FrozenCodecError::NeedsPartition {
                what: "manifest inventory roots",
                bytes: count(roots)?,
                max: count(MAX_INVENTORY_ROOTS)?,
            });
        }
        self.validate_streams()?;
        self.validate_fingerprints()?;
        self.validate_legacy_digest()?;
        Ok(())
    }

    fn validate_streams(&self) -> FrozenCodecResult<()> {
        if self.streams.len() != StreamKind::ALL.len() {
            return Err(corrupt(
                "frozen manifest",
                &format!(
                    "{} streams against the required {}",
                    self.streams.len(),
                    StreamKind::ALL.len()
                ),
            ));
        }
        for (index, stream) in self.streams.iter().enumerate() {
            let expected = StreamKind::ALL
                .get(index)
                .copied()
                .ok_or_else(|| corrupt("frozen manifest", "a stream index escaped the codec"))?;
            if stream.stream != expected.wire() {
                return Err(corrupt(
                    "frozen manifest",
                    &format!(
                        "stream {index} is {} against the required {}",
                        stream.stream,
                        expected.wire()
                    ),
                ));
            }
            stream.validate()?;
        }
        Ok(())
    }

    fn validate_fingerprints(&self) -> FrozenCodecResult<()> {
        if self.source_fingerprints.len() != INPUT_TABLES.len() {
            return Err(corrupt(
                "frozen manifest",
                &format!(
                    "{} fingerprints against the required {} input tables",
                    self.source_fingerprints.len(),
                    INPUT_TABLES.len()
                ),
            ));
        }
        for (index, fingerprint) in self.source_fingerprints.iter().enumerate() {
            let expected = INPUT_TABLES.get(index).copied().ok_or_else(|| {
                corrupt("frozen manifest", "a fingerprint index escaped the codec")
            })?;
            fingerprint.validate()?;
            if fingerprint.table != expected.file() {
                return Err(corrupt(
                    "frozen manifest",
                    &format!(
                        "fingerprint {index} covers {} against the required {}",
                        fingerprint.table,
                        expected.file()
                    ),
                ));
            }
        }
        Ok(())
    }

    fn validate_legacy_digest(&self) -> FrozenCodecResult<()> {
        if let Some(digest) = self.legacy_logical_digest.as_deref() {
            if !is_digest(digest) {
                return Err(corrupt(
                    "frozen manifest",
                    &format!("legacy logical digest {digest} is not a lowercase sha256 digest"),
                ));
            }
        }
        Ok(())
    }

    pub fn stream(&self, kind: StreamKind) -> Option<&ManifestStream> {
        self.streams.get(kind.index())
    }

    pub fn fingerprint(&self, table: Table) -> Option<&ManifestFingerprint> {
        self.source_fingerprints
            .iter()
            .find(|fingerprint| fingerprint.table == table.file())
    }

    pub fn encode(&self) -> FrozenCodecResult<Vec<u8>> {
        self.validate()?;
        let encoded = serialized_bytes(self).map_err(encoding_error)?;
        let size = count(encoded.len())?;
        if size > MANIFEST_BYTES_LIMIT {
            return Err(FrozenCodecError::RowTooLarge {
                what: "frozen manifest",
                bytes: size,
                max: MANIFEST_BYTES_LIMIT,
            });
        }
        Ok(encoded)
    }

    pub fn decode(bytes: &[u8]) -> FrozenCodecResult<Self> {
        let size = count(bytes.len())?;
        if size > MANIFEST_BYTES_LIMIT {
            return Err(FrozenCodecError::RowTooLarge {
                what: "frozen manifest",
                bytes: size,
                max: MANIFEST_BYTES_LIMIT,
            });
        }
        check_codec_head(bytes, MANIFEST_CODEC_ID, "frozen manifest")?;
        check_json_budget("frozen manifest", bytes, MAX_JSON_NODES)?;
        let manifest: Self = serde_json::from_slice(bytes).map_err(|source| {
            corrupt(
                "frozen manifest",
                &format!("{source} at byte {}", source.column()),
            )
        })?;
        manifest.validate()?;
        Ok(manifest)
    }
}

fn check_lineage(lineage: &DatasetLineage) -> FrozenCodecResult<()> {
    let supported = format!("schema revision {SCHEMA_REVISION} policy revisions 19, 20, 21 or 22");
    if lineage.schema_revision != SCHEMA_REVISION
        || census_domain::model::EventSemantics::from_revision(lineage.policy_revision).is_none()
    {
        return Err(FrozenCodecError::UnsupportedVersion {
            what: "manifest lineage",
            found: format!(
                "schema revision {} policy revision {}",
                lineage.schema_revision, lineage.policy_revision
            ),
            supported,
        });
    }
    for (what, value) in [
        ("store identity", &lineage.store_identity),
        ("source digest", &lineage.source_digest),
        ("input digest", &lineage.input_digest),
        ("input generation", &lineage.input_generation),
    ] {
        if !is_digest(value) {
            return Err(corrupt(
                "manifest lineage",
                &format!("{what} {value} is not a lowercase sha256 digest"),
            ));
        }
    }
    check_text(
        "manifest store root",
        &lineage.store_root,
        MAX_ROOT_TEXT_BYTES,
    )?;
    if let Some(job) = lineage.export_job.as_deref() {
        check_text("manifest export job", job, MAX_ROOT_TEXT_BYTES)?;
    }
    if SchoolYear::from_date(&lineage.generated_on).is_none() {
        return Err(corrupt(
            "manifest lineage",
            &format!(
                "generated_on {:?} is not an ISO date or year",
                lineage.generated_on
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
#[path = "manifest_fixtures.rs"]
mod manifest_fixtures;

#[cfg(test)]
#[path = "manifest_tests.rs"]
mod manifest_tests;
