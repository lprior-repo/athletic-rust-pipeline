use super::*;
use crate::export::ParticipationDiagnostic;

#[test]
fn unsupported_source_is_withheld_with_empty_reasons_and_retained_diagnostic() -> TestResult {
    let fixture = Fixture::new()?;
    let view = diagnostic_view(&fixture, ParticipationOutcome::Unresolved)?;
    let outcome = fixture.outcome(&view, Some(0))?;
    let directory = tempfile::tempdir()?;
    check!(eq; write(directory.path(), &fixture.dataset, &view, &outcome)?, 3);
    check!(eq; verify(directory.path(), &fixture.dataset, &view, &outcome)?, 3);
    let rows = readback::read(directory.path())?;
    check!(eq; rows.first().and_then(|row| row.first()).ok_or("candidate header")?, "retained_candidate_id");
    let row = rows
        .iter()
        .find(|row| {
            row.get(5)
                .is_some_and(|cell| cell == "[\"unsupported_provider\"]")
        })
        .ok_or("unsupported source row")?;
    check!(eq; row.get(3).ok_or("disposition")?, "unresolved");
    check!(eq; row.get(4).ok_or("reasons")?, "[]");
    check!(eq; row.get(6).ok_or("evidence")?, "[\"unsupported-source-proof\"]");
    Ok(())
}

#[test]
fn eligible_assessment_with_diagnostics_is_refused_by_selection_and_audit() -> TestResult {
    let fixture = Fixture::new()?;
    let unresolved = diagnostic_view(&fixture, ParticipationOutcome::Unresolved)?;
    let outcome = fixture.outcome(&unresolved, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &unresolved, &outcome)?;
    let eligible = diagnostic_view(&fixture, ParticipationOutcome::Eligible)?;
    check!(fixture.outcome(&eligible, None).is_err());
    check!(write(directory.path(), &fixture.dataset, &eligible, &outcome).is_err());
    check!(verify(directory.path(), &fixture.dataset, &eligible, &outcome).is_err());
    Ok(())
}

#[test]
fn diagnostic_readback_rejects_removed_replaced_or_fabricated_diagnostics() -> TestResult {
    let fixture = Fixture::new()?;
    let view = diagnostic_view(&fixture, ParticipationOutcome::Unresolved)?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = readback::read(directory.path())?;
    [
        "[]",
        "[\"source_unavailable\"]",
        "[\"invented_diagnostic\"]",
    ]
    .into_iter()
    .try_for_each(|replacement| -> TestResult {
        let mut rows = original.clone();
        let row = rows
            .iter_mut()
            .find(|row| {
                row.get(5)
                    .is_some_and(|cell| cell == "[\"unsupported_provider\"]")
            })
            .ok_or("unsupported source row")?;
        *row.get_mut(5).ok_or("diagnostic cell")? = replacement.into();
        readback::save(directory.path(), &rows)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
        Ok(())
    })
}

fn diagnostic_view(
    fixture: &Fixture,
    outcome: ParticipationOutcome,
) -> TestResult<VerifiedParticipationView> {
    let view = fixture.view()?;
    let performance = fixture
        .dataset
        .performances
        .iter()
        .find(|performance| performance.source_key == "unresolved")
        .ok_or("unresolved performance")?;
    let mut assessments = fixture.assessments()?;
    assessments.insert(
        performance.id.clone(),
        ParticipationAssessment::fixture_with_diagnostics(
            outcome,
            Default::default(),
            vec!["unsupported-source-proof".into()],
            vec![ParticipationDiagnostic::UnsupportedProvider],
        ),
    );
    Ok(VerifiedParticipationView::fixture(
        view.input_generation().into(),
        view.profile_id().clone(),
        view.participation_rule().clone(),
        assessments,
    ))
}
