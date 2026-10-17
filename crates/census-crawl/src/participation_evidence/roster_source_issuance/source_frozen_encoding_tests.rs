use super::*;
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use census_domain::evidence_subject_attachment::EvidenceSubjectAttachmentError;
use census_store::{FrozenRosterReviewedClaimsBundleV1, RosterFrozenReviewedClaimsReferenceV1};

fn frozen_source() -> TestResult<FrozenRosterReviewedClaimsBundleV1> {
    let (directory, store) = issued()?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_roster_reviewed_claims_v1(&mut grant)?;
    let frozen = current.freeze_closure(&mut grant)?;
    drop(current);
    drop(snapshot);
    drop(store);
    directory.close()?;
    Ok(frozen)
}

fn rehash(
    bytes: &[u8],
    reference: &RosterFrozenReviewedClaimsReferenceV1,
) -> TestResult<RosterFrozenReviewedClaimsReferenceV1> {
    let mut value = serde_json::to_value(reference)?;
    *value.get_mut("digest").ok_or("reference digest")? = json!(hash(bytes));
    *value.get_mut("bytes").ok_or("reference bytes")? = json!(u64::try_from(bytes.len())?);
    Ok(serde_json::from_value(value)?)
}

fn refuses_copy_and_replays_original(
    frozen: &FrozenRosterReviewedClaimsBundleV1,
    altered: &[u8],
    expected_reason: &'static str,
) -> TestResult {
    let reference = rehash(altered, frozen.reference())?;
    let mut denied = AttachmentReplayGrantV1::new();
    let error = replay_frozen_roster_reviewed_claims_v1(altered, &reference, &mut denied)
        .err()
        .ok_or("unsupported or malformed wire was accepted")?;
    check!(
        matches!(error, census_store::StoreError::RosterSourceIssuance(
        census_store::RosterSourceIssuanceStoreError::Corrupt(reason)) if reason == expected_reason)
    );
    check!(matches!(
        denied.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    let mut grant = AttachmentReplayGrantV1::new();
    let replay =
        replay_frozen_roster_reviewed_claims_v1(frozen.bytes(), frozen.reference(), &mut grant)?;
    reassess(
        replay
            .claims(&mut grant)?
            .first()
            .ok_or("replayed review absent")?,
        &mut grant,
    )
}

#[test]
fn old_v1_schema_version_is_explicitly_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    *archive.get_mut("schema").ok_or("archive schema")? =
        json!("roster-reviewed-source-publication-closure-v1");
    let records = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .ok_or("records")?;
    for row in records {
        for cell in row.as_array_mut().ok_or("record row")? {
            let bytes = STANDARD.decode(cell.as_str().ok_or("compact record bytes")?)?;
            *cell = serde_json::to_value(bytes)?;
        }
    }
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "review publication closure version",
    )
}

#[test]
fn invalid_base64_record_value_is_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    let row = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_array_mut)
        .ok_or("first record row")?;
    *row.get_mut(1).ok_or("record value")? = json!("@@@@");
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "frozen reviewed closure decoding",
    )
}

#[test]
fn missing_value_record_row_is_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    let row = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_array_mut)
        .ok_or("first record row")?;
    row.truncate(1);
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "frozen reviewed closure decoding",
    )
}

#[test]
fn extra_element_record_row_is_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    let row = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_array_mut)
        .ok_or("first record row")?;
    row.push(json!("spare"));
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "frozen reviewed closure decoding",
    )
}

#[test]
fn numeric_byte_array_record_value_is_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    let row = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_array_mut)
        .ok_or("first record row")?;
    *row.get_mut(1).ok_or("record value")? = json!([97, 98, 99]);
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "frozen reviewed closure decoding",
    )
}

#[test]
fn unpadded_base64_record_value_is_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    let row = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_array_mut)
        .ok_or("first record row")?;
    *row.get_mut(1).ok_or("record value")? = json!("Zg");
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "frozen reviewed closure decoding",
    )
}

#[test]
fn nonzero_trailing_bits_base64_record_value_is_rejected() -> TestResult {
    let frozen = frozen_source()?;
    let mut archive: Value = serde_json::from_slice(frozen.bytes())?;
    let row = archive
        .get_mut("records")
        .and_then(Value::as_array_mut)
        .and_then(|rows| rows.first_mut())
        .and_then(Value::as_array_mut)
        .ok_or("first record row")?;
    *row.get_mut(1).ok_or("record value")? = json!("Zh==");
    refuses_copy_and_replays_original(
        &frozen,
        &serde_json::to_vec(&archive)?,
        "frozen reviewed closure decoding",
    )
}
