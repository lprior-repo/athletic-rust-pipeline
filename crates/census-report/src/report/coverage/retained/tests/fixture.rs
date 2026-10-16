use crate::export::public_identity::tests::fixture::TestResult;
use crate::export::{
    CurrentAffiliationResolution, ExportDataset, VerifiedCurrentAffiliations,
    VerifiedParticipationView,
};
use census_domain::model::{CanonicalSchool, ProfileDate, ProfileId};
use census_domain::participation_candidates::participation_policy_v2;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::collections::BTreeMap;

pub(super) fn selected_dataset(store: &Store) -> TestResult<ExportDataset> {
    for athlete in store.snapshot().athletes()? {
        let (mut school, _) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            &format!("Synthetic coverage school {}", athlete.id),
            athlete.school.as_str(),
            None,
        );
        school.id = athlete.school.clone();
        school.evidence = athlete.evidence.clone();
        store.append(Table::Schools, &school)?;
    }
    let mut dataset = ExportDataset::load_public_identity_fixture(store)?;
    let profile = ProfileId::parse(&"a".repeat(64))?;
    let cutoff = ProfileDate::parse("2026-10-14")?;
    let participation_rule = participation_policy_v2()?;
    dataset.bind_selection_for_test(profile.clone(), cutoff.clone())?;
    dataset.bind_participation_rule_for_test(participation_rule.clone())?;
    dataset.bind_public_identity_fixture_from_store(store)?;
    let affiliations = VerifiedCurrentAffiliations::fixture(
        dataset.lineage.input_generation.clone(),
        profile.clone(),
        cutoff,
        dataset
            .athletes
            .iter()
            .map(|athlete| {
                (
                    athlete.id.clone(),
                    CurrentAffiliationResolution::Resolved {
                        school: athlete.school.clone(),
                        evidence_refs: vec![format!("synthetic-affiliation-{}", athlete.id)],
                    },
                )
            })
            .collect(),
    );
    let participation = VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        profile,
        participation_rule,
        BTreeMap::new(),
    );
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(dataset)
}

pub(super) fn unresolved_dataset(store: &Store) -> TestResult<ExportDataset> {
    let mut dataset = selected_dataset(store)?;
    let profile = dataset
        .selected_profile_id()
        .ok_or("selected profile")?
        .clone();
    let cutoff = dataset.selected_as_of().ok_or("selected cutoff")?.clone();
    let affiliations = VerifiedCurrentAffiliations::fixture(
        dataset.lineage.input_generation.clone(),
        profile.clone(),
        cutoff,
        BTreeMap::new(),
    );
    let participation = VerifiedParticipationView::fixture(
        dataset.lineage.input_generation.clone(),
        profile,
        participation_policy_v2()?,
        BTreeMap::new(),
    );
    dataset.bind_verified_views_for_test(participation, affiliations)?;
    Ok(dataset)
}
