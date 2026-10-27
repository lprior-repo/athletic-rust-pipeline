use super::*;

#[test]
fn result_group_recipe_preserves_old_tuples_and_uses_a_distinct_current_identity() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let dataset = ExportDataset::load(&store)?;
    let options = Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("fixture season")?,
        jurisdictions: Vec::new(),
    };
    let mut historical = dataset.lineage.clone();
    historical.policy_revision = 20;
    let original = binding::digest(&(
        options.scope.as_str(),
        options.grad_year,
        options.limit,
        options.school_year,
        Vec::<&str>::new(),
        historical.schema_revision,
        20_u32,
    ))?;
    for revision in [1, 2, 3] {
        check!(eq; binding::recipe_for_artifact(&historical, &options, revision)?, original);
    }
    for revision in [4_u32, 5] {
        let expected = binding::digest(&(
            "athletic-publication-artifact-recipe-v1",
            revision,
            &original,
        ))?;
        check!(eq; binding::recipe_for_artifact(&historical, &options, revision)?, expected);
    }
    let current = binding::recipe(&dataset.lineage, &options)?;
    let current_base = binding::digest(&(
        options.scope.as_str(),
        options.grad_year,
        options.limit,
        options.school_year,
        Vec::<&str>::new(),
        dataset.lineage.schema_revision,
        dataset.lineage.policy_revision,
    ))?;
    let expected = binding::digest(&(
        "athletic-publication-artifact-recipe-v1",
        6_u32,
        current_base,
    ))?;
    check!(eq; current, expected);
    check!(ne; current, original);
    for revision in 1..=5 {
        check!(binding::recipe_for_artifact(&dataset.lineage, &options, revision).is_err());
    }
    check!(binding::recipe_for_artifact(&historical, &options, 6).is_err());
    check!(binding::recipe_for_artifact(&dataset.lineage, &options, 7).is_err());
    Ok(())
}

#[test]
fn current_recruiting_policy_has_a_distinct_recipe_from_all_prior_integration_policies(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let current = ExportDataset::load(&store)?;
    let options = Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        jurisdictions: Vec::new(),
    };
    let current_recipe = binding::recipe(&current.lineage, &options)?;
    [8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20]
        .into_iter()
        .try_for_each(|policy| -> TestResult {
            let mut prior = current.lineage.clone();
            prior.policy_revision = policy;
            check!(ne; current_recipe, binding::recipe_for_artifact(&prior, &options, 5)?);
            check!(binding::recipe(&prior, &options).is_err());
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
            address_corpus: None,
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

#[test]
fn implicit_and_explicit_national_selection_bind_the_same_seal_scope() -> TestResult {
    for (published, requested) in [
        (Vec::new(), UsJurisdiction::CENSUS_SCOPE.to_vec()),
        (UsJurisdiction::CENSUS_SCOPE.to_vec(), Vec::new()),
    ] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        seed(&store, "National Scope Fixture")?;
        let options = Options {
            address_corpus: None,
            grad_year: Some(2027),
            out: None,
            limit: None,
            scope: crate::report::Scope::AllSources,
            school_year: SchoolYear::new(2026).ok_or("fixture season")?,
            jurisdictions: published,
        };
        let path = crate::workbook::build(&store, &options)?;
        super::super::open_publication_for_seal(
            &path,
            &store,
            options.scope,
            GradYear::CO2027,
            &requested,
        )?;
        check!(super::super::open_publication_for_seal(
            &path,
            &store,
            options.scope,
            GradYear::CO2027,
            &[UsJurisdiction::Wisconsin]
        )
        .is_err());
    }
    Ok(())
}
