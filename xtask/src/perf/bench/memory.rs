use super::{dhat, runtime};
use anyhow::{bail, Context, Result};
use std::path::Path;
use std::process::Command;

pub(super) struct MemoryMeasurement {
    pub peak_rss_kib: u64,
    pub allocation_count: u64,
    pub allocated_bytes: u64,
}

pub(super) fn measure(executable: &Path, directory: &Path, id: &str) -> Result<MemoryMeasurement> {
    let filter = format!("^{}$", regex::escape(id));
    let peak_rss_kib = peak_rss(executable, directory, &filter)?;
    let (allocation_count, allocated_bytes) = heap(executable, directory, &filter)?;
    Ok(MemoryMeasurement {
        peak_rss_kib,
        allocation_count,
        allocated_bytes,
    })
}

fn peak_rss(executable: &Path, directory: &Path, filter: &str) -> Result<u64> {
    let rss_path = directory.join("single-rss");
    let mut time = Command::new("time");
    time.args(["-v", "-o"]).arg(&rss_path).arg(executable);
    run_measurement(&mut time, directory, filter, "GNU time")?;
    runtime::parse_rss(&std::fs::read_to_string(rss_path)?)
}

fn heap(executable: &Path, directory: &Path, filter: &str) -> Result<(u64, u64)> {
    let heap_path = directory.join("single-dhat.json");
    let mut valgrind = Command::new("valgrind");
    valgrind
        .args(["--tool=dhat", "--error-exitcode=97"])
        .arg(format!("--dhat-out-file={}", heap_path.display()))
        .arg(executable);
    run_measurement(&mut valgrind, directory, filter, "Valgrind DHAT")?;
    dhat::read(&heap_path)
}

pub(super) fn run_measurement(
    command: &mut Command,
    directory: &Path,
    filter: &str,
    tool: &str,
) -> Result<()> {
    let output = command
        .args(["--bench", "--test", "--noplot", filter])
        .env("CRITERION_HOME", directory.join("memory-criterion"))
        .env("LC_ALL", "C")
        .current_dir(crate::paths::repo_root())
        .output()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                anyhow::anyhow!("{tool} is required for the memory measurement: {error}")
            }
            _ => anyhow::Error::new(error)
                .context(format!("running {tool} for the memory measurement")),
        })?;
    if !output.status.success() {
        bail!(
            "memory measurement exited with {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let stdout =
        String::from_utf8(output.stdout).context("memory measurement stdout is not UTF-8")?;
    if !stdout.lines().any(|line| line.trim() == "Success") {
        bail!("memory measurement selected no successful benchmark: {stdout}");
    }
    Ok(())
}
