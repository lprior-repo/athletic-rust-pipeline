mod harness_list;

use std::collections::HashMap;
use std::time::Instant;

use anyhow::{bail, Result};

use crate::cmd::Cmd;
use std::process::ExitStatus;

pub use harness_list::{HarnessInfo, KNOWN_HARNESS};

fn mandatory_harnesses() -> &'static [&'static str] {
    &[
        "check_fixed_point_bounds",
        "check_pr_comparison_laws",
        "check_identity_contradiction",
        "check_redirect_cycle",
        "check_retry_limit",
        "check_terminal_state_no_retry",
        "check_store_batch_arithmetic",
        "check_census_scope",
    ]
}

fn resolve_targets(user_harnesses: &[String]) -> Result<Vec<&HarnessInfo>> {
    let required = mandatory_harnesses();

    let by_name: HashMap<&str, &HarnessInfo> = KNOWN_HARNESS.iter().map(|h| (h.name, h)).collect();

    let targets = if user_harnesses.is_empty() {
        let mut missing = Vec::new();
        let mut targets = Vec::with_capacity(required.len());
        for &name in required {
            match by_name.get(name) {
                Some(&info) => targets.push(info),
                None => missing.push(name),
            }
        }
        if !missing.is_empty() {
            missing.sort();
            bail!("missing required harness(es): {}", missing.join(", "));
        }
        targets
    } else {
        let mut found = Vec::new();
        for name in user_harnesses {
            match by_name.get(name.as_str()) {
                Some(&info) => found.push(info),
                None => bail!(
                    "unknown harness: {name} (expected one of: {})",
                    KNOWN_HARNESS
                        .iter()
                        .map(|h| h.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            }
        }
        if found.is_empty() {
            bail!("no valid harnesses selected by user");
        }
        found
    };

    Ok(targets)
}

pub fn run(harnesses: &[String]) -> Result<()> {
    let targets = resolve_targets(harnesses)?;

    println!("cargo kani: {} harness(es)", targets.len());

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut timed_out = 0u32;

    for &harness in &targets {
        print!("  {}... ", harness.name);
        let start = Instant::now();
        let result = run_harness(harness);
        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                println!("PASS ({:.1}s)", elapsed.as_secs_f64());
                passed = passed.saturating_add(1);
            }
            Err(KaniError::Fail) => {
                println!("FAIL ({:.1}s)", elapsed.as_secs_f64());
                failed = failed.saturating_add(1);
            }
            Err(KaniError::Timeout) => {
                println!("TIMEOUT ({:.1}s)", elapsed.as_secs_f64());
                timed_out = timed_out.saturating_add(1);
            }
            Err(KaniError::Missing) => {
                println!("MISSING (harness not found in crate)");
                failed = failed.saturating_add(1);
            }
            Err(KaniError::BuildFail) => {
                println!("BUILD FAIL ({:.1}s)", elapsed.as_secs_f64());
                failed = failed.saturating_add(1);
            }
        }
    }

    println!(
        "\nresult: {} passed, {} failed, {} timeout",
        passed, failed, timed_out
    );

    if failed > 0 || timed_out > 0 {
        bail!(
            "{} harness(es) did not verify",
            failed.saturating_add(timed_out)
        );
    }

    Ok(())
}

#[derive(Debug)]
enum KaniError {
    Fail,
    Timeout,
    Missing,
    BuildFail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail,
    Timeout,
    Missing,
    BuildFail,
}

pub fn classify_kani_output(stdout: &str, stderr: &str) -> Outcome {
    let combined = format!("{stdout}\n{stderr}");
    if combined.contains("Timed out") {
        return Outcome::Timeout;
    }
    if combined.contains("Error: no harness found")
        || combined.contains("could not find harness")
        || combined.contains("no harness")
    {
        return Outcome::Missing;
    }
    if combined.contains("VERIFICATION:- FAILED") {
        return Outcome::Fail;
    }
    if combined.contains("CBMC failed")
        || combined.lines().any(|line| {
            let line = line.trim_start();
            line.starts_with("Error:") || line.starts_with("error:")
        })
    {
        return Outcome::BuildFail;
    }
    let Some(summary) = parse_complete_line(&combined) else {
        return Outcome::BuildFail;
    };
    if summary.failures > 0 {
        return Outcome::Fail;
    }
    if summary.verified == 0 || !combined.contains("VERIFICATION:- SUCCESSFUL") {
        return Outcome::BuildFail;
    }
    if summary.verified != summary.total {
        return Outcome::Fail;
    }
    if summary.total != 1 {
        return Outcome::BuildFail;
    }
    Outcome::Pass
}

fn classify_kani_process(status: ExitStatus, stdout: &str, stderr: &str) -> Outcome {
    match classify_kani_output(stdout, stderr) {
        Outcome::Pass if !status.success() => Outcome::BuildFail,
        outcome => outcome,
    }
}

#[derive(Debug, Clone, Copy)]
struct CompleteSummary {
    verified: u32,
    failures: u32,
    total: u32,
}

fn parse_complete_line(output: &str) -> Option<CompleteSummary> {
    let mut summaries = output
        .lines()
        .filter_map(|line| line.trim().strip_prefix("Complete - "));
    let mut parts = summaries.next()?.split(", ");
    if summaries.next().is_some() {
        return None;
    }
    let verified = parts
        .next()?
        .strip_suffix(" successfully verified harnesses")?
        .parse()
        .ok()?;
    let failures = parts.next()?.strip_suffix(" failures")?.parse().ok()?;
    let total = parts.next()?.strip_suffix(" total.")?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(CompleteSummary {
        verified,
        failures,
        total,
    })
}

fn run_harness(harness: &HarnessInfo) -> std::result::Result<(), KaniError> {
    let (status, stdout, stderr) = Cmd::new("cargo")
        .arg("kani")
        .arg("--manifest-path")
        .arg(harness.manifest_path)
        .arg("--harness")
        .arg(harness.name)
        .arg("-j")
        .arg("1")
        .capture()
        .map_err(|_| KaniError::BuildFail)?;
    let outcome = classify_kani_process(status, &stdout, &stderr);
    print!("{stdout}");
    eprint!("{stderr}");
    match outcome {
        Outcome::Pass => Ok(()),
        Outcome::Timeout => Err(KaniError::Timeout),
        Outcome::Fail => Err(KaniError::Fail),
        Outcome::Missing => Err(KaniError::Missing),
        Outcome::BuildFail => Err(KaniError::BuildFail),
    }
}

#[cfg(test)]
mod kani_tests;
