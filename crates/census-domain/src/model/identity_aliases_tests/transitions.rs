use super::{component_chain, component_member, component_review};
use crate::model::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn context() -> TestResult<(
    [CanonicalAthlete; 3],
    [ReviewCase; 2],
    [ReviewVerdictRecord; 2],
)> {
    let owner = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1001");
    let mut rows = [
        component_member("School A", owner.clone()),
        component_member("School B", owner.clone()),
        component_member("School C", owner),
    ];
    rows.sort_by(|left, right| left.id.cmp(&right.id));
    let index = index(&rows)?;
    let [smaller, middle, larger] = &rows;
    let (merge, merge_verdict) = component_review(&index, &[middle.id.cast(), larger.id.cast()])?;
    let (transfer, transfer_verdict) =
        component_review(&index, &[smaller.id.cast(), middle.id.cast()])?;
    Ok((rows, [merge, transfer], [merge_verdict, transfer_verdict]))
}

fn index(rows: &[CanonicalAthlete; 3]) -> TestResult<AthleteIdentityIndex> {
    let mut index = AthleteIdentityIndex::default();
    for row in rows {
        index.observe(row)?;
    }
    Ok(index)
}

fn decisions(
    rows: &[CanonicalAthlete; 3],
    cases: &[ReviewCase; 2],
    verdicts: &[ReviewVerdictRecord; 2],
) -> TestResult<Vec<AppliedAthleteIdentity>> {
    let builder = IdentityProjectionBuilder::new(index(rows)?, cases, verdicts)?;
    let mut records = Vec::new();
    for case in cases {
        let (_, application) = builder
            .reviewed_applications("2026-10-12")
            .find(|(review, _)| review.id == case.id)
            .ok_or("missing reviewed application")?;
        match application? {
            IdentityApplication::Accepted(application) => {
                records.push(application.record().clone())
            }
            IdentityApplication::Retained(issue) => {
                return Err(format!("retained {issue:?}").into())
            }
        }
    }
    Ok(records)
}

#[test]
fn a_smaller_transfer_journals_the_previous_canonical_owner() -> TestResult {
    let (rows, cases, verdicts) = context()?;
    let records = decisions(&rows, &cases, &verdicts)?;
    let [smaller, middle, larger] = &rows;
    let mut builder = IdentityProjectionBuilder::new(index(&rows)?, &cases, &verdicts)?;
    for record in &records {
        check!(eq; builder.consider(record)?, None);
    }
    let projection = builder.finish()?;
    check!(eq; projection.alias_transitions().len(), 2);
    let transition = projection
        .alias_transitions()
        .last()
        .ok_or("missing transfer transition")?;
    let transfer = records.last().ok_or("missing transfer record")?;
    check!(eq; transition.decision_id(), transfer.id.as_str());
    check!(eq; transition.subject(), middle.id.as_str());
    check!(eq; transition.prior_canonical_id(), middle.id.as_str());
    check!(eq; transition.canonical_id(), smaller.id.as_str());
    for row in [smaller, middle, larger] {
        check!(eq; projection.canonical_id(row.id.as_str()), smaller.id.as_str());
    }
    Ok(())
}

#[test]
fn reversing_a_transfer_restores_the_prior_root_without_mutating_records() -> TestResult {
    let (rows, cases, verdicts) = context()?;
    let records = decisions(&rows, &cases, &verdicts)?;
    let original = records.clone();
    let [smaller, middle, larger] = &rows;
    let transfer = records.last().ok_or("missing transfer record")?;
    let projection = AthleteIdentityProjection::replay_without(
        index(&rows)?,
        &cases,
        &verdicts,
        &records,
        &transfer.id,
    )?;
    check!(eq; projection.canonical_id(smaller.id.as_str()), smaller.id.as_str());
    check!(eq; projection.canonical_id(middle.id.as_str()), middle.id.as_str());
    check!(eq; projection.canonical_id(larger.id.as_str()), middle.id.as_str());
    check!(eq; projection.alias_transitions().len(), 1);
    check!(eq; records, original);
    Ok(())
}

#[test]
fn reversing_an_unknown_decision_is_refused() -> TestResult {
    let (rows, cases, verdicts) = context()?;
    let records = decisions(&rows, &cases, &verdicts)?;
    let result = AthleteIdentityProjection::replay_without(
        index(&rows)?,
        &cases,
        &verdicts,
        &records,
        "unknown-decision",
    );
    check!(matches!(result, Err(IdentityError::UnknownDecision(id)) if id == "unknown-decision"));
    Ok(())
}

fn cross_source_transfer() -> TestResult<[CanonicalAthlete; 3]> {
    let mut rows = component_chain(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9"))?;
    let [_first, original, transferred] = &mut rows;
    transferred.identity_attestations = original.identity_attestations.clone();
    Ok(rows)
}

#[test]
fn a_same_provider_transfer_keeps_an_independent_cross_source_join_verified() -> TestResult {
    let rows = cross_source_transfer()?;
    let index = index(&rows)?;
    let ids: Vec<_> = rows.iter().map(|row| row.id.cast()).collect();
    check!(index.supports_identity(AppliedIdentityKind::SamePerson, &ids));
    let (case, verdict) = component_review(&index, &ids)?;
    let cases = [case];
    let verdicts = [verdict];
    let mut builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let (_, application) = builder
        .reviewed_applications("2026-10-12")
        .next()
        .ok_or("missing reviewed transfer")?;
    let IdentityApplication::Accepted(application) = application? else {
        return Err("independent cross-source transfer was retained".into());
    };
    check!(eq; builder.consider(application.record())?, None);
    let projection = builder.finish()?;
    let root = rows
        .iter()
        .map(|row| row.id.as_str())
        .min()
        .ok_or("missing root")?;
    for row in &rows {
        check!(eq; projection.status(row.id.as_str())?, IdentityStatus::Verified);
        check!(eq; projection.canonical_id(row.id.as_str()), root);
    }
    Ok(())
}

#[test]
fn a_transfer_cannot_make_nonindependent_cross_source_claims_admissible() -> TestResult {
    let mut rows = cross_source_transfer()?;
    let [first, original, _transferred] = &mut rows;
    first.identity_attestations = original.identity_attestations.clone();
    let index = index(&rows)?;
    let ids: Vec<_> = rows.iter().map(|row| row.id.cast()).collect();
    check!(eq; index.supports_identity(AppliedIdentityKind::SamePerson, &ids), false);
    let (case, verdict) = component_review(&index, &ids)?;
    let cases = [case];
    let verdicts = [verdict];
    let builder = IdentityProjectionBuilder::new(index, &cases, &verdicts)?;
    let (_, application) = builder
        .reviewed_applications("2026-10-12")
        .next()
        .ok_or("missing reviewed transfer")?;
    check!(matches!(
        application?,
        IdentityApplication::Retained(IdentityDecisionIssue::MissingPositiveIdentityEvidence)
    ));
    check!(eq; builder.finish()?.alias_transitions().len(), 0);
    Ok(())
}
