use super::{baseline, measurement};
use crate::perf::{candidate, PerfBaseline, PerfCommand, Throughput};
use std::path::{Path, PathBuf};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn current() -> PerfBaseline {
    let mut value = baseline(measurement(Some(101.0), 0.99));
    value.metadata.sha = "independent-candidate-revision".into();
    value
}

fn write_pair(directory: &Path, candidate: &PerfBaseline) -> TestResult<(PathBuf, PathBuf)> {
    let reference = directory.join("baseline.json");
    let measured = directory.join("candidate.json");
    std::fs::write(
        &reference,
        serde_json::to_vec_pretty(&baseline(measurement(Some(100.0), 1.0)))?,
    )?;
    std::fs::write(&measured, serde_json::to_vec_pretty(candidate)?)?;
    Ok((reference, measured))
}

#[test]
fn explicit_comparison_retains_inputs_and_accepts_compatible_measurements() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (baseline, candidate) = write_pair(directory.path(), &current())?;
    let original = std::fs::read(&baseline)?;
    let measured = std::fs::read(&candidate)?;
    PerfCommand::Compare {
        baseline: baseline.clone(),
        candidate: candidate.clone(),
        tolerance: 0.05,
        reason: Some("independent retained test fixtures".into()),
    }
    .run()?;
    check!(eq; std::fs::read(baseline)?, original);
    check!(eq; std::fs::read(candidate)?, measured);
    Ok(())
}

#[test]
fn explicit_comparison_does_not_rebase_away_a_regression() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut value = current();
    value
        .groups
        .get_mut("group/benchmark")
        .ok_or("test metric")?
        .throughput = Some(Throughput::Elements(1.0));
    let (baseline, candidate) = write_pair(directory.path(), &value)?;
    let original = std::fs::read(&baseline)?;
    let error = candidate::compare_files(&baseline, &candidate, 0.05, None)
        .err()
        .ok_or("regressed artifact was accepted")?;
    check!(error.to_string().contains("regression"));
    check!(eq; std::fs::read(baseline)?, original);
    Ok(())
}

#[test]
fn every_environment_and_corpus_mismatch_still_refuses_artifacts() -> TestResult {
    let directory = tempfile::tempdir()?;
    for field in ["CPU", "Core count", "rustc", "Corpus size", "Corpus digest"] {
        let mut value = current();
        match field {
            "CPU" => value.metadata.cpu = "other processor".into(),
            "Core count" => value.metadata.cores = 2,
            "rustc" => value.metadata.rustc = "other rustc".into(),
            "Corpus size" => value.metadata.corpus_lines = 2,
            "Corpus digest" => value.metadata.corpus_sha256 = "b".repeat(64),
            _ => return Err("invalid mismatch case".into()),
        }
        let (baseline, candidate) = write_pair(directory.path(), &value)?;
        let error = candidate::compare_files(&baseline, &candidate, 0.05, None)
            .err()
            .ok_or("incompatible artifacts were accepted")?;
        check!(
            error.to_string().contains(&format!("{field} mismatch")),
            "{error:#}"
        );
    }
    Ok(())
}

#[test]
fn missing_ids_memory_and_scope_cannot_pass_artifact_comparison() -> TestResult {
    let directory = tempfile::tempdir()?;
    for field in ["workload", "metric", "memory", "scope"] {
        let mut value = current();
        match field {
            "workload" => {
                value
                    .groups
                    .remove(crate::perf::compare::CAPTURE_EXPORT_WORKLOAD);
            }
            "metric" => {
                value.groups.remove("group/benchmark");
            }
            "memory" => {
                value
                    .groups
                    .get_mut("group/benchmark")
                    .ok_or("test metric")?
                    .peak_rss_kib = None;
            }
            "scope" => {
                value
                    .groups
                    .get_mut("group/benchmark")
                    .ok_or("test metric")?
                    .timing_scope = Some("incompatible-workload-scope".into());
            }
            _ => return Err("invalid metric case".into()),
        }
        let (baseline, candidate) = write_pair(directory.path(), &value)?;
        check!(
            candidate::compare_files(&baseline, &candidate, 0.05, None).is_err(),
            "{field}"
        );
    }
    Ok(())
}

#[test]
fn invalid_baseline_metrics_are_not_authorized_by_a_valid_candidate() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (baseline, candidate) = write_pair(directory.path(), &current())?;
    let mut invalid = super::baseline(measurement(Some(100.0), 1.0));
    invalid
        .groups
        .get_mut("group/benchmark")
        .ok_or("test metric")?
        .allocated_bytes = None;
    std::fs::write(&baseline, serde_json::to_vec(&invalid)?)?;
    check!(candidate::compare_files(&baseline, &candidate, 0.05, None).is_err());
    Ok(())
}

#[test]
fn same_path_and_normalized_copies_are_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let value = baseline(measurement(Some(100.0), 1.0));
    let (baseline, candidate) = write_pair(directory.path(), &value)?;
    let error = candidate::compare_files(&baseline, &baseline, 0.05, None)
        .err()
        .ok_or("self comparison was accepted")?;
    check!(error
        .to_string()
        .contains("different performance artifacts"));
    std::fs::write(&candidate, serde_json::to_vec(&value)?)?;
    let error = candidate::compare_files(&baseline, &candidate, 0.05, None)
        .err()
        .ok_or("normalized copied artifact was accepted")?;
    check!(error.to_string().contains("same performance artifact"));
    Ok(())
}

#[cfg(unix)]
#[test]
fn symbolic_and_hard_link_aliases_do_not_make_an_independent_candidate() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (baseline, _) = write_pair(directory.path(), &current())?;
    let symbolic = directory.path().join("symbolic.json");
    let hard = directory.path().join("hard.json");
    std::os::unix::fs::symlink(&baseline, &symbolic)?;
    std::fs::hard_link(&baseline, &hard)?;
    for candidate in [symbolic, hard] {
        check!(candidate::compare_files(&baseline, &candidate, 0.05, None).is_err());
    }
    Ok(())
}

#[test]
fn artifact_comparison_keeps_the_exact_tolerance_admission_rule() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (baseline, candidate) = write_pair(directory.path(), &current())?;
    for tolerance in [-0.01, 1.0, f64::NAN, f64::INFINITY] {
        check!(candidate::compare_files(&baseline, &candidate, tolerance, None).is_err());
    }
    Ok(())
}
