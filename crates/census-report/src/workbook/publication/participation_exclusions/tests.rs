use super::*;
use crate::export::ParticipationAssessment;
use census_domain::model::{ContractRefV1, ProfileId};

mod diagnostics;
mod fixture;
mod readback;
use fixture::Fixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn participation_audit_retains_all_exclusions_at_zero_selected_row_limit() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let full = fixture.outcome(&view, None)?;
    let limited = fixture.outcome(&view, Some(0))?;
    check!(eq; limited.participation_exclusions, full.participation_exclusions);
    check!(eq; limited.participation_exclusions.len(), 3);
    let directory = tempfile::tempdir()?;
    check!(eq; write(directory.path(), &fixture.dataset, &view, &limited)?, 3);
    check!(eq; verify(directory.path(), &fixture.dataset, &view, &limited)?, 3);
    let rows = readback::read(directory.path())?;
    check!(eq; rows.first().ok_or("header")?, &HEADER.map(str::to_owned).to_vec());
    for row in rows.iter().skip(1) {
        check!(eq; row.get(7).ok_or("generation")?, view.input_generation());
        let id = row.get(1).ok_or("performance")?;
        let expected = limited
            .participation_exclusions
            .iter()
            .find(|value| value.performance_id().as_str() == id)
            .ok_or("exclusion")?;
        check!(eq; row.first().ok_or("athlete")?, expected.athlete_id().as_str());
        check!(eq; row.get(2).ok_or("event")?, expected.event_id().as_str());
        if expected.reason() == ParticipationExclusionReason::MissingParticipationEvidence {
            check!(eq; row.get(3).ok_or("disposition")?, "unresolved");
            check!(eq; row.get(4).ok_or("reasons")?, "[\"missing_participation_evidence\"]");
            check!(eq; row.get(5).ok_or("diagnostics")?, "[]");
            check!(eq; row.get(6).ok_or("proofs")?, "[]");
        } else {
            let assessment = view
                .assessment(expected.performance_id())
                .ok_or("assessment")?;
            check!(eq; row.get(5).ok_or("diagnostics")?, "[]");
            let proof_refs: Vec<String> = serde_json::from_str(row.get(6).ok_or("proofs")?)?;
            check!(eq; proof_refs, assessment.evidence_refs());
            let reasons: Vec<String> = serde_json::from_str(row.get(4).ok_or("reasons")?)?;
            check!(reasons
                .iter()
                .all(|value| value.chars().all(|c| c.is_ascii_lowercase() || c == '_')));
            check!(reasons
                .iter()
                .any(|value| value == "after_effective_cutoff"
                    || value == "missing_performance_date"));
        }
    }
    Ok(())
}

#[test]
fn participation_audit_header_only_is_valid_for_an_empty_outcome() -> TestResult {
    let fixture = Fixture::empty()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    check!(eq; write(directory.path(), &fixture.dataset, &view, &outcome)?, 0);
    check!(eq; verify(directory.path(), &fixture.dataset, &view, &outcome)?, 0);
    check!(eq; readback::read(directory.path())?.len(), 1);
    Ok(())
}

#[test]
fn participation_audit_refuses_oversized_proof_rows_before_encoding() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = std::fs::read(directory.path().join(FILE))?;
    let mut assessments = fixture.assessments()?;
    let id = assessments.keys().next().ok_or("assessment id")?.clone();
    let assessment = assessments.remove(&id).ok_or("assessment")?;
    assessments.insert(
        id,
        ParticipationAssessment::fixture(
            assessment.outcome(),
            assessment.reasons(),
            vec!["x".repeat(crate::grouping::ROW_BYTES)],
        ),
    );
    let oversized = VerifiedParticipationView::fixture(
        view.input_generation().into(),
        view.profile_id().clone(),
        view.participation_rule().clone(),
        assessments,
    );
    check!(matches!(
        write(directory.path(), &fixture.dataset, &oversized, &outcome),
        Err(ReportError::Grouping(
            crate::grouping::GroupingError::BudgetExceeded { .. }
        ))
    ));
    check!(matches!(
        verify(directory.path(), &fixture.dataset, &oversized, &outcome),
        Err(ReportError::Grouping(
            crate::grouping::GroupingError::BudgetExceeded { .. }
        ))
    ));
    check!(eq; std::fs::read(directory.path().join(FILE))?, original);
    Ok(())
}

#[test]
fn participation_audit_refuses_duplicate_or_unordered_expected_inventory() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let mut outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    outcome.participation_exclusions.reverse();
    check!(write(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    outcome.participation_exclusions.reverse();
    let duplicate = outcome
        .participation_exclusions
        .last()
        .ok_or("exclusion")?
        .clone();
    outcome.participation_exclusions.push(duplicate);
    check!(write(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    Ok(())
}

#[test]
fn participation_audit_refuses_foreign_generation_profile_or_rule() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    for (generation, profile, rule) in [
        (
            "foreign-generation".into(),
            view.profile_id().clone(),
            view.participation_rule().clone(),
        ),
        (
            view.input_generation().into(),
            ProfileId::parse(&"b".repeat(64))?,
            view.participation_rule().clone(),
        ),
        (
            view.input_generation().into(),
            view.profile_id().clone(),
            ContractRefV1::new(1, 2, &"c".repeat(64))?,
        ),
    ] {
        let foreign =
            VerifiedParticipationView::fixture(generation, profile, rule, fixture.assessments()?);
        check!(write(directory.path(), &fixture.dataset, &foreign, &outcome).is_err());
        check!(verify(directory.path(), &fixture.dataset, &foreign, &outcome).is_err());
    }
    Ok(())
}

#[test]
fn participation_audit_refuses_changed_assessment_or_missing_evidence() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let assessed = outcome
        .participation_exclusions
        .iter()
        .find(|value| matches!(value.reason(), ParticipationExclusionReason::Assessment(_)))
        .ok_or("assessed exclusion")?;
    let mut rows = fixture.assessments()?;
    rows.insert(
        assessed.performance_id().clone(),
        ParticipationAssessment::fixture(
            ParticipationOutcome::Eligible,
            Default::default(),
            Vec::new(),
        ),
    );
    let changed = VerifiedParticipationView::fixture(
        view.input_generation().into(),
        view.profile_id().clone(),
        view.participation_rule().clone(),
        rows,
    );
    check!(write(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
    check!(verify(directory.path(), &fixture.dataset, &changed, &outcome).is_err());
    let missing = VerifiedParticipationView::fixture(
        view.input_generation().into(),
        view.profile_id().clone(),
        view.participation_rule().clone(),
        Default::default(),
    );
    check!(write(directory.path(), &fixture.dataset, &missing, &outcome).is_err());
    check!(verify(directory.path(), &fixture.dataset, &missing, &outcome).is_err());
    Ok(())
}

mod documentary;
