use super::super::Throughput;
use super::{metadata, parse_bencher_line, parse_measurement, runtime};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn capture(root: &Path, directory: &str, json: &str) -> Result<()> {
    let directory = root.join(directory).join("new");
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("benchmark.json"), json)?;
    Ok(())
}

#[test]
fn criterion_full_ids_preserve_distinct_per_benchmark_throughputs() -> TestResult {
    let root = tempfile::tempdir()?;
    capture(
        root.path(),
        "group/first",
        r#"{"full_id":"group/first","throughput":{"Elements":42}}"#,
    )?;
    capture(
        root.path(),
        "group/second",
        r#"{"full_id":"group/second","throughput":{"Bytes":1024}}"#,
    )?;
    let declarations = metadata::read(root.path())?;
    let output = "test group/first ... bench: 1 s/iter (+/- 0)\ntest group/second ... bench: 2 s/iter (+/- 0)";
    let result = parse_measurement(output, &declarations, None)?;
    let first = result
        .get("group/first")
        .ok_or_else(|| anyhow::anyhow!("missing group/first measurement"))?;
    let second = result
        .get("group/second")
        .ok_or_else(|| anyhow::anyhow!("missing group/second measurement"))?;
    check!(eq;
        first.throughput,
        Some(Throughput::Elements(42.0))
    );
    check!(eq;
        second.throughput,
        Some(Throughput::Bytes(512.0))
    );
    Ok(())
}

#[test]
fn missing_duplicate_empty_and_malformed_metadata_refuse_collection() -> TestResult {
    for json in [
        "{",
        r#"{"full_id":"","throughput":null}"#,
        r#"{"full_id":"g/f","throughput":{"Elements":0}}"#,
    ] {
        let root = tempfile::tempdir()?;
        capture(root.path(), "group/first", json)?;
        check!(metadata::read(root.path()).is_err());
    }
    let root = tempfile::tempdir()?;
    check!(metadata::read(root.path()).is_err());
    capture(
        root.path(),
        "first",
        r#"{"full_id":"g/f","throughput":{"Elements":1}}"#,
    )?;
    capture(
        root.path(),
        "second",
        r#"{"full_id":"g/f","throughput":{"Elements":1}}"#,
    )?;
    check!(metadata::read(root.path()).is_err());
    Ok(())
}

#[test]
fn metadata_and_output_must_describe_the_same_nonempty_results() -> TestResult {
    let declared = BTreeMap::from([("g/f".into(), Some(Throughput::Elements(1)))]);
    for output in [
        "",
        "test g/other ... bench: 1 ns/iter",
        "test g/f ... bench: 1 ns/iter\ntest g/f ... bench: 2 ns/iter",
    ] {
        check!(parse_measurement(output, &declared, None).is_err());
    }
    let extra = BTreeMap::from([
        ("g/f".into(), Some(Throughput::Elements(1))),
        ("g/other".into(), Some(Throughput::Elements(1))),
    ]);
    check!(parse_measurement("test g/f ... bench: 1 ns/iter", &extra, None).is_err());
    Ok(())
}

#[test]
fn time_units_and_thousands_separators_convert_exactly() -> TestResult {
    for (raw, nanos) in [
        ("12,345 ns", 12_345.0),
        ("3 µs", 3_000.0),
        ("3 μs", 3_000.0),
        ("5 ms", 5_000_000.0),
        ("2.5 s", 2_500_000_000.0),
    ] {
        check!(eq;
            parse_bencher_line(&format!("test g/f ... bench: {raw}/iter"))?,
            ("g/f".into(), nanos)
        );
    }
    Ok(())
}

#[test]
fn invalid_time_and_extra_timing_fields_cannot_make_a_baseline() -> TestResult {
    for raw in [
        "0 ns",
        "-1 ns",
        "NaN ns",
        "inf ns",
        "1e309 s",
        "1 minute",
        "1 unexpected ns",
    ] {
        check!(parse_bencher_line(&format!("test g/f ... bench: {raw}/iter")).is_err());
    }
    Ok(())
}

