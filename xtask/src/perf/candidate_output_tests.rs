use super::{baseline, measurement};
use crate::perf::{baseline as artifacts, candidate, PerfBaseline};
use std::cell::Cell;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn candidate_record_preserves_all_retained_metrics_and_the_canonical_baseline() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("candidate.json");
    let canonical = artifacts::baseline_path();
    let before = std::fs::read(&canonical)?;
    let retained = artifacts::load_baseline()?;
    let expected = serde_json::to_value(&retained)?;
    candidate::record(&path, || Ok(retained))?;
    let measured: serde_json::Value = serde_json::from_slice(&std::fs::read(&path)?)?;
    check!(eq; measured, expected);
    check!(eq; std::fs::read(canonical)?, before);
    Ok(())
}

#[test]
fn existing_candidate_refuses_before_invoking_measurement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("existing.json");
    std::fs::write(&path, b"existing artifact")?;
    let measured = Cell::new(false);
    let result = candidate::record(&path, || {
        measured.set(true);
        Ok(baseline(measurement(Some(100.0), 1.0)))
    });
    check!(result.is_err());
    check!(!measured.get());
    check!(eq; std::fs::read(path)?, b"existing artifact");
    Ok(())
}

#[test]
fn canonical_candidate_refuses_before_invoking_measurement() -> TestResult {
    let path = artifacts::baseline_path();
    let before = std::fs::read(&path)?;
    let measured = Cell::new(false);
    let result = candidate::record(&path, || {
        measured.set(true);
        Ok(baseline(measurement(Some(100.0), 1.0)))
    });
    let error = result
        .err()
        .ok_or("canonical baseline was accepted as candidate output")?;
    check!(error.to_string().contains("canonical performance baseline"));
    check!(!measured.get());
    check!(eq; std::fs::read(path)?, before);
    Ok(())
}

#[test]
fn failed_or_invalid_measurement_leaves_no_candidate_artifact() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("candidate.json");
    let result = candidate::record(&path, || anyhow::bail!("controlled measurement refusal"));
    check!(result.is_err());
    check!(!path.exists());
    let result = candidate::record(&path, || {
        let mut data = baseline(measurement(Some(100.0), 1.0));
        data.groups.clear();
        Ok(data)
    });
    check!(result.is_err());
    check!(!path.exists());
    check!(eq; std::fs::read_dir(directory.path())?.count(), 0);
    Ok(())
}

#[test]
fn a_target_created_during_measurement_is_never_replaced() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("candidate.json");
    let result = candidate::record(&path, || {
        std::fs::write(&path, b"concurrent artifact")?;
        Ok(baseline(measurement(Some(100.0), 1.0)))
    });
    check!(result.is_err());
    check!(eq; std::fs::read(path)?, b"concurrent artifact");
    check!(eq; std::fs::read_dir(directory.path())?.count(), 1);
    Ok(())
}

#[test]
fn candidate_reload_retains_the_exact_existing_artifact_shape() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("candidate.json");
    let value = baseline(measurement(Some(100.0), 1.0));
    let expected = serde_json::to_value(&value)?;
    candidate::record(&path, || Ok(value))?;
    let reloaded: PerfBaseline = artifacts::load_baseline_from(&path)?;
    check!(eq; serde_json::to_value(reloaded)?, expected);
    Ok(())
}

#[cfg(unix)]
#[test]
fn canonical_directory_alias_and_dangling_target_refuse_before_measurement() -> TestResult {
    let directory = tempfile::tempdir()?;
    let canonical = artifacts::baseline_path();
    let parent = canonical.parent().ok_or("canonical baseline directory")?;
    let name = canonical.file_name().ok_or("canonical baseline name")?;
    let alias = directory.path().join("tools-alias");
    std::os::unix::fs::symlink(parent, &alias)?;
    let dangling = directory.path().join("dangling.json");
    std::os::unix::fs::symlink(directory.path().join("missing.json"), &dangling)?;
    for path in [alias.join(name), dangling.clone()] {
        let measured = Cell::new(false);
        let result = candidate::record(&path, || {
            measured.set(true);
            Ok(baseline(measurement(Some(100.0), 1.0)))
        });
        check!(result.is_err());
        check!(!measured.get());
    }
    check!(std::fs::symlink_metadata(dangling)?
        .file_type()
        .is_symlink());
    Ok(())
}
