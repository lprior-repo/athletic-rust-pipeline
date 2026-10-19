use super::super::TestResult;
use crate::export::ExportDataset;
use census_domain::model::{CanonicalPerformance, SourceNamespace};

pub(super) fn require_historical_school(
    dataset: &ExportDataset,
    performance: &CanonicalPerformance,
) -> TestResult {
    let team = dataset
        .teams
        .get(&performance.team)
        .ok_or("genuine team projection")?;
    let school = dataset
        .schools
        .get(&team.school)
        .ok_or("genuine school projection")?;
    let identity = school
        .source_identities
        .iter()
        .find(|identity| identity.namespace == SourceNamespace::MilesplitSchool)
        .ok_or("genuine Milesplit school identity")?;
    check!(eq; identity.id, "12439");
    let michigan = census_domain::UsJurisdiction::parse("MI").ok_or("Michigan jurisdiction")?;
    check!(eq; school.state, Some(michigan));
    check!(school.name.contains("Catholic"));
    check!(school.name.contains("Central"));
    let view = dataset.verified_current_affiliations()?;
    check!(matches!(
        view.resolution(&performance.athlete),
        crate::export::CurrentAffiliationResolution::Unknown
    ));
    Ok(())
}
