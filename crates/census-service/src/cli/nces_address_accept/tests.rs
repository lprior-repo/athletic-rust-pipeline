use super::*;
use census_domain::model::{CensusRun, GradYear, RunManifest, SchoolYear};
use census_domain::UsJurisdiction;
use clap::Parser;
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn cli(root: &Path, inputs: &Path) -> TestResult<Cli> {
    Ok(Cli::try_parse_from([
        "census-service",
        "--store",
        root.to_str().ok_or("store path")?,
        "nces-address-accept",
        "--source-root",
        inputs.to_str().ok_or("input path")?,
        "--ccd",
        "ccd.csv",
        "--ccd-sha256",
        "57c07542819bb066c4ec68e20ab2eb53d32eeccd84408e457418fa0ecd37f94d",
        "--ccd-url",
        "https://fixtures.invalid/nces/ccd.csv",
        "--ccd-captured-on",
        "2026-10-20",
        "--pss",
        "pss.csv",
        "--pss-sha256",
        "777b31b02fb7f47652049f8f6ad81a8b1f8f875aaa4ff948f173fea4cfe1adcf",
        "--pss-url",
        "https://fixtures.invalid/nces/pss.csv",
        "--pss-captured-on",
        "2026-10-20",
        "--apply",
    ])?)
}

fn execute(cli: &Cli) -> Result<()> {
    match &cli.command {
        crate::cli::Command::NcesAddressAccept(args) => Ok(run(cli, args)?),
        other => Err(anyhow::anyhow!("unexpected command {other:?}")),
    }
}

#[test]
fn nces_cli_opens_existing_current_qualification_and_replays_without_new_writes() -> TestResult {
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("store");
    let inputs = parent.path().join("inputs");
    std::fs::create_dir(&inputs)?;
    std::fs::write(
        inputs.join("ccd.csv"),
        include_bytes!("../../../tests/fixtures/nces_run/ccd_synthetic.csv"),
    )?;
    std::fs::write(
        inputs.join("pss.csv"),
        include_bytes!("../../../tests/fixtures/nces_run/pss_synthetic.csv"),
    )?;
    Store::create_current_qualification_v3(&root)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let manifest = RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("school year")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: UsJurisdiction::CENSUS_SCOPE
            .iter()
            .copied()
            .filter(|state| {
                [
                    UsJurisdiction::Michigan,
                    UsJurisdiction::Minnesota,
                    UsJurisdiction::Iowa,
                    UsJurisdiction::Illinois,
                    UsJurisdiction::Colorado,
                ]
                .contains(state)
            })
            .collect(),
    };
    store.bind_run(&manifest)?;
    drop((store, writer));
    let command = cli(&root, &inputs)?;
    execute(&command)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    let receipt = store.nces_address_run_v1()?.ok_or("address receipt")?;
    let sequence = store.snapshot().sequence();
    check!(eq; store.run_manifest()?, Some(manifest.clone()));
    drop((store, writer));
    execute(&command)?;
    let (store, writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    check!(eq; store.nces_address_run_v1()?, Some(receipt));
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.run_manifest()?, Some(manifest));
    drop((store, writer));
    check!(Store::open(&root).is_err());
    Ok(())
}

#[test]
fn nces_cli_missing_qualification_refuses_without_creating_a_store() -> TestResult {
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("absent");
    check!(execute(&cli(&root, parent.path())?).is_err());
    check!(!root.exists());
    Ok(())
}

#[test]
fn nces_cli_does_not_enroll_an_existing_unqualified_store() -> TestResult {
    let parent = tempfile::tempdir()?;
    let root = parent.path().join("ordinary");
    let store = Store::open(&root)?;
    let identity = store.store_identity()?;
    let sequence = store.snapshot().sequence();
    drop(store);
    let error = execute(&cli(&root, parent.path())?)
        .err()
        .ok_or("unqualified Store was accepted")?;
    check!(matches!(
        error.downcast_ref::<census_store::StoreError>(),
        Some(census_store::StoreError::PublicIdentity(
            census_store::PublicIdentityError::OptInRequired
        ))
    ));
    let store = Store::open(&root)?;
    check!(eq; store.store_identity()?, identity);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(store.nces_address_run_v1()?.is_none());
    check!(!store.out_dir().join("nces-address").exists());
    Ok(())
}
