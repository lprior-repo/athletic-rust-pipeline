use census_domain::model::{CensusRun, RunManifest, SchoolYear};
use census_domain::UsJurisdiction;
use census_store::{Store, StoreError};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn manifest(
    revision: u32,
    jurisdictions: Vec<UsJurisdiction>,
) -> Result<RunManifest, Box<dyn std::error::Error>> {
    let season = SchoolYear::new(2026).ok_or("2026 is a school year")?;
    Ok(RunManifest {
        store_identity: "c".repeat(64),
        run: CensusRun::new(season, revision).ok_or("a run revision starts at one")?,
        cohort: census_domain::model::GradYear::CO2027,
        jurisdictions,
    })
}

fn refusal(error: &StoreError) -> Option<&str> {
    match error {
        StoreError::Refused { detail } => Some(detail),
        _ => None,
    }
}

#[test]
fn a_store_without_a_run_carries_no_binding() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    if store.run_manifest()?.is_some() {
        return Err("a fresh store carries no run manifest".into());
    }
    Ok(())
}

#[test]
fn a_bound_run_survives_a_reopen_and_rebinds_idempotently() -> TestResult {
    let root = tempfile::tempdir()?;
    let bound = manifest(1, vec![UsJurisdiction::NewYork, UsJurisdiction::Ohio])?;
    {
        let store = Store::open(root.path())?;
        store.bind_run(&bound)?;
        store.bind_run(&bound)?;
        if store.run_manifest()?.as_ref() != Some(&bound) {
            return Err("the store reads back the run it was bound to".into());
        }
    }
    let store = Store::open(root.path())?;
    if store.run_manifest()?.as_ref() != Some(&bound) {
        return Err("the binding survives the process that wrote it".into());
    }
    Ok(())
}

#[test]
fn a_bound_store_refuses_every_other_run() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let bound = manifest(1, vec![UsJurisdiction::NewYork])?;
    store.bind_run(&bound)?;

    let later = manifest(2, vec![UsJurisdiction::NewYork])?;
    match store.bind_run(&later) {
        Err(error) => {
            let detail = refusal(&error).ok_or(format!("expected a binding refusal: {error:?}"))?;
            if !detail.contains("2026-1") || !detail.contains("2026-2") {
                return Err(format!("the refusal names both runs: {detail}").into());
            }
        }
        Ok(()) => return Err("a store cannot be rebound to a second run".into()),
    }

    let widened = manifest(1, vec![UsJurisdiction::NewYork, UsJurisdiction::Ohio])?;
    match store.bind_run(&widened) {
        Err(error) => {
            if refusal(&error).is_none() {
                return Err(format!("expected a binding refusal: {error:?}").into());
            }
        }
        Ok(()) => return Err("a run's obligation scope is bound with the run".into()),
    }

    if store.run_manifest()?.as_ref() != Some(&bound) {
        return Err("a refused binding leaves the retained one in place".into());
    }
    Ok(())
}
