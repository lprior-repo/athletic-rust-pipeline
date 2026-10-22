use super::super::{codec, qualification, retained::Revision, PublicIdentityError};
use crate::{AcceptedPublicIdentitySnapshot, StoreResult};
use census_domain::{
    model::StoreIdentity,
    public_identity_qualification::{PublicIdentityQualification, PublicIdentityQualificationV2},
};
use serde::Deserialize;

const MAX_HISTORICAL_FROZEN_BYTES: usize = 128 * 1024 * 1024;

pub fn replay_frozen_public_identity_v1(
    bytes: &[u8],
    expected_digest: &str,
) -> StoreResult<AcceptedPublicIdentitySnapshot> {
    super::bound(bytes.len(), MAX_HISTORICAL_FROZEN_BYTES)?;
    if !crate::journal::is_object_id(expected_digest) || super::digest(bytes) != expected_digest {
        return Err(PublicIdentityError::Corrupt("frozen graph bundle digest").into());
    }
    let probe: serde_json::Value = codec::decode("frozen graph bundle", bytes)?;
    if probe.get("version").and_then(serde_json::Value::as_u64) != Some(1) {
        return Err(PublicIdentityError::Corrupt("frozen graph bundle version").into());
    }
    let bundle: Bundle = codec::decode("frozen graph bundle", bytes)?;
    bundle.replay()
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Bundle {
    version: u32,
    store_identity: StoreIdentity,
    revision: Revision,
    declaration: String,
    head: String,
    history: String,
    operations: Vec<serde_json::Value>,
    #[serde(rename = "contexts")]
    _contexts: Option<serde_json::Value>,
}

impl Bundle {
    fn replay(self) -> StoreResult<AcceptedPublicIdentitySnapshot> {
        if self.version != 1 {
            return Err(PublicIdentityError::Corrupt("frozen graph bundle version").into());
        }
        let declaration =
            PublicIdentityQualification::V2(codec::decode::<PublicIdentityQualificationV2>(
                "frozen graph declaration",
                self.declaration.as_bytes(),
            )?);
        if declaration.store_identity() != &self.store_identity
            || self.revision.snapshot_sequence == 0
        {
            return Err(PublicIdentityError::Corrupt("frozen graph lineage binding").into());
        }
        let head = codec::head(self.head.as_bytes())?;
        let history = codec::history(self.history.as_bytes(), &self.store_identity, &head)?;
        if self.operations.len() != history.records().len()
            || self.revision.evidence_generation < head.sequence
        {
            return Err(PublicIdentityError::Corrupt("frozen graph operation inventory").into());
        }
        Ok(AcceptedPublicIdentitySnapshot::from_qualified_ledger(
            qualification::Ledger {
                declaration,
                history,
            },
            self.revision.snapshot_sequence,
        ))
    }
}
