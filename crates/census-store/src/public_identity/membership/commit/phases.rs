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
    request: &SourceCandidateMembershipRequestV1,
    digest: &str,
    head: &RegistryHead,
    capture: graph_operation::Capture,
    ledger: &Ledger,
) -> StoreResult<Pending> {
    let snapshot = store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, capture.graph.capture(), capture.graph.seal())?;
    let accepted = graph::accept_reviewed(&view, &request.case_id, &capture.observed_at)?;
    let anchor = ledger
        .history
        .projection()
        .binding_claim(&request.anchor_binding_id)
        .ok_or(PublicIdentityError::NotSourceBound)?;
    let rows = graph::member_rows(
        &view,
        &[anchor.candidate.clone(), request.candidate.clone()],
    )?;
    let authorized = request::authorized_rows(
        request,
        digest,
        &rows,
        &accepted,
        ledger.history.projection(),
    )?;
    let prepared = prepare_next(&writer.identity, head, authorized, ReplayLimits::SMALL)
        .map_err(PublicIdentityError::from)?;
    if !prepared.established_ids.is_empty() {
        return Err(PublicIdentityError::Corrupt("membership introduced public IDs").into());
    }
    let witness = HeldGraphWitness::new(
        operation,
        digest.to_owned(),
        capture.graph.clone(),
        &capture.observed_at,
        &prepared,
        Payload::Membership {
            request: request.clone(),
            decision: Decision::new(accepted.record())?,
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
) -> StoreResult<SourceCandidateMembershipOutcomeV1> {
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
) -> StoreResult<SourceCandidateMembershipOutcomeV1> {
    let Pending {
        capture,
        accepted,
        prepared,
        witness,
    } = pending;
    let operation::Replayed::Membership(outcome) =
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
