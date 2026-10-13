use super::*;

#[test]
fn combined_policy_twelve_has_a_distinct_recipe_from_all_independent_prior_policies() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let current = ExportDataset::load(&store)?;
    check!(eq; current.lineage.schema_revision, 2);
    check!(eq; current.lineage.policy_revision, 12);
    let options = Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    };
    let current_recipe = binding::recipe(&current.lineage, &options)?;
    [8, 9, 10, 11]
        .into_iter()
        .try_for_each(|policy| -> TestResult {
            let mut prior = current.lineage.clone();
            prior.policy_revision = policy;
            check!(ne; current_recipe, binding::recipe(&prior, &options)?);
            Ok(())
        })
}
