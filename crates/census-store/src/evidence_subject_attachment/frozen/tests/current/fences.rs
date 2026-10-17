use super::*;
use crate::evidence_subject_attachment::frozen::{archive, FrozenRetainedCaptureSourceV1};
use crate::evidence_subject_attachment::RetainedCaptureRootSchemaV1;
use serde_json::value::RawValue;

#[test]
fn frozen_replay_refuses_an_authentically_issued_foreign_store_accepted_owner() -> TestResult {
    let (fixture, _, _) = committed("frozen-current-foreign")?;
    let (bundle, accepted) = capture(&fixture.store)?;
    let other = Fixture::new()?;
    let other_view = other.store.snapshot();
    let foreign = other_view
        .accepted_public_identity_snapshot()?
        .ok_or("foreign accepted owner absent")?;
    check!(accepted.store_identity() != foreign.store_identity());
    let before = retained_state(&fixture.store)?;
    let other_before = retained_state(&other.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay(&bundle, &foreign, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    check!(eq; retained_state(&other.store)?, other_before);
    Ok(())
}

#[test]
fn frozen_replay_refuses_old_sequence_even_when_store_head_and_v3_qualification_match() -> TestResult
{
    let fixture = Fixture::new()?;
    let held = fixture.store.snapshot();
    let old = held
        .accepted_public_identity_snapshot()?
        .ok_or("old accepted owner absent")?;
    attach(&fixture, "frozen-current-sequence")?;
    let (bundle, current) = capture(&fixture.store)?;
    check!(eq; old.store_identity(), current.store_identity());
    check!(eq; old.head(), current.head());
    check!(eq; old.qualification_version(), current.qualification_version());
    check!(old.snapshot_sequence() != current.snapshot_sequence());
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay(&bundle, &old, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::TargetMoved))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn self_hashed_archive_inventory_must_still_equal_the_complete_replayed_ledger() -> TestResult {
    let (fixture, _, _) = committed("frozen-current-inventory")?;
    let (bundle, accepted) = capture(&fixture.store)?;
    let mut archive = archive::read(bundle.bytes(), &mut AttachmentReplayGrantV1::new())?;
    archive.inventory.records = archive
        .inventory
        .records
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    let changed = adopt(serde_json::to_vec(&archive)?)?;
    FrozenRetainedCaptureSourceV1::read(&changed, &mut AttachmentReplayGrantV1::new())?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay(&changed, &accepted, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen attachment inventory differs"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}

#[test]
fn extra_structural_external_prefix_cannot_expand_the_frozen_ledger_closure() -> TestResult {
    let (fixture, _, _) = committed("frozen-current-extra-root")?;
    let (bundle, accepted) = capture(&fixture.store)?;
    let key = "f".repeat(64);
    let raw = RawValue::from_string(format!(
        "{{\"key\":\"{key}\",\"at\":\"synthetic\",\"payload\":null}}"
    ))?;
    let payload = reference(b"null")?;
    let mut archive = archive::read(bundle.bytes(), &mut AttachmentReplayGrantV1::new())?;
    check!(eq; archive.prefixes.len(), 2);
    let schema = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;
    let physical = Store::journal_key(schema.phase(), &key);
    let encoded_bytes = u64::try_from(
        physical
            .len()
            .checked_add(raw.get().len())
            .ok_or(StoreError::CounterOverflow)?,
    )?;
    archive.prefixes.push(archive::Prefix {
        kind: archive::Kind::External,
        root: Some(archive::Root {
            schema,
            key: &key,
            payload: archive::Reference {
                digest: payload.digest.as_str(),
                bytes: payload.bytes,
            },
        }),
        physical_rows: 1,
        encoded_bytes,
        rows: vec![archive::Row {
            key: physical,
            value: &raw,
        }],
    });
    let changed = adopt(serde_json::to_vec(&archive)?)?;
    FrozenRetainedCaptureSourceV1::read(&changed, &mut AttachmentReplayGrantV1::new())?;
    let before = retained_state(&fixture.store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        replay(&changed, &accepted, &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "frozen external closure differs"
        )))
    ));
    poisoned(&mut grant)?;
    check!(eq; retained_state(&fixture.store)?, before);
    Ok(())
}
