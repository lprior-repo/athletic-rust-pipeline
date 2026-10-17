use super::*;

pub(in super::super) fn observations() -> TestResult<(TieFixture, Observation, Observation)> {
    let fixture = TieFixture::new()?;
    let case = numeric_cases()?.into_iter().next().ok_or("numeric case")?;
    let slower = fixture.report(&case, "eligible-school-result", "2026-05-01")?;
    let faster = fixture
        .report(&case, "participation-candidate", "2026-05-02")?
        .with_mark(case.better)?;
    Ok((fixture, slower, faster))
}

pub(in super::super) fn selected(
    dataset: &ExportDataset,
    view: &VerifiedParticipationView,
    limit: Option<usize>,
) -> TestResult<crate::bests::SelectionOutcome> {
    Ok(crate::bests::reduce::build_outcome(
        &crate::report::EventAdmission::of(dataset)?,
        &Options {
            scope: Scope::AllSources,
            limit,
            ..Options::default()
        },
        ParticipationSelection::Selected(view),
    )?)
}

pub(in super::super) fn reasons(
    row: &crate::bests::ParticipationExclusion,
) -> Vec<ParticipationReason> {
    match row.reason() {
        ParticipationExclusionReason::Assessment(reasons)
        | ParticipationExclusionReason::DocumentarySchoolBindingUnresolved { reasons, .. } => {
            reasons.iter().collect()
        }
        ParticipationExclusionReason::MissingParticipationEvidence => Vec::new(),
    }
}
