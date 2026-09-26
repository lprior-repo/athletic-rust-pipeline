//! `cargo kani`: verify `census-domain` invariants that unit tests cannot express.
//!
//! Mandatory harnesses:
//! - fixed-point conversion bounds (cents never exceed i64::MAX, meters never underflow)
//! - PR comparison laws (lower time wins, higher distance wins)
//! - hard identity contradiction cannot match
//! - canonical redirect cannot self-loop or cycle
//! - retry attempts <= 3
//! - terminal state cannot retry
//! - StoreBatch row/sequence arithmetic
//! - CENSUS_SCOPE exactly 49
//!
//! Output: PASS / FAIL / TIMEOUT per harness, exit nonzero on FAIL (TIMEOUT counts as FAIL).

mod harness_list;

use std::collections::HashMap;
use std::time::Instant;

use anyhow::{bail, Result};

use crate::cmd::Cmd;

pub use harness_list::{HarnessInfo, KNOWN_HARNESS};

/// Names of the mandatory harnesses that `cargo kani` must verify (§21.1).
///
/// These are the contract's required kernel names. A name here that does not exist
/// in `KNOWN_HARNESS` is a hard failure reported before any harness runs.
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

/// Resolve which harnesses to run: all mandatory ones, or validate user-provided names.
///
/// Fails hard if any required harness is missing from the known set — this is a pre-execution
/// check so we never silently run zero harnesses and report success.
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

/// Run `cargo kani` and parse its output.
///
/// `cargo kani --harness <name>` compiles the harness to CBMC, verifies invariants, and prints
/// results. Each harness either passes (all checks verified), fails (assertion violated), or
/// times out (CBMC search exhausted).
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
    /// A verification check failed (assertion violated).
    Fail,
    /// CBMC search exhausted (timeout).
    Timeout,
    /// The harness was not found (should be caught by resolve_targets, but kept for safety).
    Missing,
    /// The command itself failed (non-zero exit, build error).
    BuildFail,
}

/// Classification of a single harness output tail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    /// All verification checks passed.
    Pass,
    /// One or more checks failed.
    Fail,
    /// CBMC timed out.
    Timeout,
    /// Harness was not found.
    Missing,
    /// Build or command error (non-zero exit).
    BuildFail,
}

/// Classify a combined stdout+stderr output tail from `cargo kani --harness <name>`.
///
/// cargo-kani 0.67.0 writes its verdict to stderr. A passing harness ends with
/// `VERIFICATION:- SUCCESSFUL` and `Complete - N successfully verified harnesses`.
/// A failing harness prints `VERIFICATION:- FAILED`. A timeout prints `Timed out`.
///
/// Check order matters: verdict indicators (`VERIFICATION:- SUCCESSFUL/FAILED`) are checked
/// before error heuristics (`CBMC failed`) because real Kani output includes both — the
/// verdict is the ground truth.
pub fn classify_kani_output(stdout: &str, stderr: &str) -> Outcome {
    let combined = format!("{stdout}{stderr}");

    if combined.contains("Timed out") {
        return Outcome::Timeout;
    }

    if combined.contains("VERIFICATION:- SUCCESSFUL")
        && combined.contains("successfully verified harnesses")
    {
        if let Some((_, tail)) = combined.split_once("Complete - ") {
            let mut words = tail.split_whitespace();
            if let Some(count_str) = words.next() {
                if let Ok(count) = count_str.parse::<u32>() {
                    if count > 0 {
                        return Outcome::Pass;
                    }
                }
            }
        }
    }

    if combined.contains("VERIFICATION:- FAILED") {
        return Outcome::Fail;
    }

    if combined.contains("All checks were verified") {
        return Outcome::Pass;
    }

    if combined.contains("Error: no harness found")
        || combined.contains("could not find harness")
        || combined.contains("no harness")
    {
        return Outcome::Missing;
    }

    if combined.contains("CBMC failed") || combined.contains("Error: ") {
        return Outcome::BuildFail;
    }

    Outcome::BuildFail
}

/// Run one harness via `cargo kani --manifest-path <path> --harness <name>`.
///
/// Package-qualified invocation (the way docs/VERIFICATION-EVIDENCE.md:55-63 documents it)
/// so the runner knows exactly which package's harness to compile.
///
/// Kani writes its results to stderr, so we capture both streams.
fn run_harness(harness: &HarnessInfo) -> std::result::Result<(), KaniError> {
    let (stdout, stderr) = Cmd::new("cargo")
        .arg("kani")
        .arg("--manifest-path")
        .arg(harness.manifest_path)
        .arg("--harness")
        .arg(harness.name)
        .arg("-j")
        .arg("1")
        .capture()
        .map_err(|_| KaniError::BuildFail)?;

    match classify_kani_output(&stdout, &stderr) {
        Outcome::Pass => Ok(()),
        Outcome::Timeout => Err(KaniError::Timeout),
        Outcome::Fail => Err(KaniError::Fail),
        Outcome::Missing => Err(KaniError::Missing),
        Outcome::BuildFail => Err(KaniError::BuildFail),
    }
}

#[cfg(test)]
mod kani_tests;
