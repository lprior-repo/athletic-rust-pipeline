use super::*;

#[test]
fn recruiting_policy_fifteen_has_a_distinct_recipe_from_all_prior_integration_policies(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let current = ExportDataset::load(&store)?;
    check!(eq; current.lineage.schema_revision, 3);
    check!(eq; current.lineage.policy_revision, 15);
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        jurisdictions: Vec::new(),
    };
    let current_recipe = binding::recipe(&current.lineage, &options)?;
    [8, 9, 10, 11, 12, 13, 14]
        .into_iter()
        .try_for_each(|policy| -> TestResult {
            let mut prior = current.lineage.clone();
            prior.policy_revision = policy;
            check!(ne; current_recipe, binding::recipe(&prior, &options)?);
            Ok(())
        })
}

#[test]
fn jurisdiction_order_and_duplicates_do_not_change_the_fence_recipe() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let current = ExportDataset::load(&store)?;
    let option_set = |jurisdictions: Vec<census_domain::UsJurisdiction>| -> TestResult<Options> {
        Ok(Options {
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
            jurisdictions,
        })
    };
    let ordered = binding::recipe(
        &current.lineage,
        &option_set(vec![
            census_domain::UsJurisdiction::Ohio,
            census_domain::UsJurisdiction::Wisconsin,
        ])?,
    )?;
    let reordered = binding::recipe(
        &current.lineage,
        &option_set(vec![
            census_domain::UsJurisdiction::Wisconsin,
            census_domain::UsJurisdiction::Ohio,
            census_domain::UsJurisdiction::Ohio,
        ])?,
    )?;
    check!(eq; ordered, reordered);
    let global = binding::recipe(&current.lineage, &option_set(Vec::new())?)?;
    if ordered == global {
        return Err("a restricted fence shares the global recipe".into());
    }
    Ok(())
}
