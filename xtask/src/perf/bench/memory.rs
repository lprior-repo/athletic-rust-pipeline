use super::runtime;
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
    let rss_path = directory.join("single-rss");
    let heap_path = directory.join("single-heap");
    let mut time = Command::new("/usr/bin/time");
    time.args(["-v", "-o"]).arg(&rss_path).arg(executable);
    run_single(&mut time, directory, &filter)?;
    let peak_rss_kib = runtime::parse_rss(&std::fs::read_to_string(rss_path)?)?;
    let mut heap = Command::new("valgrind");
    heap.args(["--tool=memcheck", "--leak-check=no", "--error-exitcode=97"])
        .arg(format!("--log-file={}", heap_path.display()))
        .arg(executable);
    run_single(&mut heap, directory, &filter)?;
    let (allocation_count, allocated_bytes) =
        parse_allocations(&std::fs::read_to_string(heap_path)?)?;
    Ok(MemoryMeasurement {
        peak_rss_kib,
        allocation_count,
        allocated_bytes,
    })
}

fn run_single(command: &mut Command, directory: &Path, filter: &str) -> Result<()> {
    let output = command
        .args(["--bench", "--test", "--noplot", filter])
        .env("CRITERION_HOME", directory.join("memory-criterion"))
        .env("LC_ALL", "C")
        .current_dir(crate::paths::repo_root())
        .output()
        .context("running required GNU time/Valgrind memory measurement")?;
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

pub(super) fn parse_allocations(output: &str) -> Result<(u64, u64)> {
    let mut summaries = output
        .lines()
        .filter_map(|line| line.split_once("total heap usage:").map(|(_, rest)| rest));
    let summary = summaries
        .next()
        .context("Valgrind output has no allocation summary")?;
    if summaries.next().is_some() {
        bail!("Valgrind output has ambiguous allocation summaries");
    }
    let (allocations, rest) = summary
        .split_once(" allocs, ")
        .context("invalid allocation count summary")?;
    let (_, bytes) = rest
        .split_once(" frees, ")
        .context("invalid free count summary")?;
    let bytes = bytes
        .trim()
        .strip_suffix(" bytes allocated")
        .context("invalid allocated bytes summary")?;
    Ok((positive_count(allocations.trim())?, positive_count(bytes)?))
}

fn positive_count(raw: &str) -> Result<u64> {
    let value = raw
        .replace(',', "")
        .parse::<u64>()
        .context("invalid memory count")?;
    if value == 0 {
        bail!("memory count must be positive");
    }
    Ok(value)
}