#[test]
fn gnu_time_rss_parsing_accepts_indentation_and_rejects_missing_memory() -> TestResult {
    check!(eq;
        runtime::parse_rss("\tMaximum resident set size (kbytes): 128\n")?,
        128
    );
    for raw in [
        "",
        "Maximum resident set size (kbytes): 0",
        "Maximum resident set size (kbytes): unknown",
    ] {
        check!(runtime::parse_rss(raw).is_err());
    }
    Ok(())
}

#[test]
fn persisted_rate_cannot_compare_elements_with_bytes() -> TestResult {
    let old = tempfile::tempdir()?;
    let current = tempfile::tempdir()?;
    capture(
        old.path(),
        "g/f",
        r#"{"full_id":"g/f","throughput":{"Elements":100}}"#,
    )?;
    capture(
        current.path(),
        "g/f",
        r#"{"full_id":"g/f","throughput":{"Bytes":100}}"#,
    )?;
    let output = "test g/f ... bench: 1 s/iter (+/- 0)";
    let mut baseline = super::super::PerfBaseline {
        metadata: super::super::Meta {
            cpu: "test".into(),
            cores: 1,
            rustc: "test".into(),
            sha: "test".into(),
            corpus_lines: 1,
            corpus_sha256: "a".repeat(64),
        },
        check_reason: None,
        groups: parse_measurement(output, &metadata::read(old.path())?, None)?,
    };
    baseline.groups.insert(
        "pipeline/capture_export/captured_live_wiaa_co2027".into(),
        crate::perf::tests::capture_measurement(),
    );
    for measurement in baseline.groups.values_mut() {
        measurement.peak_rss_kib = Some(100);
        measurement.allocation_count = Some(100);
        measurement.allocated_bytes = Some(100);
        measurement.tail_time_seconds = Some(1.0);
        if measurement.timing_scope.is_none() {
            measurement.timing_scope = Some(crate::perf::scope::expected("g/f").to_owned());
        }
    }
    let baseline: super::super::PerfBaseline =
        serde_json::from_slice(&serde_json::to_vec(&baseline)?)?;
    let mut current = parse_measurement(output, &metadata::read(current.path())?, None)?;
    for measurement in current.values_mut() {
        measurement.peak_rss_kib = Some(100);
        measurement.allocation_count = Some(100);
        measurement.allocated_bytes = Some(100);
        measurement.tail_time_seconds = Some(1.0);
        measurement.timing_scope = Some(crate::perf::scope::expected("g/f").to_owned());
    }
    current.insert(
        "pipeline/capture_export/captured_live_wiaa_co2027".into(),
        crate::perf::tests::capture_measurement(),
    );
    let error = match super::super::compare::check_throughput(&baseline, &current, 0.05) {
        Err(error) => error,
        Ok(()) => return Err(anyhow::anyhow!("throughput unit change was accepted").into()),
    };
    check!(error.to_string().contains("throughput unit changed"));
    Ok(())
}

#[test]
#[cfg(unix)]
fn absent_timer_refuses_measurement_and_nonzero_processes_fail() -> TestResult {
    let directory = tempfile::tempdir()?;
    let absent = match runtime::measure_with_time(Path::new("/bin/true"), directory.path(), None) {
        Err(error) => error,
        Ok(_) => return Err(anyhow::anyhow!("missing timer was accepted").into()),
    };
    check!(absent.to_string().contains("GNU time is required"));
    let failure = match runtime::measure_with_time(
        Path::new("/bin/true"),
        directory.path(),
        Some(Path::new("/bin/false")),
    ) {
        Err(error) => error,
        Ok(_) => return Err(anyhow::anyhow!("process failure was accepted").into()),
    };
    check!(failure.to_string().contains("exit status: 1"));
    Ok(())
}
