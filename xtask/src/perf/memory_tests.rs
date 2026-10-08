use super::{baseline, groups, measurement};
use crate::perf::compare::{check_throughput, compare_environment};
use crate::perf::{GroupMeasurement, Meta};
use anyhow::{anyhow, Result};

fn metadata() -> Meta {
    Meta {
        cpu: "CPU A".into(),
        cores: 32,
        rustc: "rustc A".into(),
        sha: "revision A".into(),
        corpus_lines: 100,
        corpus_sha256: "a".repeat(64),
    }
}

fn gate_error(old: GroupMeasurement, new: GroupMeasurement) -> Result<String> {
    let baseline = baseline(old);
    let current = groups(new);
    match check_throughput(&baseline, &current, 0.05) {
        Err(error) => Ok(format!("{error:#}")),
        Ok(()) => Err(anyhow!("incompatible measurements were accepted")),
    }
}

#[test]
fn cpu_core_rustc_and_corpus_mismatches_each_refuse_comparison() -> Result<()> {
    for label in ["CPU", "Core count", "rustc", "Corpus size", "Corpus digest"] {
        let mut changed = metadata();
        match label {
            "CPU" => changed.cpu = "CPU B".into(),
            "Core count" => changed.cores = 16,
            "rustc" => changed.rustc = "rustc B".into(),
            "Corpus size" => changed.corpus_lines = 101,
            "Corpus digest" => changed.corpus_sha256 = "b".repeat(64),
            _ => return Err(anyhow!("invalid environment test case")),
        }
        let error = match compare_environment(&metadata(), &changed) {
            Err(error) => error,
            Ok(()) => return Err(anyhow!("{label} mismatch was accepted")),
        };
        check!(error.to_string().contains(&format!("{label} mismatch")));
    }
    Ok(())
}

#[test]
fn compatible_environment_allows_revision_change_without_waiving_comparability() -> Result<()> {
    let mut current = metadata();
    current.sha = "revision B".into();
    compare_environment(&metadata(), &current)?;
    Ok(())
}

#[test]
fn unavailable_environment_identity_cannot_match_itself() -> Result<()> {
    for label in ["cpu", "rustc", "cores", "corpus", "digest"] {
        let mut invalid = metadata();
        match label {
            "cpu" => invalid.cpu = "unknown".into(),
            "rustc" => invalid.rustc = String::new(),
            "cores" => invalid.cores = 0,
            "corpus" => invalid.corpus_lines = 0,
            "digest" => invalid.corpus_sha256.clear(),
            _ => return Err(anyhow!("invalid environment test case")),
        }
        check!(compare_environment(&invalid, &invalid).is_err(), "{label}");
    }
    Ok(())
}

fn set_memory(measurement: &mut GroupMeasurement, label: &str, value: Option<u64>) -> Result<()> {
    match label {
        "peak RSS" => measurement.peak_rss_kib = value,
        "allocation count" => measurement.allocation_count = value,
        "allocated bytes" => measurement.allocated_bytes = value,
        _ => return Err(anyhow!("invalid memory test case")),
    }
    Ok(())
}

#[test]
fn missing_and_zero_memory_on_either_side_fail_closed() -> Result<()> {
    for label in ["peak RSS", "allocation count", "allocated bytes"] {
        for value in [None, Some(0)] {
            let good = measurement(Some(100.0), 1.0);
            let mut invalid = good.clone();
            set_memory(&mut invalid, label, value)?;
            check!(gate_error(invalid.clone(), good.clone())?.contains(label));
            check!(gate_error(good, invalid)?.contains(label));
        }
    }
    Ok(())
}

#[test]
fn tenfold_memory_increase_fails_even_with_faster_timing_and_rate() -> Result<()> {
    for label in ["peak RSS", "allocation count", "allocated bytes"] {
        let old = measurement(Some(100.0), 1.0);
        let mut current = measurement(Some(200.0), 0.5);
        set_memory(&mut current, label, Some(1000))?;
        check!(gate_error(old, current)?.contains(&format!("{label} regression")));
    }
    Ok(())
}

#[test]
fn compatible_positive_memory_metrics_at_budget_boundary_pass() -> Result<()> {
    let old = baseline(measurement(Some(100.0), 1.0));
    let mut current = measurement(Some(100.0), 1.0);
    current.peak_rss_kib = Some(105);
    current.allocation_count = Some(105);
    current.allocated_bytes = Some(105);
    check_throughput(&old, &groups(current), 0.05)?;
    Ok(())
}

