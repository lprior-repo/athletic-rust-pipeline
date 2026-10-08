use super::Meta;
use crate::paths;
use anyhow::{bail, Context, Result};

pub fn build_meta() -> Result<Meta> {
    let corpus = super::corpus::measure(&paths::fixtures_dir())?;
    Ok(Meta {
        cpu: cpu_model()?,
        cores: available_cores()?,
        rustc: super::Cmd::new("rustc")
            .arg("--version")
            .output()?
            .trim()
            .to_owned(),
        sha: super::Cmd::new("git")
            .args(["rev-parse", "HEAD"])
            .output()?
            .trim()
            .to_owned(),
        corpus_lines: corpus.lines,
        corpus_sha256: corpus.sha256,
    })
}

pub fn cpu_model() -> Result<String> {
    let content = std::fs::read_to_string("/proc/cpuinfo").context("reading CPU identity")?;
    content
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once(':')?;
            (key.trim() == "model name" && !value.trim().is_empty())
                .then(|| value.trim().to_owned())
        })
        .context("CPU model is unavailable")
}

fn available_cores() -> Result<u32> {
    let output = super::Cmd::new("nproc").output()?;
    let cores = output
        .trim()
        .parse::<u32>()
        .context("parsing available core count")?;
    if cores == 0 {
        bail!("available core count must be positive");
    }
    Ok(cores)
}

pub fn perf_available() -> bool {
    std::process::Command::new("perf")
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}
