use super::*;
use census_domain::public_identity_history::HistoryRequest;

pub(super) fn corrected(
    store: &Store,
    fixture: &control::Controlled,
    sequence: u64,
    merged: [u64; 2],
    branch: Branch,
) -> TestResult<Vec<PublicAthleteId>> {
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("corrected snapshot")?;
    check!(eq; accepted.head().sequence, sequence);
    check!(eq; accepted.affected_record_keys(sequence), Some(fixture.affected.as_slice()));
    let record = accepted
        .history_records()
        .last()
        .ok_or("correction record")?;
    let [canonical, _, _] = &fixture.public;
    let expected = fixture
        .candidates
        .iter()
        .map(|candidate| resolved(accepted.resolve_candidate(candidate)?))
        .collect::<TestResult<Vec<_>>>()?;
    let revoked = match branch {
        Branch::Split => merged.to_vec(),
        Branch::RevokeRedundantMerge => {
            let [first, _] = merged;
            vec![first]
        }
    };
    match branch {
        Branch::Split => {
            check!(matches!(&record.request, HistoryRequest::Split(row)
                if row.partitions == fixture.partitions && row.revoke_merges == revoked
                    && row.unresolved_bindings.is_empty()
                    && row.affected_record_keys == fixture.affected));
            let PublicResolution::Ambiguous {
                successors,
                unresolved_bindings,
                changed_at,
                revoked_merges,
            } = accepted.resolve_public(canonical)?
            else {
                return Err("ambiguous split predecessor".into());
            };
            check!(eq; successors.len(), 3);
            check!(!unresolved_bindings);
            check!(eq; changed_at, sequence);
            check!(eq; revoked_merges, revoked);
            check!(expected.iter().all(|id| !fixture.public.contains(id)));
            check!(eq; expected.iter().collect::<std::collections::BTreeSet<_>>().len(), 3);
            for (candidate, id) in fixture.candidates.iter().zip(&expected) {
                check!(successors
                    .iter()
                    .any(|row| &row.public_id == id && &row.representative_candidate == candidate));
                check!(
                    matches!(accepted.resolve_public(id)?, PublicResolution::Canonical(owner)
                    if &owner.public_id == id && &owner.representative_candidate == candidate)
                );
                let partition = fixture
                    .partitions
                    .iter()
                    .find(|row| &row.representative == candidate)
                    .ok_or("original singleton partition")?;
                check!(eq; accepted.component_bindings(id)?.into_iter().collect::<Vec<_>>(), partition.binding_ids);
            }
        }
        Branch::RevokeRedundantMerge => {
            check!(matches!(&record.request, HistoryRequest::Revoke(row)
                    if row.revoke_merges == revoked
                        && row.affected_record_keys == fixture.affected));
            check!(eq; expected, vec![canonical.clone(); 3]);
            check!(
                matches!(accepted.resolve_public(canonical)?, PublicResolution::Canonical(owner)
                if owner.public_id == *canonical)
            );
        }
    }
    for (alias, expected_id) in fixture.public.iter().zip(&expected).skip(1) {
        check!(
            matches!(accepted.resolve_public(alias)?, PublicResolution::RevokedRedirect { successor, changed_at, revoked_merges }
            if &successor.public_id == expected_id && changed_at == sequence && revoked_merges == revoked)
        );
    }
    Ok(expected)
}

fn resolved(resolution: PublicResolution) -> TestResult<PublicAthleteId> {
    match resolution {
        PublicResolution::Canonical(owner)
        | PublicResolution::Redirect {
            successor: owner, ..
        }
        | PublicResolution::RevokedRedirect {
            successor: owner, ..
        } => Ok(owner.public_id),
        _ => Err("candidate must have one current public identity".into()),
    }
}

pub(super) fn source_claims(store: &Store) -> TestResult<Value> {
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    let head = claims.authority_head_bytes(&mut grant)?.to_vec();
    let values = claims.claims(&mut grant)?.iter().map(|claim| {
        Ok(json!({"operation":claim.operation(), "proposal":claim.proposal(&mut grant)?,
            "review":claim.semantic_review_bytes(&mut grant)?, "approval":claim.approval_bytes(&mut grant)?}))
    }).collect::<TestResult<Vec<_>>>()?;
    check!(eq; values.len(), 1);
    Ok(json!({"head":head,"claims":values}))
}

pub(super) fn source_context(store: &Store) -> TestResult<Value> {
    let snapshot = store.snapshot();
    let fixed = snapshot.tables_digest(&[
        Table::Schools,
        Table::Teams,
        Table::SourceObservations,
        Table::Performances,
        Table::RelayResults,
    ])?;
    let athletes = snapshot.scan::<CanonicalAthlete>(Table::Athletes)?;
    let rows = athletes.iter().map(|row| json!({"id":row.id,"name":row.canonical_name,"school":row.school,"year":row.grad_year,"gender":row.gender,"source":row.source,"evidence":row.evidence})).collect::<Vec<_>>();
    Ok(json!({"physical":fixed,"athlete_source_context":rows}))
}

pub(super) fn candidate_resolutions(
    store: &Store,
) -> TestResult<std::collections::BTreeMap<String, PublicResolution>> {
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted identity oracle")?;
    snapshot
        .scan::<census_domain::model::CanonicalAthlete>(Table::Athletes)?
        .into_iter()
        .map(|row| {
            Ok((
                row.id.to_string(),
                accepted.resolve_candidate(&row.id.cast())?,
            ))
        })
        .collect()
}
