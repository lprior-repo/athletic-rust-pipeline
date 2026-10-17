use super::*;
use census_store::ReviewedIdentityRequestV1;

pub(super) fn merge(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    fixture: &control::Controlled,
) -> TestResult<[u64; 2]> {
    let [first, middle, last] = &fixture.candidates;
    let [canonical, middle_public, last_public] = &fixture.public;
    let mut sequences = [0; 2];
    for ((operation, candidates, other), sequence) in [
        (
            "controlled-reviewed-roster-merge-ab",
            [first.clone(), middle.clone()],
            middle_public,
        ),
        (
            "controlled-reviewed-roster-merge-bc",
            [middle.clone(), last.clone()],
            last_public,
        ),
    ]
    .into_iter()
    .zip(&mut sequences)
    {
        let case_id = review::install(store, &candidates, review::Decision::Same)?;
        let mut members = vec![canonical.clone(), other.clone()];
        members.sort();
        let request = ReviewedIdentityRequestV1::Merge {
            case_id,
            canonical: canonical.clone(),
            members,
        };
        *sequence = apply(store, writer, operation, &request)?;
    }
    Ok(sequences)
}

pub(super) fn correct(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    fixture: &control::Controlled,
    merged: [u64; 2],
    branch: Branch,
) -> TestResult<u64> {
    let [canonical, _, _] = &fixture.public;
    let [first_merge, last_merge] = merged;
    if matches!(branch, Branch::RevokeRedundantMerge) {
        control::additional_review_evidence(store, fixture)?;
        let [first, middle, _] = &fixture.candidates;
        let members = [first.clone(), middle.clone()];
        let case_id = review::install(store, &members, review::Decision::Same)?;
        let request = ReviewedIdentityRequestV1::Merge {
            case_id,
            canonical: canonical.clone(),
            members: vec![canonical.clone()],
        };
        let redundant = apply(
            store,
            writer,
            "controlled-reviewed-roster-redundant-merge",
            &request,
        )?;
        check!(eq; redundant, last_merge.checked_add(1).ok_or("redundant sequence")?);
    }
    let case_id = review::install(store, &fixture.candidates, review::Decision::Different)?;
    if matches!(branch, Branch::Split) {
        lone_bridge_refused(store, writer, fixture, merged, &case_id)?;
    }
    let request = match branch {
        Branch::Split => ReviewedIdentityRequestV1::Split {
            case_id,
            canonical: canonical.clone(),
            revoke_merges: merged.to_vec(),
            partitions: fixture.partitions.clone(),
            unresolved_bindings: Vec::new(),
            affected_record_keys: fixture.affected.clone(),
        },
        Branch::RevokeRedundantMerge => ReviewedIdentityRequestV1::Revoke {
            case_id,
            canonical: canonical.clone(),
            revoke_merges: vec![first_merge],
            affected_record_keys: fixture.affected.clone(),
        },
    };
    apply(
        store,
        writer,
        "controlled-reviewed-roster-correction",
        &request,
    )
}

fn apply(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    operation: &str,
    request: &ReviewedIdentityRequestV1,
) -> TestResult<u64> {
    let head = writer.head(store)?;
    let facts = store.snapshot().tables_digest(&[
        Table::Athletes,
        Table::SourceObservations,
        Table::Performances,
        Table::RelayResults,
    ])?;
    let applied = writer.apply_reviewed_identity(
        store,
        operation,
        request,
        store.snapshot().revision(),
        &head,
    )?;
    check!(!applied.replayed());
    check!(eq; applied.head().sequence, head.sequence.checked_add(1).ok_or("identity sequence")?);
    let expected_births = match request {
        ReviewedIdentityRequestV1::Split { partitions, .. } => partitions.len(),
        ReviewedIdentityRequestV1::Merge { .. } | ReviewedIdentityRequestV1::Revoke { .. } => 0,
    };
    check!(eq; applied.introduced_public_ids().len(), expected_births);
    check!(eq; store.snapshot().tables_digest(&[
        Table::Athletes, Table::SourceObservations, Table::Performances, Table::RelayResults,
    ])?, facts);
    let receipt_key = format!("public_identity_v1:{operation}");
    let receipt = store
        .receipt(&receipt_key)?
        .ok_or("committed public identity receipt")?;
    let receipt_bytes = serde_json::to_vec(&receipt)?;
    let before = (
        store.snapshot().sequence(),
        store.receipt_count()?,
        store.snapshot().tables_digest(&Table::ALL)?,
    );
    let repeated = writer.apply_reviewed_identity(
        store,
        operation,
        request,
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.head(), applied.head());
    check!(eq; repeated.introduced_public_ids(), applied.introduced_public_ids());
    check!(eq; (
        store.snapshot().sequence(),
        store.receipt_count()?,
        store.snapshot().tables_digest(&Table::ALL)?,
    ), before);
    check!(eq; serde_json::to_vec(&store.receipt(&receipt_key)?.ok_or("replayed public identity receipt")?)?, receipt_bytes);
    Ok(applied.head().sequence)
}

fn lone_bridge_refused(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    fixture: &control::Controlled,
    merged: [u64; 2],
    case_id: &str,
) -> TestResult {
    let [canonical, _, _] = &fixture.public;
    let request = ReviewedIdentityRequestV1::Revoke {
        case_id: case_id.into(),
        canonical: canonical.clone(),
        revoke_merges: merged.to_vec(),
        affected_record_keys: fixture.affected.clone(),
    };
    let head = writer.head(store)?;
    let facts = store.snapshot().tables_digest(&Table::ALL)?;
    for _ in 0..2 {
        let result = writer.apply_reviewed_identity(
            store,
            "controlled-lone-bridge-revoke-refused",
            &request,
            store.snapshot().revision(),
            &head,
        );
        check!(matches!(
            result,
            Err(census_store::StoreError::PublicIdentity(
                census_store::PublicIdentityError::History(
                    census_domain::public_identity_history::HistoryError::SplitRequired
                )
            ))
        ));
        check!(eq; writer.head(store)?, head);
        check!(eq; store.snapshot().tables_digest(&Table::ALL)?, facts);
    }
    Ok(())
}
