use super::config::{self, Manifest};
use anyhow::{ensure, Context, Result};
use serde_json::json;
use std::path::Path;
use std::time::Duration;

#[path = "measure/process.rs"]
mod process;

pub fn run(root: &Path, out: &Path, seconds: u64) -> Result<()> {
    ensure!(
        (1..=86_400).contains(&seconds),
        "timeout outside 1..=86400 seconds"
    );
    let fixture = Manifest::read(root)?;
    let executable = std::env::current_exe()?;
    let binary_sha256 = config::file_digest(&executable)?;
    config::fresh_output(root, out)?;
    let result = process::run(
        &executable,
        root,
        &out.join("publication"),
        Duration::from_secs(seconds),
    )?;
    config::write_new(&out.join("child.stdout"), &result.stdout)?;
    config::write_new(&out.join("time-and-child.stderr"), &result.stderr)?;
    let raw = String::from_utf8_lossy(&result.stderr);
    let rss = rss(&raw);
    let child_verification = if result.success
        && !result.timed_out
        && !result.output_exceeded
        && result.cleanup_error.is_none()
    {
        child_result(out, &fixture.fixture_sha256)
    } else {
        Err(anyhow::anyhow!(
            "child did not complete successfully within limits"
        ))
    };
    let succeeded = child_verification.is_ok() && rss.is_some();
    let summary = json!({
        "schema": 1, "fixture_version": fixture.version, "fixture_sha256": fixture.fixture_sha256,
        "fixture_config": fixture.config, "expected_counts": fixture.counts,
        "binary": executable, "binary_sha256": binary_sha256,
        "platform": std::env::consts::OS, "architecture": std::env::consts::ARCH,
        "whole_publication_peak_rss": rss,
        "rss_unit_validation": if cfg!(target_os = "macos") { "pending_native_mac_unit_control; raw value only" } else { "unit printed by GNU time; not Mac qualification" },
        "measured_process": "prebuilt publish child under /usr/bin/time; compiler, generator and measurement parent excluded",
        "wall_seconds": result.elapsed_seconds, "timeout_seconds": seconds,
        "exit_code": result.code, "timed_out": result.timed_out, "interruption": result.interruption,
        "output_exceeded": result.output_exceeded,
        "captured_output_limit_bytes_per_stream": process::OUTPUT_LIMIT,
        "cleanup_error": result.cleanup_error,
        "cleanup_policy": "parent INT/TERM handled before child spawn; TERM only; bounded reap and pipe drain; refusal may leave child or descendants alive",
        "child_result_error": child_verification.as_ref().err().map(ToString::to_string),
        "passed": succeeded, "synthetic_only": true, "census_acceptance": false,
        "historical_64gb_issue_solved": false,
    });
    config::write_new(
        &out.join("measurement.json"),
        &serde_json::to_vec_pretty(&summary)?,
    )?;
    println!("{}", serde_json::to_string(&summary)?);
    ensure!(
        succeeded,
        "publication qualification failed; retained bounded outputs in {}",
        out.display()
    );
    Ok(())
}

fn child_result(out: &Path, fixture: &str) -> Result<()> {
    let result = config::bounded_read(
        &out.join("publication/publication-result.json"),
        config::MANIFEST_LIMIT,
    )?;
    let result: serde_json::Value = serde_json::from_slice(&result)?;
    let child_fixture = result
        .get("fixture_sha256")
        .and_then(serde_json::Value::as_str)
        .context("child fixture identity missing")?;
    ensure!(child_fixture == fixture, "child fixture identity mismatch");
    Ok(())
}

fn rss(raw: &str) -> Option<serde_json::Value> {
    for line in raw.lines() {
        if let Some(value) = line.trim().strip_suffix(" maximum resident set size") {
            if let Ok(value) = value.trim().parse::<u64>() {
                return Some(
                    json!({"value": value, "unit": "unverified_native_time_unit", "raw_line": line.trim()}),
                );
            }
        }
        if let Some((_, value)) = line.split_once("Maximum resident set size (kbytes):") {
            if let Ok(value) = value.trim().parse::<u64>() {
                return Some(
                    json!({"value": value, "unit": "kbytes_as_reported", "raw_line": line.trim()}),
                );
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    #[test]
    fn native_units_remain_unverified_and_missing_measurements_refuse() -> Result<()> {
        let mac = rss(" 12345  maximum resident set size\n").context("mac sample")?;
        ensure!(mac["value"] == 12345 && mac["unit"] == "unverified_native_time_unit");
        let linux = rss(" Maximum resident set size (kbytes): 6789").context("GNU sample")?;
        ensure!(linux["value"] == 6789 && linux["unit"] == "kbytes_as_reported");
        ensure!(rss("elapsed only").is_none());
        Ok(())
    }
}
