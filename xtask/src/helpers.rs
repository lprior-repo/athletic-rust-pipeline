//! Shared utilities for xtask subcommands.

use crate::cmd::Cmd;
use anyhow::Result;

/// `cargo nextest run -p census-service -E 'test(<source>)'`: one source's tests, nothing else.
pub(crate) fn source_test(source: &str) -> Result<()> {
    Cmd::new("cargo")
        .args(["nextest", "run", "-p", "census-service", "-E"])
        .arg(format!("test({source})"))
        .run()
}

/// The gate's tests lane over source targets: `cargo nextest run --workspace --lib --bins
/// --examples --all-features`, or the `cargo test` fallback `tools/gate.sh` takes without nextest.
///
/// `tools/gate.sh`'s `lane_tests` is `cargo nextest run --workspace --all-features`, falling back to
/// `cargo test --workspace --all-features --quiet`, and its strict clippy lane spells "source
/// targets" `--lib --bins --examples`. This is those two facts composed rather than a new
/// invocation: the same lane, over the targets that carry a colocated `#[cfg(test)]` module, so the
/// `tests/` integration binaries stay with the gate.
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

/// Whether `cargo-nextest` answers on `PATH`: the check `tools/gate.sh`'s tests lane makes.
pub(crate) fn nextest_installed() -> bool {
    std::process::Command::new("cargo-nextest")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// Run the criterion pipeline benchmark, filtered by whatever follows `--`.
pub(crate) fn bench(args: &[String]) -> Result<()> {
    Cmd::new("cargo")
        .args(["bench", "-p", "census-service"])
        .args(args)
        .run()
}
