use super::*;
use crate::workbook::publication::{binding as recipes, inventory};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn selected_artifact_eight_has_distinct_recipe_and_preserves_exact_seven_recipe() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    let selected = SelectedProfile::of(&fixture.dataset)?;
    let lineage = &fixture.dataset.lineage;
    let options = &fixture.options;
    let jurisdictions = vec!["MI"];
    let original_selection = recipes::digest(&(
        options.scope.as_str(),
        options.grad_year,
        options.limit,
        options.school_year,
        jurisdictions,
        lineage.schema_revision,
        lineage.policy_revision,
    ))?;
    let original = recipes::digest(&(
        "athletic-publication-artifact-recipe-v1",
        7_u32,
        original_selection,
        &selected,
    ))?;
    check!(eq; recipes::recipe_for_selected(lineage, options, &selected)?, original);
    check!(eq; recipes::recipe_for_selected_revision(lineage, options, &selected, 7)?, original);
    check!(ne; recipes::recipe_for_selected_revision(lineage, options, &selected, 8)?, original);
    check!(ne; recipes::recipe_for_selected_revision(lineage, options, &selected, 10)?, original);
    check!(recipes::recipe_for_selected_revision(lineage, options, &selected, 6).is_err());
    check!(recipes::recipe_for_selected_revision(lineage, options, &selected, 9).is_err());
    check!(recipes::recipe_for_selected_revision(lineage, options, &selected, 11).is_err());
    check!(eq; inventory::names(7)?, inventory::names(8)?);
    Ok(())
}

#[test]
fn captured_authority_binds_current_artifact_eight() -> TestResult {
    let fixture = fixture::Fixture::new()?;
    check!(binding(8, &fixture.dataset)?.is_some());
    ensure_current_options(&fixture.dataset, &fixture.options)?;
    check!(recipes::recipe_for_dataset(&fixture.dataset, &fixture.options).is_ok());
    Ok(())
}
