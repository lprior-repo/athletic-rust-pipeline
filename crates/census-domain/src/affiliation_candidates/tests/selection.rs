use super::{fixture::*, TestResult};
use crate::affiliation_candidates::*;

#[test]
fn empty_input_is_unknown_without_canonical_school_fallback() -> TestResult {
    let fixture = Fixture::new()?;
    let result = select_candidates(fixture.query()?, &[], &[])?;
    check!(matches!(result.outcome, CandidateOutcome::Unknown));
    check!(result.facts.is_empty());
    Ok(())
}

#[test]
fn distinct_schools_conflict_in_both_arrival_orders() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let a =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-B")?)?;
    raw.owner.target = OwnerTarget::School(&fixture.other_school);
    raw.relationship = RelationshipCandidate::EnrolledAtSchool(&fixture.other_school);
    let b = LocallyCheckedAssertion::check(raw)?;
    let forward = [a, b];
    let reverse = [b, a];
    let first = select_candidates(fixture.query()?, &forward, &[])?;
    let second = select_candidates(fixture.query()?, &reverse, &[])?;
    check!(matches!(first.outcome, CandidateOutcome::Conflicted));
    check!(eq; first.outcome, second.outcome);
    check!(eq; first.facts, second.facts);
    check!(eq; first.facts.len(), 2);
    Ok(())
}

#[test]
fn mapped_other_person_cannot_supply_queried_athlete() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.person = PersonCandidate::Mapped {
        person: &fixture.person,
        athlete: &fixture.other_athlete,
        decision: key("synthetic-other-binding")?,
        known: known(1, 3)?,
    };
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let result = select_candidates(fixture.query()?, &claims, &[])?;
    check!(matches!(result.outcome, CandidateOutcome::Unknown));
    check!(matches!(
        result.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::DifferentAthlete,
            ..
        })
    ));
    Ok(())
}

#[test]
fn missing_person_key_remains_subject_unresolved() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.person = PersonCandidate::MissingKey;
    let claims = [LocallyCheckedAssertion::check(raw)?];
    let result = select_candidates(fixture.query()?, &claims, &[])?;
    check!(matches!(
        result.outcome,
        CandidateOutcome::SubjectUnresolved
    ));
    check!(matches!(
        result.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::SubjectUnresolved(SubjectGap::MissingPersonKey),
            ..
        })
    ));
    Ok(())
}

#[test]
fn withdrawal_removes_trust_without_asserting_departure() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claim =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let claims = [claim];
    let changes = [fixture.change(
        &evidence,
        claim.raw().id,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?];
    let result = select_candidates(fixture.query()?, &claims, &changes)?;
    check!(matches!(result.outcome, CandidateOutcome::Unknown));
    check!(matches!(
        result.facts.first(),
        Some(FactDisposition::Assertion {
            disposition: Disposition::WithdrawnClaim,
            ..
        })
    ));
    check!(eq; result.facts.len(), 2);
    Ok(())
}

#[test]
fn withdrawal_cannot_reuse_an_assertion_id_even_before_it_is_visible() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claim =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let claims = [claim];
    let change = fixture.change(
        &evidence,
        claim.raw().id,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?;
    for first in [1, 2] {
        let mut raw = *change.raw();
        raw.id = claim.raw().id;
        raw.known = known(first, 3)?;
        let changes = [LocallyCheckedChange::check(raw)?];
        check!(matches!(
            select_candidates(fixture.query()?, &claims, &changes),
            Err(AffiliationCandidateError::DuplicateIdDifferentPayload)
        ));
    }
    Ok(())
}

#[test]
fn distinct_withdrawal_id_removes_only_its_exact_claim_and_replays_once() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let first =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let retained =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-B")?)?)?;
    let change = fixture.change(
        &evidence,
        first.raw().id,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?;
    let claims = [first, retained, first];
    let changes = [change, change];
    let selected = select_candidates(fixture.query()?, &claims, &changes)?;
    check!(eq; selected.outcome, CandidateOutcome::SingleCandidate(&retained));
    check!(eq; selected.facts.len(), 3);
    check!(eq;
        selected.facts,
        vec![
            FactDisposition::Assertion {
                fact: &first,
                disposition: Disposition::WithdrawnClaim,
            },
            FactDisposition::Assertion {
                fact: &retained,
                disposition: Disposition::Candidate,
            },
            FactDisposition::Change {
                fact: &change,
                disposition: Disposition::ChangeApplied,
            },
        ]
    );
    Ok(())
}

#[test]
fn departure_keeps_last_day_inclusive_and_next_day_ended() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claim =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let claims = [claim];
    let last = day("2026-09-30")?;
    let next = day("2026-10-01")?;
    let changes = [fixture.change(
        &evidence,
        claim.raw().id,
        ChangeKind::EndsRelationship { last_day: &last },
    )?];
    let mut query = fixture.query()?;
    query.as_of = &last;
    check!(eq; select_candidates(query, &claims, &changes)?.outcome,
    CandidateOutcome::SingleCandidate(&claim));
    query.as_of = &next;
    check!(matches!(
        select_candidates(query, &claims, &changes)?.outcome,
        CandidateOutcome::EndedWithoutReplacement
    ));
    Ok(())
}

