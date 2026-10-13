use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use super::support::{TestResult, FIXTURE_ROOT};

const CHILD_CASE: &str = "CENSUS_LINEAGE_CHILD_CASE";
const CHILD_RECEIPT: &str = "CENSUS_LINEAGE_CHILD_RECEIPT";

pub(super) fn bounded_case(name: &str, body: impl FnOnce() -> TestResult) -> TestResult {
    if std::env::var(CHILD_CASE).ok().as_deref() == Some(name) {
        body()?;
        let receipt = std::env::var_os(CHILD_RECEIPT)
            .ok_or("a supervised lineage child has no completion receipt path")?;
        std::fs::write(receipt, name.as_bytes())?;
        return Ok(());
    }
    let (_, filter) = name
        .split_once("::")
        .ok_or("a supervised lineage test name must include its crate prefix")?;
    let root = tempfile::tempdir()?;
    let receipt = root.path().join("completed");
    let mut child = Command::new(std::env::current_exe()?)
        .args(["--exact", filter, "--nocapture", "--test-threads=1"])
        .env(CHILD_CASE, name)
        .env(CHILD_RECEIPT, &receipt)
        .env(FIXTURE_ROOT, root.path())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()?;
    let status = match wait_bounded(&mut child) {
        Ok(status) => status,
        Err(failure) => match stop_child(&mut child) {
            Ok(()) => return Err(failure),
            Err(cleanup) => {
                return Err(format!("{failure}; child cleanup failed: {cleanup}").into())
            }
        },
    };
    if !status.success() {
        return Err(format!("supervised lineage case {name} exited {status}").into());
    }
    let completed = std::fs::read(&receipt)?;
    if completed.as_slice() != name.as_bytes() {
        return Err(format!("supervised lineage case {name} did not certify completion").into());
    }
    Ok(())
}

fn wait_bounded(child: &mut Child) -> TestResult<ExitStatus> {
    for _ in 0..3_000 {
        if let Some(status) = child.try_wait()? {
            return Ok(status);
        }
        std::thread::park_timeout(Duration::from_millis(10));
    }
    Err("a supervised lineage case exceeded 3,000 bounded process polls".into())
}

fn stop_child(child: &mut Child) -> TestResult {
    let mut failures = Vec::new();
    match child.try_wait() {
        Ok(Some(_)) => return Ok(()),
        Ok(None) => {}
        Err(error) => failures.push(format!("initial reap failed: {error}")),
    }
    if let Err(error) = child.kill() {
        failures.push(format!("termination failed: {error}"));
    }
    match wait_bounded(child) {
        Ok(_) if failures.is_empty() => Ok(()),
        Ok(status) => Err(format!("{}; child reaped with {status}", failures.join("; ")).into()),
        Err(error) => {
            failures.push(format!(
                "bounded reap failed: {error}; child termination is unverified"
            ));
            Err(failures.join("; ").into())
        }
    }
}
