use super::*;

pub(super) struct SourcePrepared {
    capture: graph_operation::Capture,
    accepted: census_domain::model::AcceptedAthleteIdentity,
    prepared: census_domain::public_identity_history::PreparedHistoryRecord,
    witness: HeldGraphWitness,
}

pub(super) fn prepare_source(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    candidate: &AthleteCandidateId,
    digest: &str,
    head: &RegistryHead,
    capture: graph_operation::Capture,
) -> StoreResult<SourcePrepared> {
    let snapshot = store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, capture.graph.capture(), capture.graph.seal())?;
    let (accepted, source) = graph::accept_source(&view, candidate, &capture.observed_at)?;
    let evidence =
        CandidateDecisionReference::from_accepted(&accepted, crate::public_identity::REPLAY_LIMITS)
            .map_err(PublicIdentityError::from)?;
    let prepared = prepare_next(
        &writer.identity,
        head,
        HistoryRequest::Establish {
            binding: source_binding(candidate, source, digest),
            evidence,
        },
        crate::public_identity::REPLAY_LIMITS,
    )
    .map_err(PublicIdentityError::from)?;
    let [public_id] = prepared.established_ids.as_slice() else {
        return Err(PublicIdentityError::Corrupt("establishment cardinality").into());
    };
    let witness = HeldGraphWitness::new(
        operation,
        digest.to_owned(),
        capture.graph.clone(),
        &capture.observed_at,
        &prepared,
        Payload::Source {
            candidate: candidate.clone(),
            decision: Decision::new(accepted.record())?,
            public_id: public_id.as_str().to_owned(),
        },
    )?;
    Ok(SourcePrepared {
        capture,
        accepted,
        prepared,
        witness,
    })
}

pub(super) fn finish_source(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    digest: &str,
    expected: StoreRevision<'_>,
    head: &RegistryHead,
    pending: SourcePrepared,
) -> StoreResult<IdentityCommitOutcomeV1> {
    let SourcePrepared {
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
    let crate::public_identity::operation::Replayed::Source(outcome) =
        witness.outcome(&prepared.uncommitted_record, false)?
    else {
        return Err(PublicIdentityError::OperationConflict.into());
    };
    persist_source(
        writer,
        store,
        operation,
        SourcePrepared {
            capture,
            accepted,
            prepared,
            witness,
        },
        ledger,
    )?;
    Ok(outcome)
}

fn persist_source(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    operation: &str,
    pending: SourcePrepared,
    ledger: crate::public_identity::qualification::Ledger,
) -> StoreResult<()> {
    let SourcePrepared {
        capture,
        accepted,
        prepared,
        witness,
    } = pending;
    let (records, _) = ledger.history.into_parts();
    let history = encode_next_history(
        records,
        prepared.uncommitted_record,
        &writer.identity,
        &prepared.next_head,
    )?;
    persist(
        store,
        &witness,
        operation,
        DecisionWrite::Accepted(accepted.record()),
        &prepared.next_head,
        history,
        &capture.observed_at,
    )?;
    Ok(())
}
