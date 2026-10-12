use super::super::Throughput;
use super::{metadata, parse_bencher_line, parse_measurement, read_trial, runtime};
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

fn capture_tail(root: &Path, directory: &str, id: &str, nanos: f64) -> Result<()> {
    let directory = root.join(directory).join("new");
    std::fs::create_dir_all(&directory)?;
    std::fs::write(
        directory.join("benchmark.json"),
        format!(r#"{{"full_id":"{id}","throughput":{{"Elements":100}}}}"#),
    )?;
    std::fs::write(
        directory.join("sample.json"),
        format!(r#"{{"iters":[1],"times":[{nanos}]}}"#),
    )?;
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
    let tails = BTreeMap::from([
        ("group/first".to_owned(), 1.0),
        ("group/second".to_owned(), 2.0),
    ]);
    let first_line = "test group/first ... bench: 1 s/iter (+/- 0)";
    let second_line = "test group/second ... bench: 2 s/iter (+/- 0)";
    let output = format!("{first_line}\n{second_line}");
    let result = parse_measurement(&output, &declarations, &tails)?;
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
    check!(eq; first.tail_time_seconds, Some(1.0));
    check!(eq; second.tail_time_seconds, Some(2.0));
    Ok(())
}

#[test]
fn trial_stdout_is_parsed_against_its_own_criterion_root() -> TestResult {
    let root = tempfile::tempdir()?;
    capture_tail(root.path(), "run-0/group/first", "group/first", 1e9)?;
    capture_tail(root.path(), "run-1/group/first", "group/first", 9e9)?;
    let trial = runtime::Trial {
        output: "test group/first ... bench: 2 s/iter (+/- 0)".to_owned(),
        criterion: root.path().join("run-1"),
    };
    let parsed = read_trial("core", &trial)?;
    let measurement = parsed
        .get("group/first")
        .ok_or_else(|| anyhow::anyhow!("group/first was not parsed"))?;
    check!(eq; measurement.tail_time_seconds, Some(9.0));
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
fn metadata_output_and_tail_samples_must_describe_the_same_nonempty_results() -> TestResult {
    let declared = BTreeMap::from([("g/f".to_owned(), Some(Throughput::Elements(1)))]);
    let tails = BTreeMap::from([("g/f".to_owned(), 1.0)]);
    for output in [
        "",
        "test g/other ... bench: 1 ns/iter",
        "test g/f ... bench: 1 ns/iter\ntest g/f ... bench: 2 ns/iter",
    ] {
        check!(parse_measurement(output, &declared, &tails).is_err());
    }
    let no_tails = BTreeMap::new();
    check!(parse_measurement("test g/f ... bench: 1 ns/iter", &declared, &no_tails).is_err());
    let extra_tails = BTreeMap::from([("g/f".to_owned(), 1.0), ("g/other".to_owned(), 1.0)]);
    check!(parse_measurement("test g/f ... bench: 1 ns/iter", &declared, &extra_tails).is_err());
    let extra = BTreeMap::from([
        ("g/f".to_owned(), Some(Throughput::Elements(1))),
        ("g/other".to_owned(), Some(Throughput::Elements(1))),
    ]);
    check!(parse_measurement("test g/f ... bench: 1 ns/iter", &extra, &tails).is_err());
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
    let current_root = tempfile::tempdir()?;
    capture(
        old.path(),
        "g/f",
        r#"{"full_id":"g/f","throughput":{"Elements":100}}"#,
    )?;
    capture(
        current_root.path(),
        "g/f",
        r#"{"full_id":"g/f","throughput":{"Bytes":100}}"#,
    )?;
    let tails = BTreeMap::from([("g/f".to_owned(), 1.0)]);
    let output = "test g/f ... bench: 1 s/iter (+/- 0)";
    let mut recorded = super::super::PerfBaseline {
        metadata: super::super::Meta {
            cpu: "test".into(),
            cores: 1,
            rustc: "test".into(),
            sha: "test".into(),
            corpus_lines: 1,
            corpus_sha256: "a".repeat(64),
        },
        check_reason: None,
        groups: parse_measurement(output, &metadata::read(old.path())?, &tails)?,
    };
    let mut measured = parse_measurement(output, &metadata::read(current_root.path())?, &tails)?;
    for groups in [&mut recorded.groups, &mut measured] {
        groups.insert(
            crate::perf::compare::CAPTURE_EXPORT_WORKLOAD.to_owned(),
            crate::perf::tests::capture_measurement(),
        );
        for measurement in groups.values_mut() {
            measurement.peak_rss_kib = Some(100);
            measurement.allocation_count = Some(100);
            measurement.allocated_bytes = Some(100);
            measurement.tail_time_seconds = Some(1.0);
            if measurement.timing_scope.is_none() {
                measurement.timing_scope = Some(crate::perf::scope::expected("g/f").to_owned());
            }
        }
    }
    let baseline: super::super::PerfBaseline =
        serde_json::from_slice(&serde_json::to_vec(&recorded)?)?;
    let error = match super::super::compare::check_throughput(&baseline, &measured, 0.05) {
        Err(error) => error,
        Ok(()) => return Err(anyhow::anyhow!("throughput unit change was accepted").into()),
    };
    check!(error.to_string().contains("throughput unit changed"));
    Ok(())
}
