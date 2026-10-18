use super::super::source::TestResult;
use super::Expected;
use crate::export::ExportDataset;
use census_domain::model::CanonicalPerformance;
pub(super) fn matches(
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    identity_matches(dataset, wanted, expected)?;
    schools_match(dataset, wanted, expected)
}
fn identity_matches(
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let athlete = dataset
        .athletes
        .iter()
        .find(|row| row.id == wanted.athlete)
        .ok_or("controlled performance-time candidate")?;
    check!(eq; athlete.school, *expected.historical_school);
    let view = dataset.verified_public_identities()?;
    check!(eq; view.public_id(wanted.athlete.as_str()), Some(expected.public));
    let group = view
        .group(wanted.athlete.as_str())
        .ok_or("controlled identity group")?;
    check!(eq; group.canonical_candidate().as_str(), wanted.athlete.as_str());
    check!(eq; group.public_id(), Some(expected.public));
    Ok(())
}
fn schools_match(
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let team = dataset.teams.get(&wanted.team).ok_or("controlled team")?;
    check!(eq; team.school, *expected.historical_school);
    let historical = dataset
        .schools
        .get(expected.historical_school)
        .ok_or("historical school")?;
    let current = dataset
        .schools
        .get(expected.current_school)
        .ok_or("current school")?;
    check!(historical.id != current.id);
    let affiliations = dataset.verified_current_affiliations()?;
    let resolution = affiliations.resolution(&wanted.athlete);
    check!(eq; resolution.school(), Some(expected.current_school),
        "current affiliation: {resolution:?}; audit={:?}", affiliations.audit(&wanted.athlete));
    check!(eq; resolution.status(), "resolved_at_selected_cutoff");
    let schools = crate::export::current_school::SelectedSchools::of(dataset)?;
    let fields = schools.fields(wanted.athlete.as_str())?;
    check!(eq; fields.current_school_id, expected.current_school.as_str());
    check!(eq; fields.current_school, current.name.as_str());
    Ok(())
}
