use super::super::DatasetLineage;
use super::codec::{
    check_text, corrupt, encoding_error, is_digest, FrozenCodecError, FrozenCodecResult,
    MAX_TOKEN_TEXT_BYTES, TOKEN_PROTOCOL,
};
use crate::report::{ReportError, ReportResult};
use census_domain::model::{serialized_digest, RunManifest};
use census_store::StoreSnapshot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FrozenInputToken {
    pub protocol: u32,
    pub store_identity: String,
    pub run_manifest_digest: Option<String>,
    pub store_schema: u32,
    pub key_format: u32,
    pub logical_schema: u32,
    pub semantic_policy: u32,
    pub input_root: String,
    pub snapshot_sequence: u64,
    pub as_of: String,
}

impl FrozenInputToken {
    pub fn capture(snapshot: &StoreSnapshot<'_>, lineage: &DatasetLineage) -> ReportResult<Self> {
        let manifest = snapshot.run_manifest()?;
        ensure_capture(snapshot, lineage, manifest.as_ref())?;
        let run_manifest_digest = manifest
            .as_ref()
            .map(serialized_digest)
            .transpose()
            .map_err(|error| ReportError::Invariant {
                detail: format!("digesting the bound run manifest: {error}"),
            })?;
        let token = Self {
            protocol: TOKEN_PROTOCOL,
            store_identity: lineage.store_identity.clone(),
            run_manifest_digest,
            store_schema: census_store::STORE_SCHEMA_VERSION,
            key_format: census_store::KEY_FORMAT_VERSION,
            logical_schema: lineage.schema_revision,
            semantic_policy: lineage.policy_revision,
            input_root: lineage.source_digest.clone(),
            snapshot_sequence: lineage.snapshot_sequence,
            as_of: lineage.generated_on.clone(),
        };
        token
            .validate_against(lineage)
            .map_err(|error| ReportError::Invariant {
                detail: error.to_string(),
            })?;
        Ok(token)
    }

    pub fn validate(&self) -> FrozenCodecResult<()> {
        if self.protocol != TOKEN_PROTOCOL {
            return Err(FrozenCodecError::UnsupportedVersion {
                what: "input token",
                found: format!("protocol {}", self.protocol),
                supported: format!("protocol {TOKEN_PROTOCOL}"),
            });
        }
        require_digest("input token store identity", &self.store_identity)?;
        require_digest("input token input root", &self.input_root)?;
        if let Some(digest) = self.run_manifest_digest.as_deref() {
            require_digest("input token run manifest digest", digest)?;
        }
        check_text(
            "input token store identity",
            &self.store_identity,
            MAX_TOKEN_TEXT_BYTES,
        )?;
        check_text("input token as of", &self.as_of, MAX_TOKEN_TEXT_BYTES)?;
        Ok(())
    }

    pub fn validate_against(&self, lineage: &DatasetLineage) -> FrozenCodecResult<()> {
        self.validate()?;
        let consistent = self.store_identity == lineage.store_identity
            && self.logical_schema == lineage.schema_revision
            && self.semantic_policy == lineage.policy_revision
            && self.input_root == lineage.source_digest
            && self.snapshot_sequence == lineage.snapshot_sequence
            && self.as_of == lineage.generated_on;
        if !consistent {
            return Err(corrupt(
                "input token",
                "the captured token does not describe the manifest lineage",
            ));
        }
        Ok(())
    }

    pub fn digest(&self) -> FrozenCodecResult<String> {
        serialized_digest(self).map_err(encoding_error)
    }
}

fn ensure_capture(
    snapshot: &StoreSnapshot<'_>,
    lineage: &DatasetLineage,
    manifest: Option<&RunManifest>,
) -> ReportResult<()> {
    let identity = snapshot.store_identity()?;
    if identity.as_str() != lineage.store_identity
        || snapshot.sequence() != lineage.snapshot_sequence
        || manifest.map(|bound| bound.run) != lineage.run
        || manifest.map(|bound| bound.cohort) != lineage.cohort
        || manifest.is_some_and(|bound| bound.store_identity != identity.as_str())
    {
        return Err(ReportError::Invariant {
            detail: "input token capture view differs from its lineage or run binding".to_owned(),
        });
    }
    Ok(())
}

fn require_digest(what: &'static str, value: &str) -> FrozenCodecResult<()> {
    if is_digest(value) {
        return Ok(());
    }
    Err(corrupt(
        what,
        &format!("{value} is not a lowercase sha256 digest"),
    ))
}
