#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

use std::path::Path;
use std::process::{Command, Output};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CUTOFF_REFUSAL: &str = "Error: standalone run does not support --as-of: retained-data publication has no performance cutoff";
const GATHER_REFUSAL: &str = "Error: the gather stage writes the store the running service holds: --input and --meets need --store";
const GATHER_OPTIONS: [&[&str]; 4] = [
    &[],
    &["--input", "missing-registry.csv"],
    &["--meets", "123"],
    &["--input", "missing-registry.csv", "--meets", "123"],
];

fn run(root: &Path, options: &[&str], gather: &[&str]) -> TestResult<Output> {
    Ok(Command::new(env!("CARGO_BIN_EXE_census-service"))
        .current_dir(root)
        .env("RUST_LOG", "off")
        .env("RUST_BACKTRACE", "0")
        .env("RUST_LIB_BACKTRACE", "0")
        .args(["run", "--school-year", "2026"])
        .args(options)
        .args(gather)
        .output()?)
}

fn check_refusal(output: &Output) -> TestResult {
    check!(eq; output.status.code(), Some(1));
    check!(eq; output.stdout.as_slice(), b"");
    check!(eq; std::str::from_utf8(&output.stderr)?.trim(), CUTOFF_REFUSAL);
    Ok(())
}

fn check_empty(root: &Path) -> TestResult {
    let entries = std::fs::read_dir(root)?.collect::<std::io::Result<Vec<_>>>()?;
    check!(eq; entries.len(), 0);
    Ok(())
}

#[test]
fn explicit_live_cutoff_refuses_before_ingress_validation_or_gather() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let output = run(
            directory.path(),
            &[
                "--as-of",
                "2026-10-14",
                "--ingress",
                "not-an-ingress-origin",
            ],
            gather,
        )?;
        check_refusal(&output)?;
        check_empty(directory.path())?;
    }
    Ok(())
}

#[test]
fn default_live_cutoff_refuses_before_generation_resolution_or_gather() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let output = run(
            directory.path(),
            &["--as-of", "2026-10-14", "--generation", "invalid:fixture"],
            gather,
        )?;
        check_refusal(&output)?;
        check_empty(directory.path())?;
    }
    Ok(())
}

#[test]
fn offline_cutoff_refuses_before_creating_a_store_or_gathering() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let output = run(
            directory.path(),
            &[
                "--store",
                "fixture-store",
                "--as-of",
                "2026-10-14",
                "--grad-year",
                "65535",
            ],
            gather,
        )?;
        check_refusal(&output)?;
        check_empty(directory.path())?;
    }
    Ok(())
}

#[test]
fn offline_cutoff_refuses_before_opening_an_existing_store_path() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("fixture-store");
        std::fs::create_dir(&root)?;
        let sentinel = root.join("retained-evidence");
        std::fs::write(&sentinel, b"preserve existing bytes")?;
        let output = run(
            directory.path(),
            &[
                "--store",
                "fixture-store",
                "--as-of",
                "2026-10-14",
                "--grad-year",
                "65535",
            ],
            gather,
        )?;
        check_refusal(&output)?;
        check!(eq; std::fs::read(&sentinel)?, b"preserve existing bytes");
        let entries = std::fs::read_dir(&root)?.collect::<std::io::Result<Vec<_>>>()?;
        check!(eq; entries.len(), 1);
        check!(!directory.path().join("var").try_exists()?);
    }
    Ok(())
}

#[test]
fn explicit_live_without_cutoff_retains_ingress_and_gather_validation() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let output = run(
            directory.path(),
            &[
                "--generation",
                "qualified-v1",
                "--ingress",
                "not-an-ingress-origin",
            ],
            gather,
        )?;
        check!(eq; output.status.code(), Some(1));
        let expected = if gather.is_empty() {
            "Error: relative URL without a base"
        } else {
            GATHER_REFUSAL
        };
        check!(eq; std::str::from_utf8(&output.stderr)?.trim(), expected);
        check_empty(directory.path())?;
    }
    Ok(())
}

#[test]
fn default_live_without_cutoff_retains_generation_and_gather_validation() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let output = run(
            directory.path(),
            &["--generation", "invalid:fixture"],
            gather,
        )?;
        check!(eq; output.status.code(), Some(1));
        let expected = if gather.is_empty() {
            "Error: export generation \"invalid:fixture\" carries a character outside [A-Za-z0-9._-]"
        } else {
            GATHER_REFUSAL
        };
        check!(eq; std::str::from_utf8(&output.stderr)?.trim(), expected);
        check_empty(directory.path())?;
    }
    Ok(())
}

#[test]
fn offline_without_cutoff_retains_store_opening_with_or_without_gather() -> TestResult {
    for gather in GATHER_OPTIONS {
        let directory = tempfile::tempdir()?;
        let root = directory.path().join("fixture-store");
        std::fs::write(&root, b"not a store directory")?;
        let output = run(directory.path(), &["--store", "fixture-store"], gather)?;
        check!(eq; output.status.code(), Some(1));
        check!(eq; output.stdout.as_slice(), b"");
        let diagnostic = std::str::from_utf8(&output.stderr)?;
        check!(diagnostic.contains("store open failed"));
        check!(!diagnostic.contains("standalone run does not support --as-of"));
        check!(eq; std::fs::read(&root)?, b"not a store directory");
        let entries = std::fs::read_dir(directory.path())?.collect::<std::io::Result<Vec<_>>>()?;
        check!(eq; entries.len(), 1);
    }
    Ok(())
}

#[test]
fn cutoff_still_requires_a_real_calendar_date_before_execution() -> TestResult {
    let directory = tempfile::tempdir()?;
    let output = run(directory.path(), &["--as-of", "2026-02-29"], &[])?;
    check!(eq; output.status.code(), Some(2));
    check!(eq; output.stdout.as_slice(), b"");
    let diagnostic = std::str::from_utf8(&output.stderr)?;
    check!(diagnostic.contains("--as-of"));
    check!(diagnostic.contains("2026-02-29"));
    check!(diagnostic.contains("input is out of range"));
    check_empty(directory.path())?;
    Ok(())
}

#[test]
fn standalone_help_discloses_the_cutoff_refusal() -> TestResult {
    let directory = tempfile::tempdir()?;
    let output = run(directory.path(), &["--help"], &[])?;
    check!(eq; output.status.code(), Some(0));
    check!(eq; output.stderr.as_slice(), b"");
    let help = std::str::from_utf8(&output.stdout)?;
    check!(help.contains("--as-of"));
    check!(help.contains("Unsupported for standalone run"));
    check_empty(directory.path())?;
    Ok(())
}
