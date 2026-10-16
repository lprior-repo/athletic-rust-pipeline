use super::super::historical_fixture;
use super::{options, seed, Store, TestResult};

#[test]
fn obsolete_policy20_v3_reconstruction_refuses_before_publication_effects() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Historical V3 High")?;
    let options = options()?;
    let retained = temporary.path().join("reconstructed-policy20-v3");
    let error = historical_fixture::render_policy20(&store, &retained, &options, 3)
        .err()
        .ok_or("obsolete policy-20 v3 reconstruction was accepted")?;
    check!(error
        .to_string()
        .contains("incompatible with selection policy 20"));
    check!(!retained.join("manifest.json").exists());
    check!(!retained.join("workbook.xlsx").exists());
    let current = crate::workbook::build(&store, &options)?;
    super::super::verify_published(&current)?;
    Ok(())
}

#[test]
fn obsolete_policy20_receipt_installation_refuses_before_fence() -> TestResult {
    for revision in [3, 4, 5] {
        let temporary = tempfile::tempdir()?;
        let store = Store::open(temporary.path().join("store"))?;
        seed(&store, "Artifact Cutover High")?;
        let options = options()?;
        let staging = temporary.path().join("reconstructed-policy20-cutover");
        let error = historical_fixture::render_policy20(&store, &staging, &options, revision)
            .err()
            .ok_or("obsolete policy-20 receipt installation was accepted")?;
        check!(error
            .to_string()
            .contains("incompatible with selection policy 20"));
        check!(!staging.join("manifest.json").exists());
    }
    Ok(())
}

#[test]
fn obsolete_policy20_substitution_refuses_before_fence_reuse() -> TestResult {
    let temporary = tempfile::tempdir()?;
    let store = Store::open(temporary.path().join("store"))?;
    seed(&store, "Artifact Revision Tamper High")?;
    let options = options()?;
    let current = crate::workbook::build(&store, &options)?;
    let staging = temporary.path().join("substituted-revision");
    let error = historical_fixture::render_policy20(&store, &staging, &options, 3)
        .err()
        .ok_or("obsolete policy-20 substitution was accepted")?;
    check!(error
        .to_string()
        .contains("incompatible with selection policy 20"));
    check!(!staging.join("manifest.json").exists());
    super::super::verify_published(&current)?;
    Ok(())
}
