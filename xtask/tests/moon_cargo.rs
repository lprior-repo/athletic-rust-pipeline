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
        .env_remove("MOON_RUST_TEST_THREADS")
        .env_remove("MOON_RUST_SCOPED_TEST_FEATURES")
        .env_remove("MOON_TARGET")
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
        "",
        "0",
        "00",
        "01",
        "-1",
        "+1",
        "1.5",
        "1e2",
        " 1",
        "1 ",
        "1\n",
        "1x",
        "２",
        "$(echo 1)",
    ] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("CARGO_BUILD_JOBS", "16")
            .env("MOON_RUST_BUILD_JOBS", jobs)
            .arg("build")
            .output()?;
        check!(eq; output.status.code(), Some(2), "{jobs:?}");
        check!(output.stdout.is_empty(), "Cargo started for {jobs:?}");
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
    let stdout = String::from_utf8(output.stdout)?;
    let mut fields = stdout.split('\0');
    check!(eq; fields.next(), Some("1"));
    check!(eq; fields.next(), Some("1"));
    check!(output.stderr.is_empty());
    Ok(())
}

const SCOPED_PREFIX: [&str; 9] = [
    "nextest",
    "run",
    "--config",
    ".cargo/fast-iteration.toml",
    "--all-features",
    "--locked",
    "--offline",
    "--no-fail-fast",
    "--test-threads=16",
];

const WORKSPACE_PREFIX: [&str; 10] = [
    "nextest",
    "run",
    "--config",
    ".cargo/fast-iteration.toml",
    "--workspace",
    "--all-features",
    "--locked",
    "--offline",
    "--no-fail-fast",
    "--test-threads=16",
];

const REPORT_PREFIX: [&str; 12] = [
    "nextest",
    "run",
    "--config",
    ".cargo/fast-iteration.toml",
    "-p",
    "census-report",
    "--lib",
    "--all-features",
    "--locked",
    "--offline",
    "--no-fail-fast",
    "--test-threads=16",
];

const TEST_TASKS: [(&str, &[&str]); 3] = [
    ("pipeline:tests-scoped", &SCOPED_PREFIX),
    ("pipeline:tests", &WORKSPACE_PREFIX),
    ("pipeline:report-test", &REPORT_PREFIX),
];

#[test]
fn test_override_prefixes_match_the_canonical_tasks() -> TestResult {
    let source = include_str!("../../moon.yml");
    for (target, prefix) in TEST_TASKS {
        let name = target.strip_prefix("pipeline:").ok_or("project absent")?;
        let marker = format!("  {name}:\n");
        let task = source.split_once(marker.as_str()).ok_or("task absent")?.1;
        let expected = format!("    command: 'tools/moon-cargo {}'", prefix.join(" "));
        check!(eq; task.lines().next(), Some(expected.as_str()));
    }
    Ok(())
}

