use super::source::TestResult;
use crate::export::ExportDataset;
use census_domain::model::SchoolId;
use census_domain::participation_candidates::ParticipationReason;
use census_domain::public_identity_history::PublicAthleteId;
use std::path::Path;
mod artifacts;
mod exclusions;
mod identity;
mod normalized;
mod workbook;
pub(super) struct Expected<'a> {
    pub(super) seconds: &'a str,
    pub(super) day: &'a str,
    pub(super) public: &'a PublicAthleteId,
    pub(super) current_school: &'a SchoolId,
    pub(super) historical_school: &'a SchoolId,
}
pub(super) fn assert_generation(
    dataset: &ExportDataset,
    workbook: &Path,
    expected: Expected<'_>,
) -> TestResult<()> {
    let directory = artifacts::directory_of(workbook)?;
    replay_matches(&directory, dataset)?;
    let feb = artifacts::by_mark(dataset, "49.00", "2026-02-28")?;
    let may50 = artifacts::by_mark(dataset, "50.00", "2026-05-15")?;
    let may51 = artifacts::by_mark(dataset, "51.00", "2026-05-16")?;
    check!(eq; dataset.performances.len(), 3);
    check!(may50.id != may51.id);
    check!(feb.id != may50.id);
    check!(feb.id != may51.id);
    let wanted = artifacts::by_mark(dataset, expected.seconds, expected.day)?;
    check!(wanted.id == may50.id || wanted.id == may51.id);
    check!(expected.seconds == "50.00" || expected.seconds == "51.00");
    check!(expected.day == "2026-05-15" || expected.day == "2026-05-16");
    check!(expected.current_school != expected.historical_school);
    identity::matches(dataset, &wanted, &expected)?;
    participation_matches(dataset, &feb, &may50, &may51, &wanted)?;
    let outcome = artifacts::outcome_of(dataset)?;
    selection_matches(dataset, &outcome, &wanted, &expected)?;
    normalized::matches(dataset, &directory, &outcome, &wanted, &expected)?;
    workbook::matches(dataset, workbook, &wanted, &expected)?;
    Ok(())
}
fn replay_matches(directory: &Path, dataset: &ExportDataset) -> TestResult<()> {
    let replayed = artifacts::reopened(directory)?;
    check!(eq; replayed.performances, dataset.performances);
    check!(eq; replayed.athletes, dataset.athletes);
    check!(eq; replayed.lineage.input_generation, dataset.lineage.input_generation);
    check!(eq; replayed.lineage.store_identity, dataset.lineage.store_identity);
    Ok(())
}
fn participation_matches(
    dataset: &ExportDataset,
    feb: &census_domain::model::CanonicalPerformance,
    may50: &census_domain::model::CanonicalPerformance,
    may51: &census_domain::model::CanonicalPerformance,
    wanted: &census_domain::model::CanonicalPerformance,
) -> TestResult<()> {
    let view = dataset.verified_participation()?;
    let feb_assessment = view
        .assessment(&feb.id)
        .ok_or("controlled Feb28 assessment")?;
    check!(eq; feb_assessment.outcome(), crate::export::ParticipationOutcome::Excluded);
    check!(feb_assessment
        .reasons()
        .contains(ParticipationReason::OutsideParticipationWindow));
    check!(!feb_assessment
        .reasons()
        .contains(ParticipationReason::SummerBoundaryUnresolved));
    check!(!feb_assessment
        .reasons()
        .contains(ParticipationReason::HighSchoolStatusMissing));
    check!(feb_assessment.diagnostics().is_empty());
    check!(!feb_assessment.evidence_refs().is_empty());
    let wanted_assessment = view
        .assessment(&wanted.id)
        .ok_or("controlled wanted assessment")?;
    check!(eq; wanted_assessment.outcome(), crate::export::ParticipationOutcome::Eligible);
    check!(wanted_assessment.reasons().is_empty());
    check!(wanted_assessment.diagnostics().is_empty());
    check!(!wanted_assessment.evidence_refs().is_empty());
    if wanted.id == may50.id {
        assert_may51_eligible(&view, may51)?;
        assert_may50_eligible(&view, may50)?;
    } else {
        assert_may50_revoked(&view, may50)?;
        assert_may51_eligible(&view, may51)?;
    }
    Ok(())
}
fn assert_may50_eligible(
    view: &crate::export::VerifiedParticipationView,
    may50: &census_domain::model::CanonicalPerformance,
) -> TestResult<()> {
    let assessment = view
        .assessment(&may50.id)
        .ok_or("controlled May15 50.00 assessment")?;
    check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Eligible);
    check!(assessment.reasons().is_empty());
    Ok(())
}
fn assert_may51_eligible(
    view: &crate::export::VerifiedParticipationView,
    may51: &census_domain::model::CanonicalPerformance,
) -> TestResult<()> {
    let assessment = view
        .assessment(&may51.id)
        .ok_or("controlled May16 51.00 assessment")?;
    check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Eligible);
    check!(assessment.reasons().is_empty());
    Ok(())
}
fn assert_may50_revoked(
    view: &crate::export::VerifiedParticipationView,
    may50: &census_domain::model::CanonicalPerformance,
) -> TestResult<()> {
    let assessment = view
        .assessment(&may50.id)
        .ok_or("revoked May15 50.00 retained assessment")?;
    check!(eq; assessment.outcome(), crate::export::ParticipationOutcome::Unresolved);
    check!(assessment
        .reasons()
        .contains(ParticipationReason::HighSchoolStatusMissing));
    check!(assessment.diagnostics().is_empty());
    check!(!assessment.evidence_refs().is_empty());
    Ok(())
}
fn selection_matches(
    dataset: &ExportDataset,
    outcome: &crate::bests::SelectionOutcome,
    wanted: &census_domain::model::CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    check!(eq; outcome.selections.len(), 1);
    let row = outcome.selections.first().ok_or("controlled selection")?;
    check!(eq; row.source.performance_id, wanted.id);
    check!(eq; row.source.source_key, wanted.source_key);
    check!(eq; crate::bests::mark_text(&row.result.mark), expected.seconds);
    check!(eq; crate::bests::mark_text(&wanted.mark), expected.seconds);
    check!(eq; row.meet.date, expected.day);
    check!(eq; wanted.date, expected.day);
    check!(eq; row.athlete_id().as_str(), wanted.athlete.as_str());
    check!(eq; row.inventory.policy().revision(), 21);
    check!(eq; row.inventory.eligible_group_count(), if expected.seconds == "50.00" { 2 } else { 1 });
    check!(eq; row.inventory.tied_group_count(), 1);
    check!(eq; row.inventory.tied_representative_ids(), std::slice::from_ref(&wanted.id));
    let schools = crate::export::current_school::SelectedSchools::of(dataset)?;
    let fields = schools.fields(row.athlete_id().as_str())?;
    check!(eq; fields.current_school_id, expected.current_school.as_str());
    let identities = dataset.verified_public_identities()?;
    let group = identities
        .group(row.athlete_id().as_str())
        .ok_or("selected identity")?;
    check!(eq; group.public_id(), Some(expected.public));
    Ok(())
}
