use super::workbook::inspect_workbook;
use super::{reached_phase, retained_access};
use crate::census::{AcceptanceItem, Phase};
use census_domain::model::{
    AccessBlockKind, CanonicalAthlete, CanonicalSchool, CensusRun, Evidence, Gender, GradYear,
    RunManifest, SchoolYear, SourceAccessCondition, SourceIdentity, SourceNamespace, SourceRef,
};
use census_domain::UsJurisdiction;
use census_report::export::ExportDataset;
use census_report::report::Scope;
use census_store::{Store, StoreStats, Table};
use std::path::Path;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn full_options() -> TestResult<census_report::workbook::Options> {
    Ok(census_report::workbook::Options {
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
    })
}

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

fn bind_census_run(store: &Store, season: i16, revision: u32) -> TestResult<CensusRun> {
    let run = CensusRun::new(SchoolYear::new(season).ok_or("invalid season")?, revision)
        .ok_or("invalid run")?;
    let identity = census_report::export::store_identity(store)?;
    store.bind_run(&RunManifest {
        store_identity: identity,
        run,
        cohort: GradYear::CO2027,
        jurisdictions: vec![UsJurisdiction::Wisconsin],
    })?;
    Ok(run)
}

#[test]
fn the_ladder_stops_at_the_first_artifact_the_store_lacks() -> TestResult {
    let dir = tempfile::tempdir()?;
    let absent = dir.path().join("no-workbook.xlsx");
    let phases = [
        (stats_with(0, 0, 0, 0), Phase::Discovering),
        (stats_with(1, 0, 0, 9), Phase::Reconciling),
        (stats_with(1, 2, 0, 9), Phase::Reviewing),
        (stats_with(1, 2, 3, 9), Phase::ResolvingGaps),
    ];
    for (stats, expected) in phases {
        let state = reached_phase(&stats, &absent)?;
        check!(eq; state.phase(), expected);
        check!(state.sealed().is_none(), "a walked ladder is not sealed");
    }
    Ok(())
}

#[test]
fn a_workbook_is_what_lifts_the_ladder_into_exporting() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path().join("store"))?;
    let path = census_report::workbook::build(&store, &full_options()?)?;
    let state = reached_phase(&stats_with(1, 2, 3, 9), &path)?;
    check!(eq;
        state.phase(),
        Phase::Exporting,
        "only the export lifts the last step"
    );
    Ok(())
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

fn populated_store(path: &Path) -> TestResult<Store> {
    let store = Store::open(path)?;
    let (mut school, id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Test School",
        "test school",
        None,
    );
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
    athlete
        .published_graduations
        .push(census_domain::model::PublishedGraduation {
            grad_year: GradYear::CO2027,
            source: evidence.source.clone(),
        });
    athlete.evidence.push(evidence);
    store.append(Table::Schools, &school)?;
    store.append(Table::Athletes, &athlete)?;
    Ok(store)
}

#[test]
fn a_complete_frozen_bundle_is_the_seal_certificate() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let path = census_report::workbook::build(&store, &full_options()?)?;
    let current = ExportDataset::load(&store)?;
    let check = inspect_workbook(&path, &current, GradYear::CO2027, Scope::AllSources)?;
    check!(eq; check.mapped_athletes, 1);
    check!(
        check.export_verified
            && check.counts_reconciled
            && check.coverage_reconciled
            && check.metrics_reconciled
    );
    let manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(
        path.parent()
            .ok_or("missing generation directory")?
            .join("manifest.json"),
    )?)?;
    check!(eq;
        check.digests,
        [manifest["generation_digest"]
            .as_str()
            .ok_or("missing generation digest")?]
    );
    Ok(())
}

#[test]
fn a_limited_bundle_cannot_certify_the_whole_cohort() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let options = census_report::workbook::Options {
        limit: Some(0),
        ..full_options()?
    };
    let path = census_report::workbook::build(&store, &options)?;
    let current = ExportDataset::load(&store)?;
    let error = match inspect_workbook(&path, &current, GradYear::CO2027, Scope::AllSources) {
        Err(error) => error,
        Ok(_) => return Err("limited bundle certified the whole cohort".into()),
    };
    check!(format!("{error:#}").contains("complete publication of the requested scope and cohort"));
    Ok(())
}

