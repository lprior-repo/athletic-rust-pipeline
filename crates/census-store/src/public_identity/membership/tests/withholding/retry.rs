use super::*;

#[test]
fn withholding_reservation_retries_exactly_after_journal_growth_and_blocks_split_reuse(
) -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture, "shared-admission")?;
    let Fixture { store, writer, .. } = &fixture;
    let snapshot = store.snapshot();
    let head = writer.head(store)?;
    let prepared = writer.prepare_membership_withholding(
        store,
        "shared-admission",
        request.clone(),
        snapshot.revision(),
        &head,
    )?;
    let original = prepared.record().clone();
    let graph = prepared.graph().clone();
    let approval_bytes = prepared.approval_bytes()?;
    let approval_hash = prepared.approval_digest()?;
    let mut batch = store.db.batch();
    batch.insert(
        &store.journal,
        "synthetic-membership-journal-only",
        b"retained unrelated journal work",
    );
    batch.commit()?;
    let state = retained_state(store)?;
    let retried = writer.prepare_membership_withholding(
        store,
        "shared-admission",
        request.clone(),
        snapshot.revision(),
        &head,
    )?;
    check!(eq; retried.record(), &original);
    check!(eq; retried.graph(), &graph);
    check!(eq; retried.approval_bytes()?, approval_bytes);
    check!(eq; retried.approval_digest()?, approval_hash);
    check!(eq; retained_state(store)?, state);
    let mut changed = request.clone();
    changed.rationale.push_str(" changed");
    check!(matches!(
        writer.prepare_membership_withholding(
            store,
            "shared-admission",
            changed,
            snapshot.revision(),
            &head
        ),
        Err(StoreError::PublicIdentity(
            PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; retained_state(store)?, state);
    use census_domain::public_identity_history::local_adjudication::{
        LocalIdentitySplitV1, SupersededMergeV1,
    };
    let split = LocalIdentitySplitV1 {
        case_id: request.case_id,
        canonical: request.canonical,
        supersedes: vec![SupersededMergeV1 {
            sequence: request.attachment.sequence,
            digest: request.attachment.digest,
        }],
        partitions: Vec::new(),
        unresolved_bindings: Vec::new(),
        affected_record_keys: request.affected_record_keys,
        dispositions: Vec::new(),
        rationale: "Different local operation cannot reuse reserved withholding ID".into(),
    };
    let refused = writer.prepare_local_adjudication(
        store,
        "shared-admission",
        split,
        snapshot.revision(),
        &head,
    );
    check!(matches!(
        refused,
        Err(crate::StoreError::PublicIdentity(
            crate::PublicIdentityError::OperationConflict
        ))
    ));
    check!(eq; retained_state(store)?, state);
    let (bytes, digest) = approval(retried.record(), retried.graph())?;
    let outcome = writer.apply_membership_withholding(store, retried, bytes, digest)?;
    check!(!outcome.replayed());
    check!(eq; outcome.head().sequence, 3);
    check!(eq; outcome.withheld_memberships(), original.request.expected_withheld_memberships.as_slice());
    Ok(())
}

#[test]
fn concurrent_withholding_preparations_adopt_original_timestamp_record_and_approval() -> TestResult
{
    use crate::public_identity::graph_operation::preparation_concurrency::{
        exercise, PreparedBytes,
    };
    let fixture = ready()?;
    let request = request(&fixture, "withholding-distinct-timestamps")?;
    let Fixture { store, writer, .. } = &fixture;
    let head = writer.head(store)?;
    exercise(store, "withholding-distinct-timestamps", || {
        let prepared = writer.prepare_membership_withholding(
            store,
            "withholding-distinct-timestamps",
            request.clone(),
            store.snapshot().revision(),
            &head,
        )?;
        Ok(PreparedBytes {
            record: serde_json::to_vec(prepared.record())?,
            approval: prepared.approval_bytes()?,
            digest: prepared.approval_digest()?,
        })
    })
}
