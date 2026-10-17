use super::*;
use census_store::ReviewedIdentityRequestV1;

pub(super) fn merge(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    fixture: &control::Controlled,
) -> TestResult<u64> {
    let case_id = review::install(store, &fixture.candidates, review::Decision::Same)?;
    let [canonical, _] = &fixture.public;
    let mut members = fixture.public.to_vec();
    members.sort();
    let request = ReviewedIdentityRequestV1::Merge {
        case_id,
        canonical: canonical.clone(),
        members,
    };
    apply(store, writer, "controlled-reviewed-roster-merge", &request)
}

pub(super) fn correct(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    fixture: &control::Controlled,
    merged: u64,
    branch: Branch,
) -> TestResult<u64> {
    let [canonical, _] = &fixture.public;
    if matches!(branch, Branch::RevokeRedundantMerge) {
        control::additional_review_evidence(store, fixture)?;
        let case_id = review::install(store, &fixture.candidates, review::Decision::Same)?;
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
        check!(eq; redundant, merged.checked_add(1).ok_or("redundant sequence")?);
    }
    let case_id = review::install(store, &fixture.candidates, review::Decision::Different)?;
    if matches!(branch, Branch::Split) {
        lone_bridge_refused(store, writer, fixture, merged, &case_id)?;
    }
    let request = match branch {
        Branch::Split => ReviewedIdentityRequestV1::Split {
            case_id,
            canonical: canonical.clone(),
            revoke_merges: vec![merged],
            partitions: fixture.partitions.clone(),
            unresolved_bindings: Vec::new(),
            affected_record_keys: fixture.affected.clone(),
        },
        Branch::RevokeRedundantMerge => ReviewedIdentityRequestV1::Revoke {
            case_id,
            canonical: canonical.clone(),
            revoke_merges: vec![merged],
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
    check!(eq; store.snapshot().tables_digest(&[
        Table::Athletes, Table::SourceObservations, Table::Performances, Table::RelayResults,
    ])?, facts);
    let receipt_key = format!("public_identity_v1:{operation}");
    let receipt = store
        .receipt(&receipt_key)?
        .ok_or("committed public identity receipt")?;
    let receipt_bytes = serde_json::to_vec(&receipt)?;
    let before = (store.snapshot().sequence(), store.receipt_count()?);
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
    check!(eq; (store.snapshot().sequence(), store.receipt_count()?), before);
    check!(eq; serde_json::to_vec(&store.receipt(&receipt_key)?.ok_or("replayed public identity receipt")?)?, receipt_bytes);
    Ok(applied.head().sequence)
}

fn lone_bridge_refused(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    fixture: &control::Controlled,
    merged: u64,
    case_id: &str,
) -> TestResult {
    let [canonical, _] = &fixture.public;
    let request = ReviewedIdentityRequestV1::Revoke {
        case_id: case_id.into(),
        canonical: canonical.clone(),
        revoke_merges: vec![merged],
        affected_record_keys: fixture.affected.clone(),
    };
    let head = writer.head(store)?;
    let facts = store.snapshot().tables_digest(&Table::ALL)?;
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
    Ok(())
}
