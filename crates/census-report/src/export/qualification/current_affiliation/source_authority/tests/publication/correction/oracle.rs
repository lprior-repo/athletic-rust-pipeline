use super::*;
use census_domain::public_identity_history::HistoryRequest;

pub(super) fn corrected(
    store: &Store,
    fixture: &control::Controlled,
    sequence: u64,
    merged: u64,
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
    let [canonical, alias] = &fixture.public;
    let expected = fixture
        .candidates
        .iter()
        .map(|candidate| resolved(accepted.resolve_candidate(candidate)?))
        .collect::<TestResult<Vec<_>>>()?;
    match branch {
        Branch::Split => {
            check!(matches!(&record.request, HistoryRequest::Split(row)
                if row.partitions == fixture.partitions && row.revoke_merges == vec![merged]));
            let PublicResolution::Ambiguous {
                successors,
                unresolved_bindings,
                changed_at,
                revoked_merges,
            } = accepted.resolve_public(canonical)?
            else {
                return Err("ambiguous split predecessor".into());
            };
            check!(eq; successors.len(), 2);
            check!(!unresolved_bindings);
            check!(eq; changed_at, sequence);
            check!(eq; revoked_merges, vec![merged]);
            check!(expected.iter().all(|id| !fixture.public.contains(id)));
            check!(ne; expected.first(), expected.last());
            for (candidate, id) in fixture.candidates.iter().zip(&expected) {
                check!(successors
                    .iter()
                    .any(|row| &row.public_id == id && &row.representative_candidate == candidate));
            }
        }
        Branch::RevokeRedundantMerge => {
            check!(
                matches!(&record.request, HistoryRequest::Revoke(row) if row.revoke_merges == vec![merged])
            );
            check!(eq; expected, vec![canonical.clone(), canonical.clone()]);
        }
    }
    check!(
        matches!(accepted.resolve_public(alias)?, PublicResolution::RevokedRedirect { successor, changed_at, revoked_merges }
        if Some(&successor.public_id) == expected.last() && changed_at == sequence && revoked_merges == vec![merged])
    );
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
