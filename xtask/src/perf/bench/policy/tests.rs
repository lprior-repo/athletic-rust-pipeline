use super::super::{memory, runtime, select_timing};
use super::{declared, declared_for_scope, TimingAggregation};
use crate::perf::compare::CAPTURE_EXPORT_WORKLOAD;
use crate::perf::scope::CRITERION_SCOPE;
use crate::perf::{GroupMeasurement, Throughput};
use std::collections::BTreeMap;
use std::path::Path;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn trial(entries: &[(&str, f64, f64)]) -> BTreeMap<String, GroupMeasurement> {
    let mut trial = BTreeMap::new();
    for (id, wall_time_seconds, tail_time_seconds) in entries {
        trial.insert(
            (*id).to_owned(),
            GroupMeasurement {
                throughput: Some(Throughput::Elements(1.0)),
                peak_rss_kib: None,
                allocation_count: None,
                allocated_bytes: None,
                tail_time_seconds: Some(*tail_time_seconds),
                timing_scope: None,
                wall_time_seconds: *wall_time_seconds,
            },
        );
    }
    trial
}

#[test]
fn each_benchmark_selects_its_fastest_trial_with_that_trials_tail() -> TestResult {
    let trials = vec![
        trial(&[("group/first", 3.0, 30.0), ("group/second", 2.0, 20.0)]),
        trial(&[("group/first", 1.0, 10.0), ("group/second", 5.0, 50.0)]),
        trial(&[("group/first", 2.0, 200.0), ("group/second", 3.0, 300.0)]),
    ];
    let selected = select_timing("core", trials)?;
    let first = selected
        .get("group/first")
        .ok_or_else(|| anyhow::anyhow!("group/first was not selected"))?;
    check!(eq; first.wall_time_seconds, 1.0);
    check!(eq; first.tail_time_seconds, Some(10.0));
    let second = selected
        .get("group/second")
        .ok_or_else(|| anyhow::anyhow!("group/second was not selected"))?;
    check!(eq; second.wall_time_seconds, 2.0);
    check!(eq; second.tail_time_seconds, Some(20.0));
    Ok(())
}

#[test]
fn equal_timings_keep_the_first_trial_deterministically() -> TestResult {
    let trials = vec![
        trial(&[("group/first", 2.0, 20.0)]),
        trial(&[("group/first", 2.0, 99.0)]),
    ];
    let selected = select_timing("core", trials)?;
    let first = selected
        .get("group/first")
        .ok_or_else(|| anyhow::anyhow!("group/first was not selected"))?;
    check!(eq; first.tail_time_seconds, Some(20.0));
    Ok(())
}

#[test]
fn benchmark_trials_with_different_ids_are_refused() -> TestResult {
    for trials in [
        vec![
            trial(&[("group/first", 1.0, 1.0)]),
            trial(&[("group/second", 1.0, 1.0)]),
        ],
        vec![
            trial(&[("group/first", 1.0, 1.0)]),
            trial(&[("group/first", 1.0, 1.0), ("group/second", 1.0, 1.0)]),
        ],
    ] {
        let error = select_timing("core", trials)
            .err()
            .ok_or_else(|| anyhow::anyhow!("trials with different benchmark IDs were accepted"))?;
        check!(format!("{error:#}").contains("same benchmark IDs"));
    }
    check!(select_timing("core", Vec::new()).is_err());
    Ok(())
}

#[test]
fn undocumented_timing_evidence_scopes_have_no_declared_aggregation() -> TestResult {
    check!(eq;
        declared(CAPTURE_EXPORT_WORKLOAD)?,
        TimingAggregation::FastestTrial
    );
    check!(eq;
        declared("group/first")?,
        TimingAggregation::FastestTrial
    );
    check!(eq;
        declared_for_scope(CRITERION_SCOPE)?,
        TimingAggregation::FastestTrial
    );
    let error = declared_for_scope("capture-publication/v1")
        .err()
        .ok_or_else(|| anyhow::anyhow!("an undocumented evidence scope was accepted"))?;
    check!(format!("{error:#}").contains("undocumented timing evidence scope"));
    Ok(())
}

#[test]
#[cfg(unix)]
fn failing_or_absent_trial_executables_are_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let failure = match runtime::run_trial(Path::new("/bin/false"), directory.path()) {
        Err(error) => error,
        Ok(_) => return Err(anyhow::anyhow!("failing trial was accepted").into()),
    };
    check!(failure.to_string().contains("benchmark trial exited"));
    let absent = match runtime::run_trial(Path::new("/nonexistent/benchmark"), directory.path()) {
        Err(error) => error,
        Ok(_) => return Err(anyhow::anyhow!("absent trial executable was accepted").into()),
    };
    check!(format!("{absent:#}").contains("running benchmark trial"));
    Ok(())
}

#[test]
fn missing_memory_measurement_tools_are_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut absent = std::process::Command::new("athletic-rust-pipeline-absent-tool");
    let error =
        match memory::run_measurement(&mut absent, directory.path(), "^group/first$", "GNU time") {
            Err(error) => error,
            Ok(()) => return Err(anyhow::anyhow!("absent measurement tool was accepted").into()),
        };
    check!(format!("{error:#}").contains("GNU time is required for the memory measurement"));
    Ok(())
}
