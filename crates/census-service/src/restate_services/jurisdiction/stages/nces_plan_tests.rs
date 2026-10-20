use super::*;
use crate::restate_services::plan::{
    compute_plan_fingerprint, compute_plan_fingerprint_with_nces_receipt, plan as planned,
    BrowserLaneState,
};
use crate::restate_services::wire::{HistoryWindow, JurisdictionRequest};
use crate::school_address::{accept_nces_address_run, NcesSelectedArtifactV1, NcesSelectedInputV1};
use census_domain::model::{
    serialized_digest, CensusRun, GradYear, Revision, RunManifest, SchoolYear,
};
use census_domain::UsJurisdiction;
use census_store::Store;
use std::path::PathBuf;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SELECTED_SCOPE: [UsJurisdiction; 5] = [
    UsJurisdiction::Michigan,
    UsJurisdiction::Minnesota,
    UsJurisdiction::Iowa,
    UsJurisdiction::Illinois,
    UsJurisdiction::Colorado,
];

#[test]
fn nces_plan_requires_a_valid_reopened_run_bound_receipt() -> TestResult {
    const CCD: &[u8] = include_bytes!("../../../../tests/fixtures/nces_run/ccd_synthetic.csv");
    const PSS: &[u8] = include_bytes!("../../../../tests/fixtures/nces_run/pss_synthetic.csv");
    const CCD_SHA: &str = "57c07542819bb066c4ec68e20ab2eb53d32eeccd84408e457418fa0ecd37f94d";
    const PSS_SHA: &str = "777b31b02fb7f47652049f8f6ad81a8b1f8f875aaa4ff948f173fea4cfe1adcf";

    let root = tempfile::tempdir()?;
    let source_root = root.path().join("inputs");
    std::fs::create_dir(&source_root)?;
    std::fs::write(source_root.join("ccd.csv"), CCD)?;
    std::fs::write(source_root.join("pss.csv"), PSS)?;
    let store = Store::open(root.path().join("store"))?;
    let run = RunManifest {
        store_identity: store.store_identity()?.as_str().to_string(),
        run: CensusRun::new(SchoolYear::new(2026).ok_or("season")?, 1).ok_or("run")?,
        cohort: GradYear::CO2027,
        jurisdictions: SELECTED_SCOPE.to_vec(),
    };
    store.bind_run(&run)?;
    let run_digest = serialized_digest(&run)?;
    let request = request(UsJurisdiction::Colorado)?;
    check!(eq;
        qualified_nces_receipt_digest(&store, &request).map_err(handler_error)?,
        None
    );

    let selection = NcesSelectedInputV1 {
        source_root,
        run_manifest_digest: run_digest,
        scope: run.jurisdictions.clone(),
        ccd: NcesSelectedArtifactV1 {
            path: PathBuf::from("ccd.csv"),
            sha256: CCD_SHA.to_string(),
            source_url: "https://fixtures.invalid/nces/ccd.csv".to_string(),
            captured_on: "2026-10-20".to_string(),
        },
        pss: NcesSelectedArtifactV1 {
            path: PathBuf::from("pss.csv"),
            sha256: PSS_SHA.to_string(),
            source_url: "https://fixtures.invalid/nces/pss.csv".to_string(),
            captured_on: "2026-10-20".to_string(),
        },
    };
    let receipt = accept_nces_address_run(&store, &selection)?;
    check!(eq;
        qualified_nces_receipt_digest(&store, &request).map_err(handler_error)?,
        Some(serialized_digest(&receipt)?)
    );
    let mut wrong_request = request;
    wrong_request.revision = Revision(2);
    check!(eq;
        qualified_nces_receipt_digest(&store, &wrong_request).map_err(handler_error)?,
        None
    );
    Ok(())
}

#[test]
fn nces_plan_upgrades_only_the_legacy_refusal_to_receipt_satisfaction() -> TestResult {
    let jurisdiction = UsJurisdiction::Colorado;
    let season = SchoolYear::new(2026).ok_or("season")?;
    let revision = Revision(1);
    let lane = BrowserLaneState::Absent;
    let legacy_fingerprint = compute_plan_fingerprint(jurisdiction, season, revision, lane);
    let digest = "a".repeat(64);
    let qualified_fingerprint = compute_plan_fingerprint_with_nces_receipt(
        jurisdiction,
        season,
        revision,
        lane,
        Some(&digest),
    );
    check!(eq;
        compute_plan_fingerprint_with_nces_receipt(
            jurisdiction, season, revision, lane, None
        ),
        legacy_fingerprint
    );
    check!(qualified_fingerprint != legacy_fingerprint);

    let mut prior = SourcePlan::of(&planned(jurisdiction, lane), legacy_fingerprint.clone());
    check!(prior.refused.iter().any(|row| row.slug == "nces"));
    let mut destination = Some(prior.clone());
    upgrade_legacy_nces_plan(
        &prior,
        &mut destination,
        &legacy_fingerprint,
        &qualified_fingerprint,
        &digest,
    )
    .map_err(handler_error)?;
    prior = destination.ok_or("plan upgrade missing")?;
    validate_nces_plan_state(&prior, Some(&digest)).map_err(handler_error)?;
    check!(!prior.refused.iter().any(|row| row.slug == "nces"));
    check!(!prior.sweepable.iter().any(|slug| slug == "nces"));
    check!(eq; prior.satisfied.len(), 1);
    check!(eq; prior.satisfied[0].slug.as_str(), "nces");
    check!(eq; prior.satisfied[0].evidence_digest.as_str(), digest);
    check!(validate_nces_plan_state(&prior, None).is_err());
    Ok(())
}

fn request(jurisdiction: UsJurisdiction) -> TestResult<JurisdictionRequest> {
    Ok(JurisdictionRequest {
        jurisdiction,
        season: SchoolYear::new(2026).ok_or("season")?,
        revision: Revision(1),
        history: HistoryWindow::cohort("2026-10-20")?,
        refresh: false,
        limit_per_state: None,
        concurrency: 1,
        observed_on: Some("2026-10-20".to_string()),
        authorized_hosts: Vec::new(),
        source_parallelism: 1,
    })
}

fn handler_error(error: restate_sdk::errors::HandlerError) -> Box<dyn std::error::Error> {
    format!("{error:?}").into()
}
