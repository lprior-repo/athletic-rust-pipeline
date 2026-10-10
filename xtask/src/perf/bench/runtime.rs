use crate::cmd::Cmd;
use anyhow::{bail, Context, Result};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub(super) fn compile(name: &str) -> Result<PathBuf> {
    let (status, output, stderr) = Cmd::new("cargo")
        .args([
            "bench",
            "-p",
            "census-service",
            "--bench",
            name,
            "--no-run",
            "--message-format",
            "json",
        ])
        .capture()?;
    eprint!("{stderr}");
    let mut executable = None;
    for line in output.lines() {
        let message: serde_json::Value =
            serde_json::from_str(line).context("invalid Cargo benchmark artifact JSON")?;
        if let Some(rendered) = message
            .get("message")
            .and_then(|value| value.get("rendered"))
            .and_then(|value| value.as_str())
        {
            eprint!("{rendered}");
        }
        let target = message.get("target");
        let named = target
            .and_then(|value| value.get("name"))
            .and_then(|value| value.as_str());
        let benchmark = target
            .and_then(|value| value.get("kind"))
            .and_then(|value| value.as_array())
            .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bench"));
        if named == Some(name) && benchmark {
            if let Some(path) = message.get("executable").and_then(|value| value.as_str()) {
                executable = Some(PathBuf::from(path));
            }
        }
    }
    if !status.success() {
        bail!("compiling benchmark {name} exited with {status}");
    }
    executable.with_context(|| format!("Cargo returned no executable for benchmark {name}"))
}

pub(super) struct RunOutcome {
    pub(super) output: String,
    pub(super) rss: Option<u64>,
    pub(super) criterion: PathBuf,
}

pub(super) fn measure(
    executable: &Path,
    directory: &Path,
) -> Result<(String, Option<u64>, PathBuf)> {
    let iterations = 3;
    let mut runs = Vec::new();
    for i in 0..iterations {
        let run_dir = directory.join(format!("run-{i}"));
        std::fs::create_dir_all(&run_dir)?;
        let (output, rss) = measure_with_time(executable, &run_dir, Some(Path::new("time")))?;
        runs.push(RunOutcome {
            output,
            rss,
            criterion: run_dir.join("criterion"),
        });
    }
    let best = best_run(runs).ok_or_else(|| anyhow::anyhow!("all benchmark runs failed"))?;
    Ok((best.output, best.rss, best.criterion))
}

pub(super) fn best_run(runs: Vec<RunOutcome>) -> Option<RunOutcome> {
    let mut best: Option<(u64, RunOutcome)> = None;
    let mut fallback: Option<RunOutcome> = None;
    for run in runs {
        match run.rss {
            Some(rss) => match &best {
                Some((current, _)) if rss >= *current => {}
                _ => best = Some((rss, run)),
            },
            None => {
                if fallback.is_none() {
                    fallback = Some(run);
                }
            }
        }
    }
    match best {
        Some((_, run)) => Some(run),
        None => fallback,
    }
}

pub(super) fn measure_with_time(
    executable: &Path,
    directory: &Path,
    time: Option<&Path>,
) -> Result<(String, Option<u64>)> {
    let time = time.context("GNU time is required for peak RSS measurement")?;
    let stdout_path = directory.join("stdout");
    let rss_path = directory.join("rss");
    let mut command = Command::new(time);
    command.args(["-v", "-o"]).arg(&rss_path).arg(executable);
    run_timing(&mut command, directory, &stdout_path)?;
    let stdout = std::fs::read_to_string(stdout_path).context("reading benchmark output")?;
    let rss = parse_rss(&std::fs::read_to_string(rss_path).context("reading GNU time output")?)?;
    Ok((stdout, Some(rss)))
}

fn run_timing(command: &mut Command, directory: &Path, stdout_path: &Path) -> Result<()> {
    let status = command
        .args(["--bench", "--output-format", "bencher", "--noplot"])
        .env("CRITERION_HOME", directory.join("criterion"))
        .env("LC_ALL", "C")
        .current_dir(crate::paths::repo_root())
        .stdout(Stdio::from(File::create(stdout_path)?))
        .stderr(Stdio::inherit())
        .status()
        .context("running benchmark with required GNU time")?;
    if !status.success() {
        bail!(
            "benchmark exited with {status}: {}",
            std::fs::read_to_string(stdout_path)?
        );
    }
    Ok(())
}

pub(super) fn parse_rss(output: &str) -> Result<u64> {
    let raw = output
        .lines()
        .find_map(|line| {
            line.trim_start()
                .strip_prefix("Maximum resident set size (kbytes):")
        })
        .context("GNU time output has no peak RSS")?;
    let value = raw
        .trim()
        .parse::<u64>()
        .context("invalid GNU time peak RSS")?;
    if value == 0 {
        bail!("GNU time peak RSS must be positive");
    }
    Ok(value)
}
