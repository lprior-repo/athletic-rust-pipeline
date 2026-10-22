use super::*;

pub(super) fn prepare(
    writer: &PublicIdentityWriterV1,
    store: &Store,
    request: &Request,
    ledger: Ledger,
    capture: graph_operation::Capture,
) -> StoreResult<Pending> {
    let snapshot = store.snapshot();
    let view = VerifiedRawGraph::verify(&snapshot, capture.graph.capture(), capture.graph.seal())?;
    let sources = graph::accepted_sources(&view, &capture.observed_at)?;
    let mut cohort = BTreeSet::new();
    view.for_each_merged::<CanonicalAthlete>(
        AuthorityTableV2::Athletes,
        graph::BUDGET,
        |athlete, _| {
            if athlete.grad_year == request.cohort {
                cohort.insert(athlete.id.cast());
            }
            Ok(())
        },
    )?;
    let (mut records, mut projection) = ledger.history.into_parts();
    let mut items = Vec::new();
    for (candidate, (accepted, source)) in sources {
        if !cohort.contains(&candidate)
            || !matches!(
                projection
                    .resolve_candidate(&candidate)
                    .map_err(PublicIdentityError::from)?,
                PublicResolution::Unknown
            )
        {
            continue;
        }
        let item = prepare_item(
            writer,
            request,
            &capture,
            &candidate,
            accepted,
            source,
            &projection,
        )?;
        projection = projection
            .extend_candidate(&item.record)
            .map_err(PublicIdentityError::from)?;
        push(&mut records, item.record.clone())?;
        push(&mut items, item)?;
    }
    if items.is_empty() {
        return Err(PublicIdentityError::NotSourceBound.into());
    }
    let head = projection.head().clone();
    let history = codec::encode(&records, MAX_PUBLIC_IDENTITY_HISTORY_BYTES)?;
    codec::history(&history, &writer.identity, &head)?;
    Ok(Pending {
        capture,
        items,
        head,
        history,
    })
}

fn prepare_item(
    writer: &PublicIdentityWriterV1,
    request: &Request,
    capture: &graph_operation::Capture,
    candidate: &AthleteCandidateId,
    accepted: AcceptedAthleteIdentity,
    source: SourceIdentity,
    projection: &HistoryProjection,
) -> StoreResult<Item> {
    let operation = request.child(candidate)?;
    let digest = witness::request_digest(&writer.identity, &operation, candidate)?;
    let evidence =
        CandidateDecisionReference::from_accepted(&accepted, crate::public_identity::REPLAY_LIMITS)
            .map_err(PublicIdentityError::from)?;
    let prepared = prepare_next(
        &writer.identity,
        projection.head(),
        HistoryRequest::Establish {
            binding: source_binding(candidate, source, &digest),
            evidence,
        },
        crate::public_identity::REPLAY_LIMITS,
    )
    .map_err(PublicIdentityError::from)?;
    let [public_id] = prepared.established_ids.as_slice() else {
        return Err(PublicIdentityError::Corrupt("establishment cardinality").into());
    };
    let held = HeldGraphWitness::new(
        &operation,
        digest,
        capture.graph.clone(),
        &capture.observed_at,
        &prepared,
        Payload::Source {
            candidate: candidate.clone(),
            decision: Decision::new(accepted.record())?,
            public_id: public_id.as_str().to_owned(),
        },
    )?;
    Ok(Item {
        accepted,
        record: prepared.uncommitted_record,
        witness: held,
    })
}
