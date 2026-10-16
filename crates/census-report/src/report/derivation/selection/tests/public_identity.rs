use crate::export::public_identity::population::{IdentityAdmission, IdentityPopulation};
use crate::export::public_identity::tests::fixture::{merge, source_births, TestResult};
use crate::export::ExportDataset;
use crate::report::{Derivation, Scope};

fn dataset(store: &census_store::Store) -> TestResult<ExportDataset> {
    let mut dataset = ExportDataset::load_public_identity_fixture(store)?;
    dataset.bind_selection_for_test(
        census_domain::model::ProfileId::parse(&"a".repeat(64))?,
        census_domain::model::ProfileDate::parse("2026-10-01")?,
    )?;
    dataset.bind_public_identity_fixture_from_store(store)?;
    Ok(dataset)
}

#[test]
fn selected_population_withholds_unknown_identity_without_removing_source_evidence() -> TestResult {
    let fixture = source_births()?;
    let dataset = dataset(&fixture.store)?;
    let [first, second, unknown] = &fixture.athletes;
    let population = Derivation::selected_population(&dataset, Scope::AllSources, None, &[])?;
    check!(eq; dataset.athletes.len(), 3);
    check!(eq; population.athletes().len(), 2);
    check!(population
        .athletes()
        .iter()
        .all(|row| row.id == first.id || row.id == second.id));
    let identities = IdentityPopulation::of(&dataset)?;
    check!(identities.admission(unknown.id.as_str()) == IdentityAdmission::Withheld);
    check!(identities.admission("absent") == IdentityAdmission::Absent);
    check!(eq; identities.canonical_candidate("absent"), None);
    check!(eq; dataset.verified_public_identities()?.identity_audit().count(), 3);
    Ok(())
}

#[test]
fn selected_population_merges_only_by_retained_public_history_and_keeps_original_aliases(
) -> TestResult {
    let fixture = source_births()?;
    let merged = merge(&fixture)?;
    let dataset = dataset(&fixture.store)?;
    let source = serde_json::to_vec(&dataset)?;
    let source_aliases = dataset.canonical_aliases.clone();
    let [first, second, unknown] = &fixture.athletes;
    let population = Derivation::selected_population(&dataset, Scope::AllSources, None, &[])?;
    let [athlete] = population.athletes() else {
        return Err("selected identity merge did not produce one row".into());
    };
    check!(eq; athlete.id, first.id);
    check!(eq; population.athlete_aliases().get(second.id.as_str()), Some(&first.id.to_string()));
    let identities = IdentityPopulation::of(&dataset)?;
    check!(identities.same_group(first.id.as_str(), second.id.as_str()));
    check!(!identities.same_group(first.id.as_str(), unknown.id.as_str()));
    check!(identities.admission(first.id.as_str()) == IdentityAdmission::Accepted);
    check!(eq; dataset.verified_public_identities()?.head(), merged.head());
    check!(eq; dataset.canonical_aliases, source_aliases);
    check!(eq; serde_json::to_vec(&dataset)?, source);
    Ok(())
}

#[test]
fn selected_population_cannot_fall_back_to_automatic_identity_without_public_authority(
) -> TestResult {
    let fixture = source_births()?;
    let mut dataset = ExportDataset::load_public_identity_fixture(&fixture.store)?;
    dataset.bind_selection_for_test(
        census_domain::model::ProfileId::parse(&"a".repeat(64))?,
        census_domain::model::ProfileDate::parse("2026-10-01")?,
    )?;
    check!(IdentityPopulation::of(&dataset).is_err());
    check!(Derivation::selected_population(&dataset, Scope::AllSources, None, &[]).is_err());
    Ok(())
}
