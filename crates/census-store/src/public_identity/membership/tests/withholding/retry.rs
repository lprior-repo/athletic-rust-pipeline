use super::*;

#[test]
fn withholding_reservation_retries_exactly_after_journal_growth_and_blocks_split_reuse(
) -> TestResult {
    let fixture = ready()?;
    let request = request(&fixture)?;
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
    check!(eq; retained_state(store)?, state);
    let mut changed = request.clone();
    changed.rationale.push_str(" changed");
    check!(writer
        .prepare_membership_withholding(
            store,
            "shared-admission",
            changed,
            snapshot.revision(),
            &head
        )
        .is_err());
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
            crate::PublicIdentityError::Corrupt("manual admission missing, corrupt, or exhausted")
        ))
    ));
    check!(eq; retained_state(store)?, state);
    let (bytes, digest) = approval(retried.record())?;
    writer.apply_membership_withholding(store, retried, bytes, digest)?;
    Ok(())
}
