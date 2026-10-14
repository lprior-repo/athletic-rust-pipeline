#![forbid(unsafe_code)]

#[macro_use]
#[path = "../../tools/fallible_checks.rs"]
mod fallible_checks;

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn repo_path(path: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(path)
}

fn cargo_case() -> Result<(tempfile::TempDir, Command), Box<dyn std::error::Error>> {
    let root = tempfile::Builder::new().prefix("moon cargo ").tempdir()?;
    let cargo = root.path().join("cargo");
    std::fs::write(
        &cargo,
        "#!/usr/bin/env bash\n\
         printf '%s\\0' \"${CARGO_BUILD_JOBS-<unset>}\" \"${MOON_RUST_BUILD_JOBS-<unset>}\" \"$@\"\n\
         exit 37\n",
    )?;
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755))?;
    let mut command = Command::new("bash");
    command
        .arg(repo_path("tools/moon-cargo"))
        .env("MOON_RUST_CARGO", &cargo)
        .env_remove("CARGO_BUILD_JOBS")
        .env_remove("MOON_RUST_BUILD_JOBS")
        .env_remove("BASH_ENV")
        .env_remove("ENV");
    Ok((root, command))
}

#[test]
fn unset_override_preserves_existing_jobs_and_cargo_exit() -> TestResult {
    for (jobs, expected) in [(None, "<unset>"), (Some("16"), "16")] {
        let (_root, mut command) = cargo_case()?;
        if let Some(value) = jobs {
            command.env("CARGO_BUILD_JOBS", value);
        }
        let output = command.arg("check").output()?;
        check!(eq; output.status.code(), Some(37));
        check!(eq; String::from_utf8(output.stdout)?, format!("{expected}\0<unset>\0check\0"));
        check!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn override_replaces_task_jobs_and_preserves_literal_arguments() -> TestResult {
    let arguments = ["nextest", "run", "--build-jobs=7", "-E", "test(foo)", ""];
    for jobs in ["1", "2", "16"] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("CARGO_BUILD_JOBS", "16")
            .env("MOON_RUST_BUILD_JOBS", jobs)
            .args(arguments)
            .output()?;
        check!(eq; output.status.code(), Some(37));
        let expected = format!("{jobs}\0{jobs}\0{}\0", arguments.join("\0"));
        check!(eq; String::from_utf8(output.stdout)?, expected);
        check!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn invalid_override_refuses_to_start_cargo() -> TestResult {
    for jobs in [
        "", "0", "00", "01", "-1", "+1", "1.5", "1e2", " 1", "1 ", "1\n", "1x", "２", "$(echo 1)",
    ] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("CARGO_BUILD_JOBS", "16")
            .env("MOON_RUST_BUILD_JOBS", jobs)
            .arg("build")
            .output()?;
        check!(eq; output.status.code(), Some(2), "{jobs:?}");
        check!(output.stdout.is_empty(), "Cargo started for {jobs:?}");
        check!(eq; String::from_utf8(output.stderr)?,
            "tools/moon-cargo: MOON_RUST_BUILD_JOBS must be a positive decimal integer without leading zeros\n");
    }
    Ok(())
}

#[test]
fn override_does_not_bypass_selected_cargo_guard() -> TestResult {
    let (_root, mut command) = cargo_case()?;
    let output = command
        .env_remove("MOON_RUST_CARGO")
        .env("MOON_RUST_BUILD_JOBS", "1")
        .arg("build")
        .output()?;
    check!(!output.status.success());
    check!(output.stdout.is_empty());
    check!(String::from_utf8(output.stderr)?.contains("Run through tools/moon-local"));
    Ok(())
}

#[test]
fn gate_cargo_children_inherit_the_effective_override() -> TestResult {
    let (root, mut command) = cargo_case()?;
    let path = std::env::var_os("PATH").ok_or("PATH is required for Bash")?;
    let mut paths = vec![root.path().to_path_buf()];
    paths.extend(std::env::split_paths(&path));
    let output = command
        .env("MOON_RUST_CARGO", "bash")
        .env("PATH", std::env::join_paths(paths)?)
        .env("CARGO_BUILD_JOBS", "16")
        .env("MOON_RUST_BUILD_JOBS", "1")
        .arg(repo_path("tools/gate.sh"))
        .args(["--release", "--lane", "contract"])
        .output()?;
    let stdout = String::from_utf8(output.stdout)?;
    check!(eq; output.status.code(), Some(1), "{stdout}");
    check!(stdout.contains("1\01\0run\0-q\0-p\0xtask\0--\0contract\0"), "{stdout:?}");
    check!(stdout.contains("architecture contract: FAIL"), "{stdout}");
    check!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn nested_wrapper_reapplies_override_after_task_defaults() -> TestResult {
    let (root, mut command) = cargo_case()?;
    let output = command
        .env("MOON_RUST_CARGO", "bash")
        .env("MOON_RUST_BUILD_JOBS", "1")
        .env("CARGO_STUB", root.path().join("cargo"))
        .env("CARGO_WRAPPER", repo_path("tools/moon-cargo"))
        .args([
            "-c",
            "CARGO_BUILD_JOBS=16 MOON_RUST_CARGO=\"$CARGO_STUB\" exec bash \"$CARGO_WRAPPER\" test --locked --offline",
        ])
        .output()?;
    check!(eq; output.status.code(), Some(37));
    check!(eq; String::from_utf8(output.stdout)?, "1\01\0test\0--locked\0--offline\0");
    check!(output.stderr.is_empty());
    Ok(())
}
