use super::*;
use census_domain::roster_affiliation_authority::{
    RosterAuthorityObjectRefV1, MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES,
};
use census_store::RosterSourceEvidenceV1;
use sha2::{Digest, Sha256};

pub(super) fn original(
    evidence: &RosterSourceEvidenceV1,
    expected: &RosterAuthorityObjectRefV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let bytes = evidence.physical_projection_manifest(grant)?;
    check!(eq; bytes.len(), 68_462);
    check!(bytes.len() > 64 * 1024);
    check!(bytes.len() <= MAX_ROSTER_PHYSICAL_PROJECTION_MANIFEST_BYTES);
    check!(eq; expected.bytes().get(), u64::try_from(bytes.len())?);
    check!(eq; expected.digest().as_str(), format!("{:x}", Sha256::digest(&bytes)));
    let value: serde_json::Value = serde_json::from_slice(&bytes)?;
    check!(eq; value.get("store"), Some(&serde_json::to_value(evidence.store_identity())?));
    check!(eq; value.get("snapshot_sequence"), Some(&serde_json::json!(evidence.snapshot_sequence())));
    let rows = value
        .get("rows")
        .and_then(serde_json::Value::as_array)
        .ok_or("manifest physical rows")?;
    check!(eq; rows.len(), evidence.physical_projection(grant)?.len());
    Ok(())
}

pub(super) fn current(store: &Store) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let sources = claims.claims(&mut grant)?;
    check!(eq; sources.len(), 1);
    let source = sources.first().ok_or("genuine reviewed roster claim")?;
    let expected = source
        .proposal(&mut grant)?
        .binding()
        .physical_projection_inventory();
    original(source.evidence(&mut grant)?, expected, &mut grant)
}
