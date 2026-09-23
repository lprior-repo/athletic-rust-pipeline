//! `contract`: the architectural constants other work relies on, asserted as checks.
//!
//! Each check is one claim a reader — or an agent — is entitled to rely on without re-deriving it, and
//! each is measured against the live tree rather than a copy of it: the enum the domain declares, the
//! specs this build registers, the files in this repository, the package set `cargo metadata` answers
//! with. A check that holds prints what it measured, because a passing check with nothing behind it
//! is a claim rather than evidence; a check that fails prints one line per violation and makes the
//! process exit non-zero.
//!
//! Three verdicts, not two. **PASS** is the claim holding. **FAIL** is a claim the tree contradicts.
//! **KNOWN DEVIATION** is a claim the tree contradicts *today* for a reason a named downstream change
//! owns: it prints the claim, the reason and the current value, and it does not fail the process —
//! because a gate that fails on a migration that has not landed trains its readers to ignore it. The
//! deviation lives in a table beside the check that reports it ([`registry::DEVIATIONS`]), so a
//! resolved deviation is a visible edit rather than a silent disappearance, and anything outside that
//! table fails.
//!
//! The eight checks, in the order they print:
//!
//! 1. `census_domain::UsJurisdiction::CENSUS_SCOPE` is the contiguous market: `ALL` minus
//!    `EXCLUDED_FROM_CENSUS`, name-for-name and in that order, so a substitution or a reordering is a
//!    violation rather than a cosmetic edit.
//! 2. The `athleticnet` source is registered with a browser transport (a known deviation until the
//!    browser migration lands).
//! 3. Every retry ceiling holds: 3 for a handler attribute, 1 for an inner policy (see [`crate::retry`]).
//! 4. No `.py` file exists in the tree outside `target/`, `.git/` and `var/`.
//! 5. Every registered descriptor admits a host origin, a positive rate and at least one in-flight
//!    request.
//! 6. The scanned package set equals the workspace's own (`cargo metadata`), so widening the
//!    workspace cannot silently escape the scan.
//! 7. Every document the repository's front doors name exists.
//! 8. Every module under `sources/` is either a registered source or a listed reader, and every
//!    registered source has a module, so the registry and the adapter tree describe one set.
//!
//! Checks 1, 2 and 5 are model-level claims: they read the declarations and the registry, not the
//! files, so they hold or fail independently of how the crate is split into modules.

use anyhow::Result;

mod checks;
mod docs;
mod registry;
mod tree;

/// One asserted constant: what was measured, and every reason the assertion does not hold.
pub(crate) struct Check {
    number: usize,
    name: &'static str,
    /// The measurement the verdict is made of, printed whatever the verdict is.
    detail: String,
    /// One line per violation; empty when the check holds.
    failures: Vec<String>,
    /// The line naming a contradiction a downstream change owns, when there is one.
    deviation: Option<String>,
}

impl Check {
    /// A check that holds, with the measurement that says so.
    pub(crate) fn holds(number: usize, name: &'static str, detail: String) -> Self {
        Self {
            number,
            name,
            detail,
            failures: Vec::new(),
            deviation: None,
        }
    }

    /// A check that does not hold, with one line per violation.
    pub(crate) fn violated(
        number: usize,
        name: &'static str,
        detail: String,
        failures: Vec<String>,
    ) -> Self {
        Self {
            number,
            name,
            detail,
            failures,
            deviation: None,
        }
    }

    /// A check the tree contradicts today, for a reason the deviation table records.
    pub(crate) fn deviates(
        number: usize,
        name: &'static str,
        detail: String,
        deviation: String,
    ) -> Self {
        Self {
            number,
            name,
            detail,
            failures: Vec::new(),
            deviation: Some(deviation),
        }
    }
}

/// Run every check and print the verdict, failing the process when any of them is violated.
pub(crate) fn run() -> Result<()> {
    let checks = checks::all();
    println!("contract: {} architectural checks", checks.len());
    let mut violated = 0usize;
    let mut deviated = 0usize;
    for check in &checks {
        if !check.failures.is_empty() {
            violated = violated.saturating_add(1);
            println!(
                "  check {} {}: FAIL ({})",
                check.number, check.name, check.detail
            );
            for failure in &check.failures {
                println!("    {failure}");
            }
            continue;
        }
        if let Some(deviation) = &check.deviation {
            deviated = deviated.saturating_add(1);
            println!(
                "  check {} {}: KNOWN DEVIATION ({})",
                check.number, check.name, check.detail
            );
            println!("    {deviation}");
            continue;
        }
        println!(
            "  check {} {}: PASS ({})",
            check.number, check.name, check.detail
        );
    }
    if violated == 0 {
        println!("contract: PASS ({deviated} known deviation(s))");
        return Ok(());
    }
    anyhow::bail!(
        "{violated} of {} architectural checks are violated, {deviated} known deviation(s)",
        checks.len()
    )
}
