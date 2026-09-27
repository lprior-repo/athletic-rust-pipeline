
use crate::cmd::Cmd;
use anyhow::Result;

pub(crate) fn source_test(source: &str) -> Result<()> {
    Cmd::new("cargo")
        .args(["nextest", "run", "-p", "census-service", "-E"])
        .arg(format!("test({source})"))
        .run()
}

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