#[test]
fn unset_and_all_modes_preserve_exact_scoped_arguments() -> TestResult {
    for mode in [None, Some("all")] {
        let (_root, mut command) = cargo_case()?;
        command.env("MOON_TARGET", "pipeline:tests-scoped");
        if let Some(mode) = mode {
            command.env("MOON_RUST_SCOPED_TEST_FEATURES", mode);
        }
        let arguments = SCOPED_PREFIX.into_iter().chain([
            "--features=loom",
            "--no-default-features",
            "-E",
            "test(a) | test(b)",
            "",
        ]);
        let arguments = arguments.collect::<Vec<_>>();
        let output = command.args(&arguments).output()?;
        check!(eq; output.status.code(), Some(37));
        check!(eq; String::from_utf8(output.stdout)?, format!("<unset>\0<unset>\0{}\0", arguments.join("\0")));
        check!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn default_mode_removes_only_owned_all_features_and_retains_build_limits() -> TestResult {
    let (_root, mut command) = cargo_case()?;
    let arguments = SCOPED_PREFIX
        .into_iter()
        .chain([
            "-p",
            "census-service",
            "--lib",
            "-E",
            "test(a) | test(b)",
            "--no-tests",
            "fail",
            "",
        ])
        .collect::<Vec<_>>();
    let output = command
        .env("MOON_TARGET", "pipeline:tests-scoped")
        .env("MOON_RUST_SCOPED_TEST_FEATURES", "default")
        .env("CARGO_BUILD_JOBS", "16")
        .env("MOON_RUST_BUILD_JOBS", "1")
        .args(&arguments)
        .output()?;
    let expected = arguments
        .iter()
        .enumerate()
        .filter_map(|(index, value)| (index != 4).then_some(*value))
        .collect::<Vec<_>>();
    check!(eq; output.status.code(), Some(37));
    check!(eq; String::from_utf8(output.stdout)?, format!("{}\0{}\0{}\0", 1, 1, expected.join("\0")));
    check!(output.stderr.is_empty());
    Ok(())
}

#[test]
fn invalid_feature_modes_or_wrong_tasks_refuse_before_cargo() -> TestResult {
    for mode in [
        "",
        "none",
        "Default",
        " default",
        "default ",
        "default\n",
        "$(echo default)",
    ] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("MOON_TARGET", "pipeline:tests-scoped")
            .env("MOON_RUST_SCOPED_TEST_FEATURES", mode)
            .args(SCOPED_PREFIX)
            .output()?;
        check!(eq; output.status.code(), Some(2));
        check!(output.stdout.is_empty());
    }
    for mode in ["all", "default"] {
        for target in [
            "",
            "pipeline:tests",
            "pipeline:gate",
            "pipeline:build-scoped",
        ] {
            let (_root, mut command) = cargo_case()?;
            let output = command
                .env("MOON_TARGET", target)
                .env("MOON_RUST_SCOPED_TEST_FEATURES", mode)
                .args(SCOPED_PREFIX)
                .output()?;
            check!(eq; output.status.code(), Some(2));
            check!(output.stdout.is_empty());
        }
    }
    Ok(())
}

#[test]
fn changed_or_truncated_scoped_prefix_refuses_before_cargo() -> TestResult {
    for mode in ["all", "default"] {
        for index in 0..SCOPED_PREFIX.len() {
            let mut arguments = SCOPED_PREFIX;
            arguments[index] = "changed";
            for arguments in [arguments.as_slice(), &SCOPED_PREFIX[..index]] {
                let (_root, mut command) = cargo_case()?;
                let output = command
                    .env("MOON_TARGET", "pipeline:tests-scoped")
                    .env("MOON_RUST_SCOPED_TEST_FEATURES", mode)
                    .args(arguments)
                    .output()?;
                check!(eq; output.status.code(), Some(2));
                check!(output.stdout.is_empty());
            }
        }
    }
    Ok(())
}

#[test]
fn default_mode_refuses_conflicting_caller_feature_controls() -> TestResult {
    for flag in [
        "--all-features",
        "--all-features=true",
        "--features",
        "--features=native-fault-injection",
        "-F",
        "-Fnative-fault-injection",
        "--no-default-features",
        "--no-default-features=true",
    ] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("MOON_TARGET", "pipeline:tests-scoped")
            .env("MOON_RUST_SCOPED_TEST_FEATURES", "default")
            .args(SCOPED_PREFIX)
            .arg(flag)
            .output()?;
        check!(eq; output.status.code(), Some(2));
        check!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn default_mode_keeps_selected_cargo_and_build_job_guards() -> TestResult {
    for missing_cargo in [false, true] {
        let (_root, mut command) = cargo_case()?;
        command
            .env("MOON_TARGET", "pipeline:tests-scoped")
            .env("MOON_RUST_SCOPED_TEST_FEATURES", "default");
        if missing_cargo {
            command.env_remove("MOON_RUST_CARGO");
        } else {
            command.env("MOON_RUST_BUILD_JOBS", "0");
        }
        let output = command.args(SCOPED_PREFIX).output()?;
        check!(!output.status.success());
        check!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn unset_test_threads_preserves_each_canonical_task() -> TestResult {
    for (target, prefix) in TEST_TASKS {
        let (_root, mut command) = cargo_case()?;
        let output = command.env("MOON_TARGET", target).args(prefix).output()?;
        check!(eq; output.status.code(), Some(37));
        check!(eq; String::from_utf8(output.stdout)?, format!("<unset>\0<unset>\0{}\0", prefix.join("\0")));
        check!(output.stderr.is_empty());
    }
    Ok(())
}

#[test]
fn test_thread_override_replaces_only_the_canonical_thread_argument() -> TestResult {
    for (target, prefix) in TEST_TASKS {
        for threads in 1..=16 {
            let (_root, mut command) = cargo_case()?;
            let arguments = prefix
                .iter()
                .copied()
                .chain([
                    "--build-jobs=7",
                    "--features=loom",
                    "-E",
                    "test(a) | test(b)",
                    "-Etest(project)",
                    "--no-tests",
                    "fail",
                    "",
                ])
                .collect::<Vec<_>>();
            let output = command
                .env("MOON_TARGET", target)
                .env("MOON_RUST_TEST_THREADS", threads.to_string())
                .env("CARGO_BUILD_JOBS", "16")
                .env("MOON_RUST_BUILD_JOBS", "1")
                .args(&arguments)
                .output()?;
            let replacement = format!("--test-threads={threads}");
            let expected = arguments
                .iter()
                .map(|argument| match *argument {
                    "--test-threads=16" => replacement.as_str(),
                    value => value,
                })
                .collect::<Vec<_>>();
            check!(eq; output.status.code(), Some(37), "{target} {threads}");
            check!(eq; String::from_utf8(output.stdout)?, format!("{}\0{}\0{}\0", 1, 1, expected.join("\0")));
            check!(output.stderr.is_empty());
        }
    }
    Ok(())
}

#[test]
fn test_thread_override_composes_with_scoped_feature_modes() -> TestResult {
    for mode in ["all", "default"] {
        for threads in ["1", "16"] {
            let (_root, mut command) = cargo_case()?;
            let arguments = SCOPED_PREFIX
                .into_iter()
                .chain(["-p", "census-store", "--lib", "--no-tests", "fail"])
                .collect::<Vec<_>>();
            let output = command
                .env("MOON_TARGET", "pipeline:tests-scoped")
                .env("MOON_RUST_TEST_THREADS", threads)
                .env("MOON_RUST_SCOPED_TEST_FEATURES", mode)
                .args(&arguments)
                .output()?;
            let replacement = format!("--test-threads={threads}");
            let expected = arguments
                .iter()
                .filter_map(|argument| match *argument {
                    "--all-features" if mode == "default" => None,
                    "--test-threads=16" => Some(replacement.as_str()),
                    value => Some(value),
                })
                .collect::<Vec<_>>();
            check!(eq; output.status.code(), Some(37));
            check!(eq; String::from_utf8(output.stdout)?, format!("<unset>\0<unset>\0{}\0", expected.join("\0")));
            check!(output.stderr.is_empty());
        }
    }
    Ok(())
}

#[test]
fn invalid_test_thread_override_refuses_before_cargo() -> TestResult {
    for threads in [
        "",
        "0",
        "00",
        "01",
        "17",
        "100000000000000000000000000000000000000",
        "-1",
        "+1",
        "1.5",
        "1e1",
        " 1",
        "1 ",
        "1\n",
        "1x",
        "２",
        "$(echo 1)",
    ] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("MOON_TARGET", "pipeline:tests-scoped")
            .env("MOON_RUST_TEST_THREADS", threads)
            .args(SCOPED_PREFIX)
            .output()?;
        check!(eq; output.status.code(), Some(2), "{threads:?}");
        check!(output.stdout.is_empty(), "Cargo started for {threads:?}");
    }
    Ok(())
}

#[test]
fn test_thread_override_refuses_unsupported_tasks() -> TestResult {
    for target in [
        "",
        "pipeline:check",
        "pipeline:gate",
        "pipeline:build-scoped",
        "other:tests-scoped",
    ] {
        let (_root, mut command) = cargo_case()?;
        let output = command
            .env("MOON_TARGET", target)
            .env("MOON_RUST_TEST_THREADS", "1")
            .args(SCOPED_PREFIX)
            .output()?;
        check!(eq; output.status.code(), Some(2), "{target:?}");
        check!(output.stdout.is_empty());
    }
    Ok(())
}

#[test]
fn test_thread_override_refuses_changed_or_truncated_prefixes() -> TestResult {
    for (target, prefix) in TEST_TASKS {
        for (index, _) in prefix.iter().enumerate() {
            let mut changed = prefix.to_vec();
            *changed.get_mut(index).ok_or("prefix index absent")? = "changed";
            let truncated = prefix.get(..index).ok_or("prefix range absent")?;
            for arguments in [changed.as_slice(), truncated] {
                let (_root, mut command) = cargo_case()?;
                let output = command
                    .env("MOON_TARGET", target)
                    .env("MOON_RUST_TEST_THREADS", "1")
                    .args(arguments)
                    .output()?;
                check!(eq; output.status.code(), Some(2), "{target} {index}");
                check!(output.stdout.is_empty());
            }
        }
    }
    Ok(())
}

#[test]
fn test_thread_override_refuses_mismatched_canonical_targets() -> TestResult {
    for (target, _) in TEST_TASKS {
        for (other, prefix) in TEST_TASKS {
            if target == other {
                continue;
            }
            let (_root, mut command) = cargo_case()?;
            let output = command
                .env("MOON_TARGET", target)
                .env("MOON_RUST_TEST_THREADS", "1")
                .args(prefix)
                .output()?;
            check!(eq; output.status.code(), Some(2), "{target} {other}");
            check!(output.stdout.is_empty());
        }
    }
    Ok(())
}

#[test]
fn test_thread_override_refuses_duplicate_thread_controls() -> TestResult {
    for (target, prefix) in TEST_TASKS {
        for flag in [
            "--test-threads",
            "--test-threads=1",
            "--test-threads=16",
            "--jobs",
            "--jobs=1",
            "-j",
            "-j1",
            "-j=1",
            "-vj1",
            "-rvj1",
        ] {
            let (_root, mut command) = cargo_case()?;
            let output = command
                .env("MOON_TARGET", target)
                .env("MOON_RUST_TEST_THREADS", "1")
                .args(prefix)
                .args([flag, "1"])
                .output()?;
            check!(eq; output.status.code(), Some(2), "{target} {flag}");
            check!(output.stdout.is_empty());
        }
    }
    Ok(())
}

#[test]
fn test_thread_override_keeps_selected_cargo_and_build_job_guards() -> TestResult {
    for missing_cargo in [false, true] {
        let (_root, mut command) = cargo_case()?;
        command
            .env("MOON_TARGET", "pipeline:tests-scoped")
            .env("MOON_RUST_TEST_THREADS", "1");
        if missing_cargo {
            command.env_remove("MOON_RUST_CARGO");
        } else {
            command.env("MOON_RUST_BUILD_JOBS", "0");
        }
        let output = command.args(SCOPED_PREFIX).output()?;
        check!(!output.status.success());
        check!(output.stdout.is_empty());
    }
    Ok(())
}
