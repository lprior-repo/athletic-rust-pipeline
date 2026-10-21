use super::*;
use census_domain::public_identity_history::HistoryRequest;

mod attachments;
mod consumers;

pub(super) use attachments::{historical_status_target_moved, moved};
pub(super) use consumers::{dataset, publication};

pub(super) fn merged(store: &Store, fixture: &controlled::Controlled, second: u64) -> TestResult {
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("merged snapshot")?;
    check!(eq; accepted.head().sequence, second);
    let [canonical, alias, bystander] = &fixture.public;
    check!(
        matches!(accepted.resolve_public(canonical)?, PublicResolution::Canonical(owner)
        if owner.public_id == *canonical)
    );
    check!(
        matches!(accepted.resolve_public(alias)?, PublicResolution::Redirect { successor, changed_at }
        if successor.public_id == *canonical && changed_at == second)
    );
    check!(
        matches!(accepted.resolve_public(bystander)?, PublicResolution::Redirect { successor, changed_at }
        if successor.public_id == *canonical && changed_at == second)
    );
    for candidate in &fixture.candidates {
        check!(eq; resolved(accepted.resolve_candidate(candidate)?)?, canonical.clone());
    }
    Ok(())
}

pub(super) fn corrected(
    store: &Store,
    fixture: &controlled::Controlled,
    sequence: u64,
    first: u64,
    second: u64,
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
    let [canonical, alias, bystander] = &fixture.public;
    let expected = fixture
        .candidates
        .iter()
        .map(|candidate| resolved(accepted.resolve_candidate(candidate)?))
        .collect::<TestResult<Vec<_>>>()?;
    match branch {
        Branch::Split => {
            check!(matches!(&record.request, HistoryRequest::Split(row)
                if row.partitions == fixture.partitions && row.revoke_merges == vec![first, second]));
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
            check!(eq; revoked_merges, vec![first, second]);
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
                matches!(&record.request, HistoryRequest::Revoke(row) if row.revoke_merges == vec![first])
            );
            check!(eq; expected, vec![canonical.clone(), canonical.clone(), canonical.clone()]);
        }
    }
    let revoked = match branch {
        Branch::Split => vec![first, second],
        Branch::RevokeRedundantMerge => vec![first],
    };
    check!(
        matches!(accepted.resolve_public(alias)?, PublicResolution::RevokedRedirect { successor, changed_at, revoked_merges }
        if Some(&successor.public_id) == expected.get(1) && changed_at == sequence && revoked_merges == revoked)
    );
    check!(matches!(accepted.resolve_public(bystander)?, resolution
    if {
        let target = expected.get(2);
        match branch {
            Branch::Split => matches!(&resolution, PublicResolution::RevokedRedirect { successor, changed_at, revoked_merges }
                if Some(&successor.public_id) == target && *changed_at == sequence && *revoked_merges == vec![first, second]),
            Branch::RevokeRedundantMerge => matches!(&resolution, PublicResolution::RevokedRedirect { successor, changed_at, revoked_merges }
                if Some(&successor.public_id) == target && *changed_at == sequence && *revoked_merges == vec![first]),
        }
    }));
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
