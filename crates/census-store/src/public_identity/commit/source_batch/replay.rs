use super::*;

pub(super) fn replay(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    request: &Request,
    ledger: &Ledger,
) -> StoreResult<Option<Vec<IdentityCommitOutcomeV1>>> {
    let snapshot = store.db.snapshot();
    let raw = codec::load(&snapshot, &store.receipts, &request.operation, 65_536)?;
    let receipt: Option<crate::Receipt> = raw
        .as_ref()
        .map(|raw| codec::decode(&request.operation, raw))
        .transpose()?;
    validate_completion_receipt(request, &receipt)?;
    let mut outcomes = Vec::new();
    let mut seen = BTreeSet::new();
    let mut graph = None;
    for record in ledger.history.records() {
        let key = witness::key(record.sequence);
        let raw = codec::load(
            &snapshot,
            &store.journal,
            &key,
            crate::public_identity::local_adjudication::MAX_WITNESS_BYTES,
        )?
        .ok_or(PublicIdentityError::Corrupt(
            "missing cohort replay witness",
        ))?;
        let held = HeldGraphWitness::decode(&key, &raw)?;
        if !held.operation().starts_with(&request.prefix) {
            continue;
        }
        let Some(receipt) = &receipt else {
            return Err(
                PublicIdentityError::Corrupt("cohort children without completion receipt").into(),
            );
        };
        replay_child(writer, request, receipt, &held, &mut seen)?;
        if let Some(previous) = outcomes.last() {
            let previous: &IdentityCommitOutcomeV1 = previous;
            if previous.head().sequence.checked_add(1) != Some(record.sequence) {
                return Err(PublicIdentityError::Corrupt("cohort child sequence").into());
            }
        }
        if graph.as_ref().is_some_and(|prior| prior != held.graph()) {
            return Err(PublicIdentityError::Corrupt("cohort child graph").into());
        }
        graph = Some(held.graph().clone());
        push(&mut outcomes, outcome(&held, record, true)?)?;
    }
    let Some(receipt) = receipt else {
        return Ok(None);
    };
    if u64::try_from(outcomes.len()).map_err(|_| StoreError::CounterOverflow)? != receipt.appended {
        return Err(PublicIdentityError::Corrupt("cohort completion count").into());
    }
    replay_graph(writer, store, request, graph.as_ref())?;
    Ok(Some(outcomes))
}

fn replay_child(
    writer: &PublicIdentityWriterV1,
    request: &Request,
    receipt: &crate::Receipt,
    held: &HeldGraphWitness,
    seen: &mut BTreeSet<AthleteCandidateId>,
) -> StoreResult<()> {
    let Payload::Source { candidate, .. } = &held.payload else {
        return Err(PublicIdentityError::Corrupt("cohort child payload").into());
    };
    if held.operation() != request.child(candidate)?
        || held.request_digest()
            != witness::request_digest(&writer.identity, held.operation(), candidate)?
        || held.observed_at() != receipt.at
        || !seen.insert(candidate.clone())
    {
        return Err(PublicIdentityError::Corrupt("cohort child binding").into());
    }
    Ok(())
}

fn replay_graph(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    request: &Request,
    graph: Option<&crate::public_identity::graph_reference::GraphAuthorityReferenceV1>,
) -> StoreResult<()> {
    let operation = graph_operation::raw_operation(&writer.identity, &request.operation)?;
    let reserved = crate::public_identity::local_adjudication::admission::graph::load_graph(
        store,
        &writer.identity,
        &operation,
    )?
    .ok_or(PublicIdentityError::Corrupt("cohort raw reservation"))?;
    let graph = graph.ok_or(PublicIdentityError::Corrupt("cohort graph missing"))?;
    if graph.capture() != reserved.capture() || graph.seal() != reserved.seal() {
        return Err(PublicIdentityError::Corrupt("cohort raw operation binding").into());
    }
    Ok(())
}

fn validate_completion_receipt(
    request: &Request,
    receipt: &Option<crate::Receipt>,
) -> StoreResult<()> {
    if let Some(receipt) = receipt {
        if receipt.digest != request.digest {
            return Err(PublicIdentityError::OperationConflict.into());
        }
        if receipt.operation != request.operation || receipt.appended == 0 || receipt.credited {
            return Err(PublicIdentityError::Corrupt("cohort completion receipt").into());
        }
    }
    Ok(())
}
