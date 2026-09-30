use super::super::Throughput;
use super::{metadata, parse_bencher_line, parse_measurement, runtime};
use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

fn capture(root: &Path, directory: &str, json: &str) -> Result<()> {
    let directory = root.join(directory).join("new");
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join("benchmark.json"), json)?;
    Ok(())
}

#[test]
fn criterion_full_ids_preserve_distinct_per_benchmark_throughputs() -> Result<()> {
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
    capture(
        root.path(),
        "group/timing",
        r#"{"full_id":"group/timing","throughput":null}"#,
    )?;
    let declarations = metadata::read(root.path())?;
    let output = "test group/first ... bench: 1 s/iter (+/- 0)\ntest group/second ... bench: 2 s/iter (+/- 0)\ntest group/timing ... bench: 500 ms/iter (+/- 0)";
    let result = parse_measurement(output, &declarations, None)?;
    assert_eq!(
        result["group/first"].throughput,
        Some(Throughput::Elements(42.0))
    );
    assert_eq!(
        result["group/second"].throughput,
        Some(Throughput::Bytes(512.0))
    );
    assert_eq!(result["group/timing"].throughput, None);
    assert_eq!(result["group/timing"].wall_time_seconds, 0.5);
    Ok(())
}

#[test]
fn missing_duplicate_empty_and_malformed_metadata_refuse_collection() -> Result<()> {
    for json in [
        "{",
        r#"{"full_id":"","throughput":null}"#,
        r#"{"full_id":"g/f","throughput":{"Elements":0}}"#,
    ] {
        let root = tempfile::tempdir()?;
        capture(root.path(), "group/first", json)?;
        assert!(metadata::read(root.path()).is_err());
    }
    let root = tempfile::tempdir()?;
    assert!(metadata::read(root.path()).is_err());
    capture(
        root.path(),
        "first",
        r#"{"full_id":"g/f","throughput":null}"#,
    )?;
    capture(
        root.path(),
        "second",
        r#"{"full_id":"g/f","throughput":null}"#,
    )?;
    assert!(metadata::read(root.path()).is_err());
    Ok(())
}

#[test]
fn metadata_and_output_must_describe_the_same_nonempty_results() {
    let declared = BTreeMap::from([("g/f".into(), Some(Throughput::Elements(1)))]);
    for output in [
        "",
        "test g/other ... bench: 1 ns/iter",
        "test g/f ... bench: 1 ns/iter\ntest g/f ... bench: 2 ns/iter",
    ] {
        assert!(parse_measurement(output, &declared, None).is_err());
    }
    let extra = BTreeMap::from([
        ("g/f".into(), Some(Throughput::Elements(1))),
        ("g/other".into(), Some(Throughput::Elements(1))),
    ]);
    assert!(parse_measurement("test g/f ... bench: 1 ns/iter", &extra, None).is_err());
}

#[test]
fn time_units_and_thousands_separators_convert_exactly() -> Result<()> {
    for (raw, nanos) in [
        ("12,345 ns", 12_345.0),
        ("3 µs", 3_000.0),
        ("3 μs", 3_000.0),
        ("5 ms", 5_000_000.0),
        ("2.5 s", 2_500_000_000.0),
    ] {
        assert_eq!(
            parse_bencher_line(&format!("test g/f ... bench: {raw}/iter"))?,
            ("g/f".into(), nanos)
        );
    }
    Ok(())
}

#[test]
fn invalid_time_and_extra_timing_fields_cannot_make_a_baseline() {
    for raw in [
        "0 ns",
        "-1 ns",
        "NaN ns",
        "inf ns",
        "1e309 s",
        "1 minute",
        "1 unexpected ns",
    ] {
        assert!(parse_bencher_line(&format!("test g/f ... bench: {raw}/iter")).is_err());
    }
}

#[test]
fn gnu_time_rss_parsing_accepts_indentation_and_rejects_missing_memory() -> Result<()> {
    assert_eq!(
        runtime::parse_rss("\tMaximum resident set size (kbytes): 128\n")?,
        128
    );
    for raw in [
        "",
        "Maximum resident set size (kbytes): 0",
        "Maximum resident set size (kbytes): unknown",
    ] {
        assert!(runtime::parse_rss(raw).is_err());
    }
    Ok(())
}

#[test]
fn persisted_rate_cannot_compare_elements_with_bytes() -> Result<()> {
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
    let baseline = super::super::PerfBaseline {
        metadata: super::super::Meta {
            cpu: "test".into(),
            cores: 1,
            rustc: "test".into(),
            sha: "test".into(),
            corpus_lines: 1,
        },
        check_reason: None,
        groups: parse_measurement(output, &metadata::read(old.path())?, None)?,
    };
    let baseline: super::super::PerfBaseline =
        serde_json::from_slice(&serde_json::to_vec(&baseline)?)?;
    let current = parse_measurement(output, &metadata::read(current.path())?, None)?;
    assert!(super::super::compare::check_throughput(&baseline, &current, 0.05).is_err());
    Ok(())
}

#[test]
#[cfg(unix)]
fn absent_timer_keeps_rss_unavailable_and_nonzero_children_fail() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let (_, rss) = runtime::measure_with_time(Path::new("/bin/true"), directory.path(), None)?;
    assert_eq!(rss, None);
    let failure = runtime::measure_with_time(Path::new("/bin/false"), directory.path(), None)
        .expect_err("benchmark process failure must be propagated");
    assert!(failure.to_string().contains("exit status: 1"), "{failure}");
    Ok(())
}
