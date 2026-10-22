use super::*;
use base64::{engine::general_purpose::STANDARD, Engine};
use census_store::RosterFrozenReviewedClaimsReferenceV1;

fn refuse_inner(
    bundle: &FrozenRosterReviewedClaimsBundleV1,
    bytes: &[u8],
    reason: &str,
) -> TestResult {
    let mut reference = serde_json::to_value(bundle.reference())?;
    *reference
        .get_mut("digest")
        .ok_or("reference digest absent")? = serde_json::json!(hash(bytes));
    *reference.get_mut("bytes").ok_or("reference bytes absent")? = serde_json::json!(bytes.len());
    let reference: RosterFrozenReviewedClaimsReferenceV1 = serde_json::from_value(reference)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let error = replay_frozen_roster_reviewed_claims_v1(bytes, &reference, &mut grant)
        .err()
        .ok_or("tampered inner closure was accepted")?;
    check!(
        matches!(error, census_store::StoreError::RosterSourceIssuance(
        census_store::RosterSourceIssuanceStoreError::Corrupt(actual)) if actual == reason)
    );
    check!(grant.ensure_available().is_err());
    Ok(())
}

pub(super) fn refuse_changed_or_missing_last_object(
    bundle: &FrozenRosterReviewedClaimsBundleV1,
    expected: &Witness,
) -> TestResult {
    let raw = std::str::from_utf8(bundle.bytes())?;
    let key = format!(
        "roster_source_issuance/v1/objects/{}",
        hash(&expected.last_raw)
    );
    let pair = serde_json::to_string(&(
        STANDARD.encode(key.as_bytes()),
        STANDARD.encode(&expected.last_raw),
    ))?;
    check!(eq; raw.matches(&pair).count(), 1);
    let mut changed_raw = expected.last_raw.clone();
    let first = changed_raw.first_mut().ok_or("empty physical row")?;
    *first ^= 1;
    let changed = serde_json::to_string(&(
        STANDARD.encode(key.as_bytes()),
        STANDARD.encode(changed_raw),
    ))?;
    refuse_inner(
        bundle,
        raw.replacen(&pair, &changed, 1).as_bytes(),
        "review object digest",
    )?;
    let adjacent = format!("{pair},");
    let missing = if raw.contains(&adjacent) {
        raw.replacen(&adjacent, "", 1)
    } else {
        let adjacent = format!(",{pair}");
        check!(raw.contains(&adjacent));
        raw.replacen(&adjacent, "", 1)
    };
    refuse_inner(bundle, missing.as_bytes(), "review closure object absent")
}
