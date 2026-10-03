use anyhow::{ensure, Context};
use serde_json::{json, Value};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use super::artifacts::{write_json, Result};

pub(super) fn arguments() -> Result<(PathBuf, PathBuf, String)> {
    let mut args = std::env::args_os().skip(1);
    let root = PathBuf::from(args.next().context("pass an unused output directory")?);
    let binary = std::fs::canonicalize(PathBuf::from(
        args.next()
            .context("pass rebuilt census-service binary path")?,
    ))?;
    ensure!(
        std::fs::metadata(&binary)?.is_file(),
        "CLI binary path is not a file"
    );
    let observed = args
        .next()
        .context("pass a separate RFC3339 adapter observation instant")?
        .into_string()
        .map_err(|_| anyhow::anyhow!("observation instant is not UTF-8"))?;
    ensure!(
        args.next().is_none(),
        "usage: qualification_postal UNUSED_DIRECTORY CENSUS_SERVICE_BINARY OBSERVED_RFC3339"
    );
    chrono::DateTime::parse_from_rfc3339(&observed).context("invalid observation instant")?;
    ensure!(!root.as_os_str().is_empty(), "output directory is empty");
    Ok((root, binary, observed))
}

pub(super) fn run_csv(root: &Path, binary: &Path) -> Result<Value> {
    let stdout = root.join("csv-cli.stdout");
    let stderr = root.join("csv-cli.stderr");
    let args = [
        OsString::from("--store"),
        root.join("store").into_os_string(),
        "export-data".into(),
        "--data".into(),
        root.join("data").into_os_string(),
        "--school-year".into(),
        "2026".into(),
    ];
    let argv = utf8_argv(&args)?;
    let mut child = Command::new(binary)
        .args(&args)
        .stdin(Stdio::null())
        .stdout(create_output(&stdout)?)
        .stderr(create_output(&stderr)?)
        .spawn()
        .context("starting real offline export-data CLI")?;
    let status = match wait_csv(&mut child, &stdout, &stderr) {
        Ok(status) => status,
        Err(error) => return record_failure(root, binary, &argv, &mut child, error),
    };
    ensure_output_budget(&stdout, &stderr)
        .context("terminated CSV CLI output exceeded qualification budget")?;
    let evidence = json!({"binary": binary, "binary_bytes": std::fs::metadata(binary)?.len(),
        "argv": argv, "exit_code": status.code(), "success": status.success(),
        "stdout": String::from_utf8(std::fs::read(&stdout)?)?, "stderr": String::from_utf8(std::fs::read(&stderr)?)?});
    write_json(&root.join("csv-cli.json"), &evidence)?;
    ensure!(
        status.success(),
        "real offline CSV CLI failed: {status}; see csv-cli.json/stdout/stderr"
    );
    Ok(evidence)
}

fn utf8_argv(args: &[OsString; 7]) -> Result<[&str; 7]> {
    let mut argv = [""; 7];
    argv.iter_mut()
        .zip(args)
        .try_for_each(|(slot, arg)| -> Result<()> {
            *slot = arg.to_str().context("CLI argv is not UTF-8")?;
            Ok(())
        })?;
    Ok(argv)
}

fn create_output(path: &Path) -> Result<std::fs::File> {
    Ok(std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?)
}

fn ensure_output_budget(stdout: &Path, stderr: &Path) -> Result<()> {
    ensure!(
        std::fs::metadata(stdout)?.len() <= 1024 * 1024
            && std::fs::metadata(stderr)?.len() <= 1024 * 1024,
        "CLI output exceeded qualification budget"
    );
    Ok(())
}

fn wait_csv(child: &mut Child, stdout: &Path, stderr: &Path) -> Result<ExitStatus> {
    (0..600)
        .find_map(|_| {
            let mut poll = || -> Result<Option<ExitStatus>> {
                ensure_output_budget(stdout, stderr)?;
                Ok(child.try_wait()?)
            };
            match poll() {
                Ok(Some(status)) => Some(Ok(status)),
                Ok(None) => {
                    std::thread::sleep(Duration::from_millis(100));
                    None
                }
                Err(error) => Some(Err(error)),
            }
        })
        .context("CSV CLI exceeded 60-second qualification deadline")
        .and_then(|result| result)
}

fn record_failure(
    root: &Path,
    binary: &Path,
    argv: &[&str; 7],
    child: &mut Child,
    error: anyhow::Error,
) -> Result<Value> {
    child
        .kill()
        .context("terminating failed qualification CLI")?;
    let status = child.wait().context("reaping failed qualification CLI")?;
    write_json(
        &root.join("csv-cli.json"),
        &json!({"binary": binary, "argv": argv,
        "exit_code": status.code(), "success": false, "error": error.to_string(),
        "stdout_path": root.join("csv-cli.stdout"), "stderr_path": root.join("csv-cli.stderr")}),
    )?;
    Err(error)
}
