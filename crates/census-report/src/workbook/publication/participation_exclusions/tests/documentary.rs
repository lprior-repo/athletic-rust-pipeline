use super::*;
use crate::export::DocumentaryParticipationRow;
use census_domain::model::{Id, PerformanceId};
use census_domain::participation_candidates::{CandidateOutcome, ParticipationReasons};
use std::collections::BTreeMap;

mod guards;
mod readback;

#[test]
fn documentary_csv_keeps_eight_columns_and_actual_outcome_tags() -> TestResult {
    let fixture = Fixture::empty()?;
    let view = view(&fixture, rows()?)?;
    let outcome = outcome(&fixture, &view)?;
    let directory = tempfile::tempdir()?;
    check!(eq; write(directory.path(), &fixture.dataset, &view, &outcome)?, 3);
    check!(eq; verify(directory.path(), &fixture.dataset, &view, &outcome)?, 3);
    let csv = super::readback::read(directory.path())?;
    check!(eq; csv.first().ok_or("header")?, &HEADER.map(str::to_owned).to_vec());
    for record in csv.iter().skip(1) {
        check!(eq; record.len(), 8);
        let id = record.get(1).ok_or("performance ID")?;
        let row = view
            .documentary_rows()
            .find(|row| row.performance_id().as_str() == id)
            .ok_or("documentary source")?;
        let (disposition, tag) = match row.domain_outcome() {
            CandidateOutcome::EligibleCandidate => {
                ("unresolved", "documentary_assessment_eligible")
            }
            CandidateOutcome::Excluded => ("excluded", "documentary_assessment_excluded"),
            CandidateOutcome::Unresolved => ("unresolved", "documentary_assessment_unresolved"),
        };
        check!(eq; record.get(0).ok_or("candidate")?, row.athlete_id().as_str());
        check!(eq; record.get(2).ok_or("event")?, row.event_id().as_str());
        check!(eq; record.get(3).ok_or("disposition")?, disposition);
        check!(eq; record.get(4).ok_or("reasons")?, &serde_json::to_string(&row.assessment().reasons().iter().collect::<Vec<_>>())?);
        check!(eq; record.get(5).ok_or("diagnostics")?, &serde_json::to_string(&[tag, "school_binding_unresolved"])?);
        check!(eq; record.get(6).ok_or("evidence")?, &serde_json::to_string(row.assessment().evidence_refs())?);
        check!(eq; record.get(7).ok_or("generation")?, view.input_generation());
    }
    check!(fixture.dataset.performances.is_empty());
    check!(outcome.selections.is_empty());
    Ok(())
}

fn rows() -> TestResult<BTreeMap<PerformanceId, DocumentaryParticipationRow>> {
    let baseline = Fixture::new()?;
    let assessments = baseline.assessments()?;
    let excluded = assessments
        .values()
        .find(|row| row.outcome() == ParticipationOutcome::Excluded)
        .ok_or("excluded reasons")?
        .reasons();
    let unresolved = assessments
        .values()
        .find(|row| row.outcome() == ParticipationOutcome::Unresolved)
        .ok_or("unresolved reasons")?
        .reasons();
    Ok([
        (
            "eligible",
            CandidateOutcome::EligibleCandidate,
            ParticipationReasons::default(),
        ),
        ("excluded", CandidateOutcome::Excluded, excluded),
        ("unresolved", CandidateOutcome::Unresolved, unresolved),
    ]
    .into_iter()
    .map(|(label, actual, reasons)| {
        let id = Id::mint("perf", &["synthetic-documentary", label]);
        let row = DocumentaryParticipationRow::fixture(
            id.clone(),
            Id::mint("event", &[label]),
            Id::mint("athlete", &[label]),
            actual,
            reasons,
            vec![format!(
                "synthetic documentary, \"{label}\"\nsource reference"
            )],
        );
        (id, row)
    })
    .collect())
}

fn view(
    fixture: &Fixture,
    rows: BTreeMap<PerformanceId, DocumentaryParticipationRow>,
) -> TestResult<VerifiedParticipationView> {
    Ok(fixture.view()?.fixture_with_documentary(rows, Vec::new()))
}

fn outcome(fixture: &Fixture, view: &VerifiedParticipationView) -> TestResult<SelectionOutcome> {
    let profile = fixture.dataset.selected_authority()?.profile();
    let [cohort] = profile.input().graduation_years.as_slice() else {
        return Err("one selected cohort".into());
    };
    Ok(crate::bests::build_participation_outcome(
        &crate::report::EventAdmission::of(&fixture.dataset)?,
        &crate::bests::Options {
            scope: crate::report::Scope::AllSources,
            grad_year: Some(cohort.get()),
            jurisdictions: profile.input().jurisdictions.clone(),
            limit: None,
        },
        view,
    )?)
}