#[test]
fn new_source_evidence_invalidates_the_old_seal_candidate() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let path = census_report::workbook::build(&store, &full_options()?)?;
    let (school, _) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Later School",
        "later school",
        None,
    );
    store.append(Table::Schools, &school)?;
    let current = ExportDataset::load(&store)?;
    let error = match inspect_workbook(&path, &current, GradYear::CO2027, Scope::AllSources) {
        Err(error) => error,
        Ok(_) => return Err("stale bundle certified".into()),
    };
    check!(format!("{error:#}").contains("publication is stale"));
    Ok(())
}

#[test]
fn a_foreign_cohort_is_refused_before_anything_is_measured() -> TestResult {
    use super::census_cohort;
    let error = census_cohort(2028).err().ok_or("expected ForeignCohort")?;
    let text = error.to_string();
    check!(
        text.contains("Class-of-2027") && text.contains("2028"),
        "the message names both the census seal cohort and the request"
    );
    Ok(())
}

#[test]
fn a_request_run_disagreeing_with_the_bound_run_is_a_typed_error() -> TestResult {
    use super::seal;
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    bind_census_run(&store, 2026, 1)?;
    let request = super::SealRequest {
        grad_year: 2027,
        scope: Scope::AllSources,
        workbook: None,
        write: false,
        journal: None,
        source_failures: None,
        run: Some(
            CensusRun::new(SchoolYear::new(2026).ok_or("invalid season")?, 2)
                .ok_or("invalid run")?,
        ),
    };
    let error = seal(&store, &request).err().ok_or("expected RunMismatch")?;
    let text = error.to_string();
    check!(
        text.contains("2026-2") && text.contains("2026-1"),
        "the message names both the measured and bound runs"
    );
    Ok(())
}

#[test]
fn an_unbound_store_carries_a_run_acceptance_item() -> TestResult {
    use super::seal;
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let path = census_report::workbook::build(&store, &full_options()?)?;
    let request = super::SealRequest {
        grad_year: 2027,
        scope: Scope::AllSources,
        workbook: Some(path),
        write: false,
        journal: None,
        source_failures: None,
        run: None,
    };
    let outcome = seal(&store, &request)?;
    let open = outcome.evidence.open_items();
    check!(
        open.contains(&AcceptanceItem::RunBound),
        "an unbound store cannot seal until a run identity is bound"
    );
    check!(
        outcome.state.sealed().is_none(),
        "the seal refused because of the open item"
    );
    Ok(())
}

#[test]
fn a_bound_run_reaches_the_evidence_and_the_seal_record() -> TestResult {
    use super::seal;
    let directory = tempfile::tempdir()?;
    let store = populated_store(directory.path())?;
    let run = bind_census_run(&store, 2026, 1)?;
    let path = census_report::workbook::build(&store, &full_options()?)?;
    let request = super::SealRequest {
        grad_year: 2027,
        scope: Scope::AllSources,
        workbook: Some(path),
        write: true,
        journal: Some(super::JournalCounts {
            jurisdiction_sweeps: Some(0),
            source_objects: Some(0),
            silent_sources: Vec::new(),
        }),
        source_failures: None,
        run: Some(run),
    };
    let outcome = seal(&store, &request)?;
    check!(eq; outcome.evidence.run, Some(run));
    check!(
        !outcome
            .evidence
            .open_items()
            .contains(&AcceptanceItem::RunBound),
        "a matching bound run clears the RunBound acceptance item"
    );
    let recorded = outcome
        .refusal
        .as_ref()
        .ok_or("a store whose phases never advanced must still refuse the seal")?;
    check!(
        recorded.contains("cannot move from acquiring to complete"),
        "the refusal names the unadvanced phase ladder: {recorded}"
    );
    check!(eq; outcome.recorded, None);
    Ok(())
}