#[test]
fn later_committed_withdrawal_does_not_change_older_known_selection() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claim =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let claims = [claim];
    let mut change = *fixture
        .change(
            &evidence,
            claim.raw().id,
            ChangeKind::WithdrawsClaim {
                scope: &fixture.interval,
            },
        )?
        .raw();
    change.known = known(2, 3)?;
    let changes = [LocallyCheckedChange::check(change)?];
    let older = select_candidates(fixture.query()?, &claims, &changes)?;
    check!(eq; older.outcome, CandidateOutcome::SingleCandidate(&claim));
    let mut newer = fixture.query()?;
    newer.known.revision = revision(2)?;
    check!(matches!(
        select_candidates(newer, &claims, &changes)?.outcome,
        CandidateOutcome::Unknown
    ));
    check!(matches!(
        older.facts.last(),
        Some(FactDisposition::Change {
            disposition: Disposition::NotVisibleAtKnownRevision,
            ..
        })
    ));
    Ok(())
}

#[test]
fn missing_or_foreign_change_target_refuses() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let changes = [fixture.change(
        &evidence,
        key("absent-claim")?,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?];
    check!(matches!(
        select_candidates(fixture.query()?, &[], &changes),
        Err(AffiliationCandidateError::DanglingOrForeignChangeTarget)
    ));
    Ok(())
}

#[test]
fn unresolved_correction_policy_never_chooses_replacement() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claims = [LocallyCheckedAssertion::check(fixture.assertion(
        &evidence,
        &owners,
        key("claim-A")?,
    )?)?];
    let changes = [fixture.change(
        &evidence,
        key("claim-A")?,
        ChangeKind::Corrects {
            replacement: key("claim-B")?,
            scope: &fixture.interval,
        },
    )?];
    check!(matches!(
        select_candidates(fixture.query()?, &claims, &changes),
        Err(AffiliationCandidateError::UnsupportedChange)
    ));
    Ok(())
}

#[test]
fn identical_replay_deduplicates_but_same_id_different_payload_refuses() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    let claim = LocallyCheckedAssertion::check(raw)?;
    let duplicate = [claim, claim];
    check!(eq; select_candidates(fixture.query()?, &duplicate, &[])?
        .facts
        .len(),
    1);
    let mut changed = raw;
    changed.temporal = TemporalScope::Unresolved(TemporalGap::Undated);
    let different = [claim, LocallyCheckedAssertion::check(changed)?];
    check!(matches!(
        select_candidates(fixture.query()?, &different, &[]),
        Err(AffiliationCandidateError::DuplicateIdDifferentPayload)
    ));
    Ok(())
}

#[test]
fn mixed_lineage_and_hidden_dependency_refuse() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let mut raw = fixture.assertion(&evidence, &owners, key("claim-A")?)?;
    raw.known = KnownBounds::new(key("different-lineage")?, revision(1)?, revision(3)?)?;
    let claims = [LocallyCheckedAssertion::check(raw)?];
    check!(matches!(
        select_candidates(fixture.query()?, &claims, &[]),
        Err(AffiliationCandidateError::MixedKnownLineage)
    ));
    raw.known = known(1, 3)?;
    raw.owner.known = known(2, 3)?;
    let claims = [LocallyCheckedAssertion::check(raw)?];
    check!(matches!(
        select_candidates(fixture.query()?, &claims, &[]),
        Err(AffiliationCandidateError::DependencyNotVisible)
    ));
    Ok(())
}

#[test]
fn present_change_target_requires_exact_source_person_and_mapped_athlete() -> TestResult {
    let fixture = Fixture::new()?;
    let evidence = fixture.evidence()?;
    let owners = [&fixture.owner];
    let claim =
        LocallyCheckedAssertion::check(fixture.assertion(&evidence, &owners, key("claim-A")?)?)?;
    let claims = [claim];
    let other_person = crate::model::SourceSubject::new(
        crate::model::SourceNamespace::MilesplitAthlete,
        "700002".to_owned(),
    )?;
    let change = fixture.change(
        &evidence,
        key("claim-A")?,
        ChangeKind::WithdrawsClaim {
            scope: &fixture.interval,
        },
    )?;
    for (person, athlete) in [
        (&other_person, &fixture.athlete),
        (&fixture.person, &fixture.other_athlete),
    ] {
        let mut raw = *change.raw();
        raw.person = PersonCandidate::Mapped {
            person,
            athlete,
            decision: key("synthetic-other-binding")?,
            known: known(1, 3)?,
        };
        let changes = [LocallyCheckedChange::check(raw)?];
        check!(matches!(
            select_candidates(fixture.query()?, &claims, &changes),
            Err(AffiliationCandidateError::DanglingOrForeignChangeTarget)
        ));
    }
    Ok(())
}
