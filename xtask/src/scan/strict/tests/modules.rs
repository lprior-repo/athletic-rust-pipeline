use super::*;

#[test]
fn external_test_modules_are_classified_by_declarations_not_filename() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("lib.rs");
    let test = directory.path().join("helper.rs");
    let production = directory.path().join("production_tests.rs");
    let sources = vec![
        (root, "#[cfg(test)] #[path=\"helper.rs\"] mod tests; #[path=\"production_tests.rs\"] mod production;".to_string()),
        (test.clone(), function(26)), (production, function(26)),
    ];
    check!(eq; modules::test_files(&sources, &[])?, std::collections::BTreeSet::from([test]));
    directory.close()?;
    Ok(())
}

#[test]
fn shared_external_module_is_production_even_with_test_reference() -> TestResult {
    let root = PathBuf::from("/source/lib.rs");
    let shared = PathBuf::from("/source/shared.rs");
    let sources = vec![
        (
            root,
            "#[cfg(test)] #[path=\"shared.rs\"] mod tests; #[path=\"shared.rs\"] mod production;"
                .to_string(),
        ),
        (shared, function(26)),
    ];
    check!(eq; modules::test_files(&sources, &[])?, std::collections::BTreeSet::<PathBuf>::new());
    Ok(())
}

#[test]
fn declared_cargo_target_is_production_even_when_linked_only_by_test_module() -> TestResult {
    let root = PathBuf::from("/source/lib.rs");
    let binary = PathBuf::from("/source/binary.rs");
    let sources = vec![
        (
            root,
            "#[cfg(test)] #[path=\"binary.rs\"] mod tests;".to_string(),
        ),
        (binary.clone(), function(26)),
    ];
    check!(eq; modules::test_files(&sources, &[binary])?, std::collections::BTreeSet::<PathBuf>::new());
    Ok(())
}

#[test]
fn stringified_module_tokens_do_not_trigger_module_path_discovery() -> TestResult {
    let sources = vec![(
        PathBuf::from("/source/lib.rs"),
        "const S: &str = stringify!(mod missing;);".to_string(),
    )];
    check!(eq; modules::test_files(&sources, &[])?, std::collections::BTreeSet::<PathBuf>::new());
    Ok(())
}

#[test]
fn opaque_macro_module_tokens_remain_expansion_obligations_not_source_paths() -> TestResult {
    let source = "generate! { mod missing; }";
    let roots = [PathBuf::from("/source/lib.rs")];
    let root = roots.first().context("fixture root missing")?;
    check!(eq; modules::references(root, source, &roots)?, Vec::<PathBuf>::new());
    let found = inspect(source)?;
    check!(eq; found.unresolved.len(), 1);
    check!(enforce_report(&report(found)).is_err());
    Ok(())
}
