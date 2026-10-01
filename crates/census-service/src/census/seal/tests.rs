use super::workbook::inspect_workbook;
use super::{reached_phase, retained_access};
use crate::census::Phase;
use census_domain::model::{
    AccessBlockKind, CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear,
    SourceAccessCondition, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_store::{Store, StoreStats, Table};
use std::path::Path;

fn stats_with(snapshots: u64, review: u64, coverage: u64, observations: u64) -> StoreStats {
    StoreStats {
        tables: vec![
            (Table::Snapshots.file().to_string(), snapshots),
            (Table::ReviewCases.file().to_string(), review),
            (Table::Coverage.file().to_string(), coverage),
        ],
        appended: Vec::new(),
        observations,
        bytes_on_disk: 0,
        store_bytes: 0,
    }
}

#[test]
fn the_ladder_stops_at_the_first_artifact_the_store_lacks() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let absent = dir.path().join("no-workbook.xlsx");
    let phases = [
        (stats_with(0, 0, 0, 0), Phase::Discovering),
        (stats_with(1, 0, 0, 9), Phase::Reconciling),
        (stats_with(1, 2, 0, 9), Phase::Reviewing),
        (stats_with(1, 2, 3, 9), Phase::ResolvingGaps),
    ];
    for (stats, expected) in phases {
        let state = reached_phase(&stats, &absent).expect("walks");
        assert_eq!(state.phase(), expected);
        assert!(state.sealed().is_none(), "a walked ladder is not sealed");
    }
}

#[test]
fn a_workbook_is_what_lifts_the_ladder_into_exporting() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let store = Store::open(dir.path().join("store")).expect("own store");
    let path = census_report::workbook::build(&store, &Default::default())
        .expect("publish verified workbook");
    let state = reached_phase(&stats_with(1, 2, 3, 9), &path).expect("walks");
    assert_eq!(
        state.phase(),
        Phase::Exporting,
        "only the export lifts the last step"
    );
}

#[test]
fn access_conditions_split_into_refusals_and_throttles() {
    let rows = vec![
        condition(AccessBlockKind::Forbidden, "refused.test"),
        condition(AccessBlockKind::RobotsDisallowed, "disallowed.test"),
        condition(AccessBlockKind::HumanRequired, "profile.test"),
        condition(AccessBlockKind::RateLimited, "throttled.test"),
        condition(AccessBlockKind::Timeout, "slow.test"),
        condition(AccessBlockKind::Unavailable, "down.test"),
        condition(AccessBlockKind::BrowserUnavailable, "lane.test"),
    ];

    assert_eq!(
        retained_access(&rows),
        (7, 3, 4),
        "three refusals and four cooldown-bounded rows over seven"
    );
    assert_eq!(
        retained_access(&[]),
        (0, 0, 0),
        "a store that has never been blocked retains nothing"
    );
}

fn condition(kind: AccessBlockKind, host: &str) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "milesplit",
        host,
        kind,
        403,
        "2026-09-22T00:00:00Z",
        "probe",
    )
}

fn populated_store(path: &Path) -> Store {
    let store = Store::open(path).expect("own store");
    let (mut school, id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Test School", "test school");
    let evidence = Evidence::parsed(
        SourceRef::new(
            "milesplit",
            Some("https://www.milesplit.com/athletes/1234".into()),
        ),
        "2026-06-01",
    );
    school.evidence.push(evidence.clone());
    let mut athlete = CanonicalAthlete::new(
        &id,
        "Ada Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "1234"),
    );
    athlete.evidence.push(evidence);
    store
        .append(Table::Schools, &school)
        .expect("persist school");
    store
        .append(Table::Athletes, &athlete)
        .expect("persist athlete");
    store
}

#[test]
fn a_complete_frozen_bundle_is_the_seal_certificate() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = populated_store(directory.path());
    let path = census_report::workbook::build(&store, &Default::default()).expect("publish");
    let current = ExportDataset::load(&store).expect("current source input");
    let check =
        inspect_workbook(&path, &current, 2027, Scope::AllSources).expect("certify publication");
    assert_eq!(check.mapped_athletes, 1);
    assert!(
        check.export_verified
            && check.counts_reconciled
            && check.coverage_reconciled
            && check.metrics_reconciled
    );
    let manifest: serde_json::Value = serde_json::from_slice(
        &std::fs::read(path.parent().expect("generation").join("manifest.json"))
            .expect("read manifest"),
    )
    .expect("decode manifest");
    assert_eq!(
        check.digests,
        [manifest["generation_digest"]
            .as_str()
            .expect("generation digest")]
    );
}

#[test]
fn a_limited_bundle_cannot_certify_the_whole_cohort() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = populated_store(directory.path());
    let options = census_report::workbook::Options {
        limit: Some(0),
        ..Default::default()
    };
    let path =
        census_report::workbook::build(&store, &options).expect("publish qualification slice");
    let current = ExportDataset::load(&store).expect("current input");
    let error = inspect_workbook(&path, &current, 2027, Scope::AllSources)
        .expect_err("limited seal refused");
    assert!(format!("{error:#}").contains("complete publication of the requested scope and cohort"));
}

#[test]
fn new_source_evidence_invalidates_the_old_seal_candidate() {
    let directory = tempfile::tempdir().expect("scratch directory");
    let store = populated_store(directory.path());
    let path = census_report::workbook::build(&store, &Default::default()).expect("publish");
    let (school, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Later School", "later school");
    store
        .append(Table::Schools, &school)
        .expect("append later source evidence");
    let current = ExportDataset::load(&store).expect("current input");
    let error =
        inspect_workbook(&path, &current, 2027, Scope::AllSources).expect_err("stale seal refused");
    assert!(format!("{error:#}").contains("publication is stale"));
}
