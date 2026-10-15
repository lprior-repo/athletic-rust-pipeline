use super::*;

pub(super) struct Pending {
    capture: graph_operation::Capture,
    accepted: census_domain::model::AcceptedAthleteIdentity,
    prepared: census_domain::public_identity_history::PreparedHistoryRecord,
    witness: HeldGraphWitness,
}

pub(super) fn prepare(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    request: &ReviewedIdentityRequestV1,
    digest: &str,
    head: &RegistryHead,
    capture: graph_operation::Capture,
    ledger: &Ledger,
) -> StoreResult<Pending> {
    let snapshot = store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, capture.graph.capture(), capture.graph.seal())?;
    let accepted = graph::accept_reviewed(&view, request.case_id(), &capture.observed_at)?;
    crate::public_identity::correction::affected::validate(
        &view,
        ledger.history.projection(),
        request,
    )?;
    let prepared = prepare_next(
        &writer.identity,
        head,
        request.authorized(&accepted)?,
        ReplayLimits::SMALL,
    )
    .map_err(PublicIdentityError::from)?;
    if &ReviewedIdentityRequestV1::from_history(
        request.case_id(),
        &prepared.uncommitted_record.request,
    )? != request
    {
        return Err(PublicIdentityError::NonCanonicalRequest.into());
    }
    let witness = HeldGraphWitness::new(
        operation,
        digest.to_owned(),
        capture.graph.clone(),
        &capture.observed_at,
        &prepared,
        Payload::Reviewed {
            kind: request.kind(),
            case_id: request.case_id().into(),
            decision: Decision::new(accepted.record())?,
            introduced_public_ids: prepared
                .established_ids
                .iter()
                .map(|id| id.as_str().to_owned())
                .collect(),
        },
    )?;
    Ok(Pending {
        capture,
        accepted,
        prepared,
        witness,
    })
}

pub(super) fn finish(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    digest: &str,
    expected: StoreRevision<'_>,
    head: &RegistryHead,
    pending: Pending,
) -> StoreResult<ReviewedIdentityCommitOutcomeV1> {
    let Pending {
        capture,
        accepted,
        prepared,
        witness,
    } = pending;
    let _guard = store.lock_appends();
    let ledger = writer.verified(store)?;
    if let Some(outcome) = repeated(store, operation, digest, &ledger)? {
        return Ok(outcome);
    }
    let ledger =
        graph_operation::require_current(writer, store, operation, expected, head, &capture.graph)?;
    graph_operation::require_available_live(store, &writer.identity, &ledger, &witness)?;
    ledger
        .history
        .projection()
        .clone()
        .extend_candidate(&prepared.uncommitted_record)
        .map_err(PublicIdentityError::from)?;
    graph_operation::reserve(
        store,
        operation,
        digest,
        expected,
        head,
        &capture,
        witness.clone(),
    )?;
    persist_pending(
        writer,
        store,
        operation,
        Pending {
            capture,
            accepted,
            prepared,
            witness,
        },
        ledger,
    )
}

fn persist_pending(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    pending: Pending,
    ledger: Ledger,
) -> StoreResult<ReviewedIdentityCommitOutcomeV1> {
    let Pending {
        capture,
        accepted,
        prepared,
        witness,
    } = pending;
    let operation::Replayed::Reviewed(outcome) =
        witness.outcome(&prepared.uncommitted_record, false)?
    else {
        return Err(PublicIdentityError::OperationConflict.into());
    };
    let (records, _) = ledger.history.into_parts();
    let history = commit::encode_next_history(
        records,
        prepared.uncommitted_record,
        &writer.identity,
        &prepared.next_head,
    )?;
    commit::persist(
        store,
        &witness,
        operation,
        commit::DecisionWrite::Accepted(accepted.record()),
        &prepared.next_head,
        history,
        &capture.observed_at,
    )?;
    Ok(outcome)
}
