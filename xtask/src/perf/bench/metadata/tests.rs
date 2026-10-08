use super::read;
use crate::perf::Throughput;
use anyhow::{anyhow, Result};
use std::collections::BTreeMap;

#[test]
fn missing_or_null_declared_throughput_refuses_metadata_collection() -> Result<()> {
    for json in [
        r#"{"full_id":"group/benchmark"}"#,
        r#"{"full_id":"group/benchmark","throughput":null}"#,
    ] {
        let root = tempfile::tempdir()?;
        let directory = root.path().join("group/benchmark/new");
        std::fs::create_dir_all(&directory)?;
        std::fs::write(directory.join("benchmark.json"), json)?;
        let error = read(root.path())
            .err()
            .ok_or_else(|| anyhow!("missing declared throughput was accepted"))?;
        check!(format!("{error:#}").contains("throughput"));
        check!(format!("{error:#}").contains("group/benchmark"));
    }
    Ok(())
}

#[test]
fn missing_or_zero_declared_throughput_refuses_measurement_collection() -> Result<()> {
    for amount in [
        None,
        Some(Throughput::Elements(0)),
        Some(Throughput::Bytes(0)),
    ] {
        let declarations = BTreeMap::from([("group/benchmark".into(), amount)]);
        let error = super::super::parse_measurement(
            "test group/benchmark ... bench: 1 s/iter",
            &declarations,
            Some(100),
        )
        .err()
        .ok_or_else(|| anyhow!("missing or zero declared throughput was measured"))?;
        check!(format!("{error:#}").contains("throughput"));
    }
    Ok(())
}

#[test]
fn invalid_declared_throughput_refuses_metadata_collection() -> Result<()> {
    for raw in [
        r#"{"Elements":0}"#,
        r#"{"Bytes":0}"#,
        r#"{"Elements":-1}"#,
        r#"{"Elements":1e309}"#,
        r#"{"Elements":"NaN"}"#,
    ] {
        let root = tempfile::tempdir()?;
        let directory = root.path().join("group/benchmark/new");
        std::fs::create_dir_all(&directory)?;
        let json = format!(r#"{{"full_id":"group/benchmark","throughput":{raw}}}"#);
        std::fs::write(directory.join("benchmark.json"), json)?;
        let error = read(root.path())
            .err()
            .ok_or_else(|| anyhow!("invalid declared throughput was accepted: {raw}"))?;
        check!(format!("{error:#}").contains("Criterion"));
    }
    Ok(())
}
