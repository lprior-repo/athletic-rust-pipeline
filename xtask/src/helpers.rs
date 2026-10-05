use crate::cmd::Cmd;
use anyhow::{bail, Result};

fn normalize_source_slug(source: &str) -> Result<String> {
    let Some(first) = source.as_bytes().first() else {
        bail!("source name is empty");
    };
    if !(first.is_ascii_alphabetic() || *first == b'_')
        || !source
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'_' | b'-'))
    {
        bail!("source name must be a literal identifier: {source}");
    }
    Ok(source.replace('-', "_"))
}

pub(crate) fn source_test(source: &str) -> Result<()> {
    let slug = normalize_source_slug(source)?;
    if nextest_installed() {
        return Cmd::new("cargo")
            .args([
                "nextest",
                "run",
                "--workspace",
                "--all-features",
                "--no-tests",
                "fail",
                "-E",
            ])
            .arg(format!("test({slug})"))
            .run();
    }
    println!("cargo-nextest absent: falling back to cargo test");
    let (status, listing, stderr) = Cmd::new("cargo")
        .args(["test", "--workspace", "--all-features", "--quiet"])
        .arg(&slug)
        .args(["--", "--list"])
        .capture()?;
    if !status.success() {
        bail!("`cargo test --list` exited with status {status} for {slug}: {stderr}");
    }
    if !lists_a_test(&listing) {
        bail!("no test matches source {slug}: cargo test selected an empty lane");
    }
    Cmd::new("cargo")
        .args(["test", "--workspace", "--all-features", "--quiet"])
        .arg(slug)
        .run()
}

fn lists_a_test(listing: &str) -> bool {
    listing.lines().any(|line| line.ends_with(": test"))
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod tests;

pub(crate) fn source_tests() -> Result<()> {
    let targets = ["--lib", "--bins", "--examples"];
    if nextest_installed() {
        return Cmd::new("cargo")
            .args(["nextest", "run", "--workspace", "--all-features"])
            .args(targets)
            .run();
    }
    println!("cargo-nextest absent: falling back to cargo test");
    Cmd::new("cargo")
        .args(["test", "--workspace", "--all-features", "--quiet"])
        .args(targets)
        .run()
}

pub(crate) fn nextest_installed() -> bool {
    std::process::Command::new("cargo-nextest")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

pub(crate) fn bench(args: &[String]) -> Result<()> {
    Cmd::new("cargo")
        .args(["bench", "-p", "census-service"])
        .args(args)
        .run()
}
