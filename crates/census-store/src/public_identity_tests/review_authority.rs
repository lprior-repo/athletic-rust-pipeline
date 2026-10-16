use crate::PublicIdentityError;
use census_domain::model::{
    separate_generic_identity_cases, AthleteIdentityIndex, CanonicalAthlete, IdentityApplication,
    IdentityProjectionBuilder, ReviewCase, ReviewState, ReviewVerdictRecord,
    ATHLETE_IDENTITY_FAMILY,
};

use super::support::refused;
use super::*;

#[test]
fn public_identity_specific_pending_resolved_and_rejected_cases_block_establishment() -> TestResult
{
    for state in [
        ReviewState::Pending,
        ReviewState::Resolved,
        ReviewState::Retained,
    ] {
        let (_directory, store, writer) = fixture()?;
        let athlete = subject("blocked-review", "301")?;
        store.append(Table::Athletes, &athlete)?;
        let mut case = specific_case(&athlete)?;
        case.state = state;
        store.replace(Table::ReviewCases, &case)?;
        if state == ReviewState::Retained {
            store.replace(Table::IdentityVerdicts, &rejected_verdict(&case))?;
        }
        let before = retained_state(&store)?;
        let head = writer.head(&store)?;
        let error = refused(writer.establish_source_bound(
            &store,
            "blocked-review",
            &athlete.id.cast(),
            store.snapshot().revision(),
            &head,
        ))?;
        check!(matches!(
            error,
            StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
        ));
        check!(eq; writer.head(&store)?, head);
        let staged = retained_state(&store)?;
        super::support::assert_graph_staging(
            &store,
            &before,
            &staged,
            writer.store_identity(),
            "blocked-review",
        )?;
        let retry = refused(writer.establish_source_bound(
            &store,
            "blocked-review",
            &athlete.id.cast(),
            store.snapshot().revision(),
            &head,
        ))?;
        check!(matches!(
            retry,
            StoreError::PublicIdentity(PublicIdentityError::NotSourceBound)
        ));
        check!(eq; writer.head(&store)?, head);
        check!(eq; retained_state(&store)?, staged);
    }
    Ok(())
}

#[test]
fn public_identity_supersedable_generic_case_keeps_canonical_legacy_classification() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("supersedable-review", "302")?;
    store.append(Table::Athletes, &athlete)?;
    let generic = ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        athlete.id.as_str(),
        "Synthetic generic identity",
        "Unbound context",
    );
    let mut stale = specific_case(&athlete)?;
    stale.state = ReviewState::Resolved;
    stale.detail = "Changed retained evidence makes this binding stale".into();
    let (supersedable, bound) =
        separate_generic_identity_cases(vec![generic.clone(), stale.clone()], &[]);
    check!(eq; supersedable, vec![generic.clone()]);
    check!(eq; bound, vec![stale.clone()]);
    let mut index = AthleteIdentityIndex::default();
    index.observe(&athlete)?;
    let builder = IdentityProjectionBuilder::new(index, &bound, &[])?;
    let expected = builder
        .source_applications("2026-10-14")
        .next()
        .ok_or("missing application")??;
    check!(matches!(expected, IdentityApplication::Accepted(_)));
    store.replace_many(Table::ReviewCases, &[generic.clone(), stale.clone()])?;
    let head = writer.head(&store)?;
    let committed = writer.establish_source_bound(
        &store,
        "supersedable-review",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &head,
    )?;
    check!(!committed.replayed());
    check!(eq; committed.head().sequence, 1);
    let retained = store.scan::<ReviewCase>(Table::ReviewCases)?;
    check!(eq; retained.len(), 2);
    check!(retained.contains(&generic));
    check!(retained.contains(&stale));
    Ok(())
}

fn specific_case(athlete: &CanonicalAthlete) -> TestResult<ReviewCase> {
    let mut index = AthleteIdentityIndex::default();
    index.observe(athlete)?;
    let members = vec![athlete.id.cast()];
    let subject = "Synthetic specific identity concern";
    let detail = "Bound retained evidence requires adjudication";
    let evidence = index.case_evidence(subject, detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        athlete.id.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = members;
    Ok(case)
}

fn rejected_verdict(case: &ReviewCase) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "rejected".into(),
        field: "identity".into(),
        value: "insufficient_evidence".into(),
        accepted: false,
        confidence: 0,
        rationale: "Synthetic rejection remains retained".into(),
        reviewer: "fixture".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    }
}
