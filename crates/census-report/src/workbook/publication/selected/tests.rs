use super::*;
use crate::workbook::publication::{binding as recipes, inventory};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_revision_adds_exactly_two_normalized_families_and_requires_exact_counts() -> TestResult
{
    let previous = inventory::names(6)?;
    let selected = inventory::names(7)?;
    check!(eq; previous.len(), 9);
    check!(eq; selected.len(), 11);
    check!(!previous.contains(&"participation_exclusions.csv"));
    check!(selected.contains(&"participation_exclusions.csv"));
    check!(!previous.contains(&"current_affiliations.csv"));
    check!(selected.contains(&"current_affiliations.csv"));
    check!(eq; selected.iter().copied().filter(|name| *name != "participation_exclusions.csv" && *name != "current_affiliations.csv").collect::<Vec<_>>(), previous);
    let counts: inventory::RowCounts = selected.iter().map(|name| ((*name).into(), 0)).collect();
    inventory::validate(7, Some(&counts))?;
    inventory::compare(7, Some(&counts), &counts)?;
    check!(inventory::validate(6, Some(&counts)).is_err());
    check!(inventory::validate(7, None).is_err());
    let mut changed = counts.clone();
    changed.remove("participation_exclusions.csv");
    check!(inventory::validate(7, Some(&changed)).is_err());
    changed.insert("participation_exclusions.csv".into(), 1);
    check!(inventory::compare(7, Some(&counts), &changed).is_err());
    changed.insert("foreign.csv".into(), 0);
    check!(inventory::validate(7, Some(&changed)).is_err());
    Ok(())
}

#[test]
fn historical_selected_recipe_binds_profile_cutoff_and_participation_rule_without_changing_policy(
) -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let dataset = &fixture.dataset;
    let profile = SelectedProfile::of(dataset)?;
    let selected = recipes::recipe_for_selected(&dataset.lineage, &fixture.options, &profile)?;
    check!(eq; dataset.lineage.policy_revision, 21);
    check!(eq; census_domain::model::EventSemantics::CURRENT.revision(), 21);
    check!(revision(dataset).is_err());
    check!(recipes::recipe_for_dataset(dataset, &fixture.options).is_err());
    check!(eq; super::super::ARTIFACT_REVISION, 6);
    check!(recipes::recipe_for_artifact(&dataset.lineage, &fixture.options, 7).is_err());
    check!(ne; selected, recipes::recipe(&dataset.lineage, &fixture.options)?);
    for changed in [
        SelectedProfile {
            profile_id: ProfileId::parse(&"c".repeat(64))?,
            ..profile.clone()
        },
        SelectedProfile {
            as_of: ProfileDate::parse("2026-10-02")?,
            ..profile.clone()
        },
        SelectedProfile {
            participation_rule: ContractRefV1::new(1, 1, &"d".repeat(64))?,
            ..profile.clone()
        },
    ] {
        check!(ne; selected, recipes::recipe_for_selected(&dataset.lineage, &fixture.options, &changed)?);
    }
    Ok(())
}

#[test]
fn selected_revision_requires_runtime_verified_binding_and_never_downgrades_to_six() -> TestResult {
    let mut fixture = fixture::Fixture::new()?;
    let directory = tempfile::tempdir()?;
    let frozen = directory.path().join("frozen.json");
    fixture.dataset.save_frozen(&frozen)?;
    let reopened = ExportDataset::reopen_frozen(&frozen)?;
    check!(reopened.selected_profile_id().is_some());
    check!(revision(&reopened).is_err());
    check!(binding(6, &fixture.dataset).is_err());
    fixture.dataset.bind_historical_policy_for_test(21)?;
    check!(eq; revision(&fixture.dataset)?, 6);
    check!(binding(7, &fixture.dataset).is_err());
    check!(validate(7, None).is_err());
    Ok(())
}

#[test]
fn selected_fixture_uses_retained_store_issued_public_identity_for_each_candidate() -> TestResult {
    let mut fixture = fixture::Fixture::new()?;
    let athlete = fixture
        .dataset
        .athletes
        .first()
        .ok_or("synthetic selected athlete")?;
    let candidate = athlete.id.clone();
    let view = fixture.dataset.verified_public_identities()?;
    let public_id = view
        .public_id(candidate.as_str())
        .ok_or("retained public identity")?
        .clone();
    check!(ne; public_id.as_str(), candidate.as_str());
    check!(eq; view.snapshot_sequence(), fixture.dataset.lineage.snapshot_sequence);
    check!(eq; view.store_identity().as_str(), fixture.dataset.lineage.store_identity);
    check!(eq; view.head().sequence, 1);
    check!(eq; view.candidate_resolution(candidate.as_str()), Some(&view.resolve_public(&public_id)?));
    let generation = fixture.dataset.lineage.input_generation.clone();
    fixture.bind('c', "2026-10-02", 'd')?;
    let rebound = fixture.dataset.verified_public_identities()?;
    check!(ne; rebound.input_generation(), generation);
    check!(eq; rebound.input_generation(), fixture.dataset.lineage.input_generation);
    check!(eq; rebound.public_id(candidate.as_str()), Some(&public_id));
    Ok(())
}
