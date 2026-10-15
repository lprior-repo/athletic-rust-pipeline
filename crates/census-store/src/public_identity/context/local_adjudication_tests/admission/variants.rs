use super::*;
use crate::public_identity::local_adjudication::admission;
use census_domain::public_identity_history::local_adjudication::LocalIdentityAdjudicationRecordV2;
use census_domain::public_identity_history::local_membership_withholding::{
    LocalMembershipWithholdV1, LocalMembershipWithholdingRecordV1, MembershipAttachmentV1,
    LOCAL_MEMBERSHIP_WITHHOLDING_POLICY,
};
use fjall::Readable;

#[test]
fn tagged_reservations_share_one_cap_and_refuse_cross_variant_retries() -> TestResult {
    let fixture = merged()?;
    let request = request(&fixture)?;
    let head = fixture.writer.head(&fixture.store)?;
    let operation = "shared-cap-split";
    let split = fixture.writer.reserve_local_adjudication(
        &fixture.store,
        operation,
        request.clone(),
        fixture.store.snapshot().revision(),
        &head,
    )?;
    let mut record = withholding_record(split.prepared.record())?;
    let identity = fixture.store.store_identity()?;
    let guard = fixture.store.lock_appends();
    let charges = [
        split.inputs.admission_charge(&fixture.store, &identity)?,
        split.affected.admission_charge(&fixture.store)?,
    ];
    let withheld = admission::reserve_withholding(&fixture.store, record.clone(), charges.clone())?;
    assert!(withheld.split_record().is_err());
    assert_eq!(withheld.withholding_record()?, &record);
    assert!(withheld
        .require_request(
            &record.operation,
            &request,
            fixture.store.snapshot_locked().revision(),
            &head
        )
        .is_err());
    let original =
        admission::load(&fixture.store, &identity, operation)?.ok_or("split reservation")?;
    assert!(original
        .require_withholding_request(
            operation,
            &record.request,
            fixture.store.snapshot_locked().revision(),
            &head
        )
        .is_err());
    drop(guard);
    let counters = header(&fixture.store)?;
    assert_eq!(counters["reservations"].as_u64(), Some(2));
    super::accounting::saturate(&fixture.store, operation)?;
    let sequence = fixture.store.snapshot().sequence();
    let _guard = fixture.store.lock_appends();
    record.operation = "shared-cap-fresh-withholding".into();
    assert!(admission::reserve_withholding(&fixture.store, record, charges).is_err());
    assert_eq!(sequence, fixture.store.snapshot_locked().sequence());
    assert_eq!(
        header(&fixture.store)?["bytes"].as_u64(),
        Some(128 * 1024 * 1024)
    );
    Ok(())
}

fn header(store: &crate::Store) -> TestResult<serde_json::Value> {
    let raw = store
        .db
        .snapshot()
        .get(&store.journal, "manual-context-admission-v1\0header")?
        .ok_or("header")?;
    Ok(serde_json::from_slice(&raw)?)
}

fn withholding_record(
    source: &LocalIdentityAdjudicationRecordV2,
) -> TestResult<LocalMembershipWithholdingRecordV1> {
    let attachment = source.request.supersedes.first().ok_or("prior merge")?;
    Ok(LocalMembershipWithholdingRecordV1 {
        version: 1,
        policy: LOCAL_MEMBERSHIP_WITHHOLDING_POLICY.into(),
        store: source.store.clone(),
        operation: "shared-cap-withholding".into(),
        expected_head: source.expected_head.clone(),
        evidence_generation: source.evidence_generation,
        derived_generation: source.derived_generation,
        snapshot_sequence: source.snapshot_sequence,
        authority_context: source.authority_context.clone(),
        affected_context: source.affected_context.clone(),
        members: source.members.clone(),
        request: LocalMembershipWithholdV1 {
            case_id: source.request.case_id.clone(),
            canonical: source.request.canonical.clone(),
            attachment: MembershipAttachmentV1 {
                sequence: attachment.sequence,
                digest: attachment.digest.clone(),
                membership_id: "synthetic-admission-only-variant".into(),
            },
            expected_withheld_memberships: Vec::new(),
            affected_record_keys: source.request.affected_record_keys.clone(),
            rationale: "Synthetic admission bookkeeping only; never approved or applied".into(),
        },
        observed_at: source.observed_at.clone(),
        dispositions: source.request.dispositions.clone(),
    })
}
