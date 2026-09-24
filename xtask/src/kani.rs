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

use std::time::Instant;

use anyhow::{bail, Result};

use crate::cmd::Cmd;

/// Names of the mandatory harnesses that `cargo kani` must verify.
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

/// Run `cargo kani` and parse its output.
///
/// `cargo kani --harness <name>` compiles the harness to CBMC, verifies invariants, and prints
/// results. Each harness either passes (all checks verified), fails (assertion violated), or
/// times out (CBMC search exhausted).
pub fn run(harnesses: &[String]) -> Result<()> {
    let all = mandatory_harnesses();
    let targets: Vec<&str> = if harnesses.is_empty() {
        all.to_vec()
    } else {
        let mut found = Vec::new();
        for name in harnesses {
            if all.contains(&name.as_str()) {
                found.push(name.as_str());
            } else {
                eprintln!(
                    "unknown harness: {name} (expected one of: {})",
                    all.join(", ")
                );
                for h in all {
                    eprintln!("  {h}");
                }
                std::process::exit(1);
            }
        }
        if found.is_empty() {
            bail!("no valid harness names provided");
        }
        found
    };

    let mut passed = 0u32;
    let mut failed = 0u32;
    let mut timed_out = 0u32;
    let total = targets.len();

    println!("cargo kani: {} harness(es)", total);

    for harness in &targets {
        print!("  {harness}... ");
        let start = Instant::now();
        let result = run_harness(harness);
        let elapsed = start.elapsed();

        match result {
            Ok(_) => {
                println!("PASS ({:.1}s)", elapsed.as_secs_f64());
                passed += 1;
            }
            Err(KaniError::Fail) => {
                println!("FAIL ({:.1}s)", elapsed.as_secs_f64());
                failed += 1;
            }
            Err(KaniError::Timeout) => {
                println!("TIMEOUT ({:.1}s)", elapsed.as_secs_f64());
                timed_out += 1;
            }
            Err(KaniError::Missing) => {
                println!("MISSING (harness not found in crate)");
                failed += 1;
            }
        }
    }

    println!(
        "\nresult: {} passed, {} failed, {} timeout",
        passed, failed, timed_out
    );

    if failed > 0 || timed_out > 0 {
        bail!("{} harness(es) did not verify", failed + timed_out);
    }

    Ok(())
}

#[derive(Debug)]
enum KaniError {
    Fail,
    Timeout,
    Missing,
}

/// Run one harness via `cargo kani --harness <name>` and parse its output.
///
/// `cargo kani` prints verification results to stderr. A passing harness ends with
/// "Verification Time: ..." and "All checks were verified". A failing harness prints
/// "Fails: X" where X > 0. A timeout prints "Timed out".
fn run_harness(name: &str) -> std::result::Result<(), KaniError> {
    let output = Cmd::new("cargo")
        .arg("kani")
        .arg("--harness")
        .arg(name)
        .arg("-j")
        .arg("1")
        .output()
        .map_err(|_| KaniError::Fail)?;

    if output.contains("All checks were verified") {
        Ok(())
    } else if output.contains("Timed out") {
        Err(KaniError::Timeout)
    } else if output.contains("Fails:") && !output.contains("0") {
        Err(KaniError::Fail)
    } else if output.contains("Error: no harness found")
        || output.contains("no harness")
        || output.contains("could not find harness")
    {
        Err(KaniError::Missing)
    } else {
        // If CBMC compiled but didn't produce expected output, treat as failure
        Err(KaniError::Fail)
    }
}
