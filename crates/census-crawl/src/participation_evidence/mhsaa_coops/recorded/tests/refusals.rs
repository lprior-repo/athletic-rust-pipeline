use super::fixture::*;
use super::*;
use census_domain::evidence_subject_attachment::AttachmentDigestV1;
use census_store::{RetainedCaptureLeafV1, RetainedCaptureSlotV1};

#[test]
fn wrong_schema_slot_hash_length_and_version_refuse_cold_without_writes_and_poison_grant(
) -> fixture::TestResult {
    let (directory, store) = fresh()?;
    let packet = super::super::super::packet::tests::packet()?;
    stage(&store, &packet)?.apply(&store)?;
    let leaf = exact(&store, &packet)?;
    drop(store);
    let store = reopen(&directory)?;
    let before = state(&store)?;
    let mut cases: Vec<RetainedCaptureLeafV1> = Vec::new();
    let mut altered = leaf.clone();
    altered.root.schema = RetainedCaptureRootSchemaV1::HtmlResultCaptureV1;
    cases.push(altered);
    let mut altered = leaf.clone();
    altered.slot = RetainedCaptureSlotV1::ProviderProfile;
    cases.push(altered);
    let mut altered = leaf.clone();
    altered.object.digest = AttachmentDigestV1::parse(&"0".repeat(64))?;
    cases.push(altered);
    let mut altered = leaf.clone();
    altered.object.bytes = altered
        .object
        .bytes
        .checked_add(1)
        .ok_or("length overflow")?;
    cases.push(altered);
    let mut altered = leaf.clone();
    altered.version = 2;
    cases.push(altered);
    for altered in cases {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(store
            .snapshot()
            .verify_retained_capture_leaf_v1(&altered, &mut grant)
            .is_err());
        let usage = grant.usage();
        check!(grant.ensure_available().is_err());
        check!(store
            .snapshot()
            .verify_retained_capture_leaf_v1(&leaf, &mut grant)
            .is_err());
        check!(eq; grant.usage(), usage);
        check!(eq; state(&store)?, before);
    }
    check!(eq; exact(&store, &packet)?, leaf);
    check!(eq; state(&store)?, before);
    Ok(())
}

#[test]
fn exhausted_shared_read_grant_refuses_payload_without_writes_and_preserves_poisoned_debits(
) -> fixture::TestResult {
    let (_directory, store) = fresh()?;
    let packet = super::super::super::packet::tests::packet()?;
    stage(&store, &packet)?.apply(&store)?;
    let before = state(&store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    grant.charge_bytes(AttachmentReplayGrantV1::MAX_ENCODED_BYTES)?;
    let usage = grant.usage();
    check!(read_recorded_mhsaa_coop_pdf_packet(
        &store.snapshot(),
        &packet.reference()?,
        &mut grant
    )
    .is_err());
    check!(grant.ensure_available().is_err());
    check!(eq; grant.usage().encoded_bytes(), usage.encoded_bytes());
    check!(grant.usage().traversal_steps() > usage.traversal_steps());
    let refused = grant.usage();
    check!(read_recorded_mhsaa_coop_pdf_packet(
        &store.snapshot(),
        &packet.reference()?,
        &mut grant
    )
    .is_err());
    check!(eq; grant.usage(), refused);
    check!(eq; state(&store)?, before);
    Ok(())
}