#[test]
fn memory_just_above_tolerance_fails_independently_for_each_budget() -> Result<()> {
    for label in ["peak RSS", "allocation count", "allocated bytes"] {
        let old = measurement(Some(100.0), 1.0);
        let mut current = old.clone();
        set_memory(&mut current, label, Some(106))?;
        check!(gate_error(old, current)?.contains(&format!("{label} regression")));
    }
    Ok(())
}

#[test]
fn invalid_serialized_memory_cannot_become_a_positive_measurement() -> Result<()> {
    for field in ["peak_rss_kib", "allocation_count", "allocated_bytes"] {
        for raw in ["-1", "0.5", "1e309", "\"NaN\"", "18446744073709551616"] {
            let json = format!(r#"{{"wall_time_seconds":1,"{field}":{raw}}}"#);
            check!(
                serde_json::from_str::<GroupMeasurement>(&json).is_err(),
                "{field}: {raw}"
            );
        }
    }
    Ok(())
}

#[test]
fn missing_invalid_or_regressed_tail_cannot_hide_behind_fast_mean() -> Result<()> {
    for tail in [
        None,
        Some(0.0),
        Some(-1.0),
        Some(f64::NAN),
        Some(f64::INFINITY),
        Some(10.0),
    ] {
        let old = measurement(Some(100.0), 1.0);
        let mut current = measurement(Some(200.0), 0.5);
        current.tail_time_seconds = tail;
        check!(gate_error(old.clone(), current.clone())?.contains("tail time"));
        if tail != Some(10.0) {
            check!(gate_error(current, old)?.contains("tail time"));
        }
    }
    Ok(())
}

#[test]
fn wall_time_regression_is_checked_even_when_declared_rate_improves() -> Result<()> {
    let old = measurement(Some(100.0), 1.0);
    let current = measurement(Some(200.0), 10.0);
    check!(gate_error(old, current)?.contains("wall time regression"));
    Ok(())
}

#[test]
fn changed_json_capture_with_identical_text_line_count_refuses_perf_comparison() -> Result<()> {
    let corpus = tempfile::tempdir()?;
    std::fs::write(
        corpus.path().join("index.html"),
        "<html>captured index</html>\n",
    )?;
    let json = corpus.path().join("results.json");
    std::fs::write(&json, br#"{"result":"10.941"}"#)?;
    let first = crate::perf::corpus::measure(corpus.path())?;
    let mut old = metadata();
    old.corpus_lines = first.lines;
    old.corpus_sha256 = first.sha256;
    std::fs::write(&json, br#"{"result":"10.944"}"#)?;
    let second = crate::perf::corpus::measure(corpus.path())?;
    let mut current = metadata();
    current.corpus_lines = second.lines;
    current.corpus_sha256 = second.sha256;
    let error = compare_environment(&old, &current)
        .err()
        .ok_or_else(|| anyhow!("changed capture accepted as comparable"))?;
    check!(error.to_string().contains("Corpus digest mismatch"));
    Ok(())
}

#[test]
fn absent_old_or_unknown_memory_backend_refuses_comparison_even_against_itself() -> Result<()> {
    for scope in [
        None,
        Some("criterion-per-iteration/v1"),
        Some("criterion-per-iteration/v1;memory=memcheck"),
        Some("criterion-per-iteration/v1;memory=dhat-heap-json-v3"),
        Some(crate::perf::scope::expected(
            crate::perf::compare::CAPTURE_EXPORT_WORKLOAD,
        )),
    ] {
        let mut invalid = measurement(Some(100.0), 1.0);
        invalid.timing_scope = scope.map(str::to_owned);
        let good = measurement(Some(100.0), 1.0);
        for (old, current) in [
            (invalid.clone(), invalid.clone()),
            (invalid.clone(), good.clone()),
            (good.clone(), invalid),
        ] {
            check!(gate_error(old, current)?.contains("memory backend"));
        }
    }
    Ok(())
}

#[test]
fn integer_memory_budget_preserves_exact_five_percent_above_float_precision() -> Result<()> {
    for label in ["peak RSS", "allocation count", "allocated bytes"] {
        let mut old = measurement(Some(100.0), 1.0);
        let mut current = old.clone();
        set_memory(&mut old, label, Some(72_057_594_037_928_040))?;
        set_memory(&mut current, label, Some(75_660_473_739_824_442))?;
        check_throughput(&baseline(old.clone()), &groups(current.clone()), 0.05)?;
        set_memory(&mut current, label, Some(75_660_473_739_824_443))?;
        check!(gate_error(old, current)?.contains(&format!("{label} regression")));
    }
    Ok(())
}
