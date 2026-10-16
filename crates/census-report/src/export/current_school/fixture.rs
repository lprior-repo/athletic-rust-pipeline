use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use census_domain::model::{CanonicalSchool, ContractRefV1, ProfileDate, ProfileId};
use census_store::Store;
use std::collections::BTreeMap;

type TestResult<T> = Result<T, Box<dyn std::error::Error>>;

pub(crate) fn capture(store: &Store) -> TestResult<(ExportDataset, ())> {
    let (school, _) = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Synthetic Current Wisconsin High",
        "synthetic current wisconsin high",
        None,
    );
    capture_with_school(store, school)
}

pub(crate) fn capture_with_school(
    store: &Store,
    school: CanonicalSchool,
) -> TestResult<(ExportDataset, ())> {
    let mut dataset = ExportDataset::load_public_identity_fixture(store)?;
    let id = school.id.clone();
    dataset.schools.insert(id.clone(), school);
    let profile = ProfileId::parse(&"a".repeat(64))?;
    let cutoff = ProfileDate::parse("2026-10-01")?;
    let rule = ContractRefV1::new(1, 1, &"b".repeat(64))?;
    dataset.bind_selection_for_test(profile.clone(), cutoff.clone())?;
    dataset.bind_participation_rule_for_test(rule.clone())?;
    let generation = dataset.lineage.input_generation.clone();
    let affiliations = VerifiedCurrentAffiliations::fixture(
        generation.clone(),
        profile.clone(),
        cutoff,
        dataset
            .athletes
            .iter()
            .map(|athlete| {
                (
                    athlete.id.clone(),
                    CurrentAffiliationResolution::Resolved {
                        school: id.clone(),
                        evidence_refs: vec!["synthetic-current-affiliation-proof".into()],
                    },
                )
            })
            .collect(),
    );
    let participation =
        VerifiedParticipationView::fixture(generation, profile, rule, BTreeMap::new());
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    dataset.bind_public_identity_fixture_from_store(store)?;
    Ok((dataset, ()))
}

pub(crate) fn bind_resolution(
    dataset: &mut ExportDataset,
    resolution: CurrentAffiliationResolution,
) -> TestResult<()> {
    let profile = dataset.selected_profile_id().ok_or("profile")?.clone();
    let cutoff = dataset.selected_as_of().ok_or("cutoff")?.clone();
    let rule = dataset.selected_participation_rule().ok_or("rule")?.clone();
    let generation = dataset.lineage.input_generation.clone();
    let affiliations = VerifiedCurrentAffiliations::fixture(
        generation.clone(),
        profile.clone(),
        cutoff,
        dataset
            .athletes
            .iter()
            .map(|athlete| (athlete.id.clone(), resolution.clone()))
            .collect(),
    );
    let participation =
        VerifiedParticipationView::fixture(generation, profile, rule, BTreeMap::new());
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(())
}
