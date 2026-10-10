use std::ffi::OsStr;

use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn options_parse_every_flag() -> TestResult {
    let args = [
        "--listen",
        "127.0.0.1:9101",
        "--data-dir",
        "/tmp/db",
        "--max-concurrent",
        "3",
        "--drain-timeout",
        "7",
        "--memory-budget-gib",
        "96",
    ]
    .into_iter()
    .map(str::to_string);
    let options = ServeOptions::from_env(args)?;
    let listen = options.listen.to_string();
    if listen != "127.0.0.1:9101" {
        return Err(format!("listen: left={listen:?}, right=\"127.0.0.1:9101\"").into());
    }
    if options.data_dir.to_str() != Some("/tmp/db") {
        return Err(format!(
            "data_dir: left={:?}, right={:?}",
            options.data_dir.to_str(),
            Some("/tmp/db")
        )
        .into());
    }
    if options.max_concurrent != 3 {
        return Err(format!("max_concurrent: left={:?}, right=3", options.max_concurrent).into());
    }
    if options.drain_timeout != Duration::from_secs(7) {
        return Err(format!(
            "drain_timeout: left={:?}, right={:?}",
            options.drain_timeout,
            Duration::from_secs(7)
        )
        .into());
    }
    if options.memory_budget_bytes != 96_u64 * 1024 * 1024 * 1024 {
        return Err(format!(
            "memory_budget_bytes: left={:?}, right={:?}",
            options.memory_budget_bytes,
            96_u64 * 1024 * 1024 * 1024
        )
        .into());
    }
    Ok(())
}

#[test]
fn options_reject_unknown_flags_and_zero_concurrency() {
    let unknown = ["--nope"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(unknown),
        Err(BootstrapError::UnknownFlag { .. })
    ));
    let zero = ["--max-concurrent", "0"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(zero),
        Err(BootstrapError::ConcurrencyIsZero)
    ));
    let zero = ["--memory-budget-gib", "0"].into_iter().map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(zero),
        Err(BootstrapError::MemoryBudgetIsZero)
    ));
    let junk = ["--memory-budget-gib", "lots"]
        .into_iter()
        .map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(junk),
        Err(BootstrapError::MemoryBudgetNotAGib { .. })
    ));
    let huge = ["--memory-budget-gib", "2048"]
        .into_iter()
        .map(str::to_string);
    assert!(matches!(
        ServeOptions::from_env(huge),
        Err(BootstrapError::MemoryBudgetTooLarge { .. })
    ));
    let default = ServeOptions::default();
    assert_eq!(default.memory_budget_bytes, 48_u64 * 1024 * 1024 * 1024);
}

#[test]
fn options_reject_concurrency_above_the_operational_ceiling() -> TestResult {
    let at_ceiling = [
        "--max-concurrent".to_string(),
        MAX_CONCURRENT_CEILING.to_string(),
    ];
    let options = ServeOptions::from_env(at_ceiling.into_iter())?;
    check!(eq; options.max_concurrent, MAX_CONCURRENT_CEILING);
    for value in [MAX_CONCURRENT_CEILING + 1, usize::MAX] {
        let over = ["--max-concurrent".to_string(), value.to_string()];
        match ServeOptions::from_env(over.into_iter()) {
            Err(BootstrapError::ConcurrencyTooLarge {
                value: rejected,
                ceiling,
            }) => {
                check!(eq; rejected, value);
                check!(eq; ceiling, MAX_CONCURRENT_CEILING);
            }
            Err(other) => return Err(format!("unexpected error: {other}").into()),
            Ok(_) => return Err("oversized concurrency accepted".into()),
        }
    }
    Ok(())
}

#[test]
fn options_name_the_flag_whose_value_is_missing_or_unparseable() -> TestResult {
    let missing = ["--listen"].into_iter().map(str::to_string);
    let error = match ServeOptions::from_env(missing) {
        Err(error) => error,
        Ok(_) => return Err("missing listen value accepted".into()),
    };
    if !matches!(&error, BootstrapError::MissingValue { flag } if flag == "--listen") {
        return Err(format!("expected missing --listen refusal, got {error:?}").into());
    }
    if !error.to_string().contains("--listen") {
        return Err(format!("missing value refusal did not name --listen: {error}").into());
    }

    let junk_seconds = ["--drain-timeout", "soon"].into_iter().map(str::to_string);
    if !matches!(ServeOptions::from_env(junk_seconds), Err(BootstrapError::DrainTimeoutNotASeconds { raw, .. }) if raw == "soon")
    {
        return Err("expected invalid drain seconds refusal for soon".into());
    }

    let junk_address = ["--listen", "not-an-address"]
        .into_iter()
        .map(str::to_string);
    if !matches!(ServeOptions::from_env(junk_address), Err(BootstrapError::ListenNotAnAddress { raw, .. }) if raw == "not-an-address")
    {
        return Err("expected invalid listen address refusal for not-an-address".into());
    }

    let help = ["--help"].into_iter().map(str::to_string);
    if !matches!(
        ServeOptions::from_env(help),
        Err(BootstrapError::HelpRequested)
    ) {
        return Err("expected HelpRequested".into());
    }
    Ok(())
}

#[test]
fn options_enable_the_lane_only_with_a_profile() -> TestResult {
    let plain = ServeOptions::from_env(Vec::<String>::new().into_iter())?;
    if !plain.lane.is_none() {
        return Err("no flag means no lane".into());
    }

    let without = tempfile::tempdir()?;
    let with = tempfile::tempdir()?;
    std::fs::write(with.path().join("chromium"), b"")?;
    let path = std::env::join_paths([without.path(), with.path()])?;

    let args = ["--browser-profile", "/tmp/profile", "--browser-headless"]
        .into_iter()
        .map(str::to_string);
    let lane = ServeOptions::from_env_with_path(args, Some(path.as_os_str()))?
        .lane
        .ok_or("missing configured browser lane")?;
    if lane.profile_dir.to_str() != Some("/tmp/profile") {
        return Err(format!(
            "profile_dir: left={:?}, right={:?}",
            lane.profile_dir.to_str(),
            Some("/tmp/profile")
        )
        .into());
    }
    let expected_executable = with.path().join("chromium");
    if lane.executable != expected_executable {
        return Err(format!(
            "the default browser is the one PATH holds: left={:?}, right={expected_executable:?}",
            lane.executable
        )
        .into());
    }
    if lane.headed {
        return Err("--browser-headless turns the headed profile off".into());
    }
    if lane.source_origin.as_str() != "https://www.athletic.net/" {
        return Err(format!(
            "source_origin: left={:?}, right=\"https://www.athletic.net/\"",
            lane.source_origin.as_str()
        )
        .into());
    }
    if lane.tabs != 2 {
        return Err(format!("tabs: left={:?}, right=2", lane.tabs).into());
    }

    let relative = ["--browser-profile", "var/browser-profile"]
        .into_iter()
        .map(str::to_string);
    let lane = ServeOptions::from_env_with_path(relative, Some(path.as_os_str()))?
        .lane
        .ok_or("missing relative-profile browser lane")?;
    if !lane.profile_dir.is_absolute() {
        return Err(format!("the lane rejects a relative profile directory, so the flag must resolve against the working directory: {}", lane.profile_dir.display()).into());
    }
    if !lane.profile_dir.ends_with("var/browser-profile") {
        return Err(format!(
            "relative profile suffix missing: {}",
            lane.profile_dir.display()
        )
        .into());
    }

    let executable_alone = ["--browser-executable", "/usr/bin/chromium"]
        .into_iter()
        .map(str::to_string);
    if !matches!(ServeOptions::from_env_with_path(executable_alone, None), Err(BootstrapError::LaneFlagWithoutProfile { flag }) if flag == "--browser-executable")
    {
        return Err("expected --browser-executable without profile refusal".into());
    }
    let headless_alone = ["--browser-headless"].into_iter().map(str::to_string);
    if !matches!(ServeOptions::from_env_with_path(headless_alone, None), Err(BootstrapError::LaneFlagWithoutProfile { flag }) if flag == "--browser-headless")
    {
        return Err("expected --browser-headless without profile refusal".into());
    }

    let refined = [
        "--browser-profile",
        "/tmp/profile",
        "--browser-executable",
        "/usr/bin/chromium",
    ]
    .into_iter()
    .map(str::to_string);
    let lane = ServeOptions::from_env_with_path(refined, None)?
        .lane
        .ok_or("missing configured browser lane")?;
    let executable = lane.executable.to_str();
    if executable != Some("/usr/bin/chromium") {
        return Err(format!(
            "executable: left={executable:?}, right={:?}",
            Some("/usr/bin/chromium")
        )
        .into());
    }
    if !lane.headed {
        return Err("configured executable lane should be headed".into());
    }
    Ok(())
}

#[test]
fn options_refuse_a_lane_whose_default_browser_no_path_entry_holds() -> TestResult {
    let args = || {
        ["--browser-profile", "/tmp/profile"]
            .into_iter()
            .map(str::to_string)
    };
    let refused = |path: Option<&OsStr>| -> TestResult<BootstrapError> {
        match ServeOptions::from_env_with_path(args(), path) {
            Err(error) => Ok(error),
            Ok(_) => Err("lane without a browser accepted".into()),
        }
    };

    let directory = tempfile::tempdir()?;
    std::fs::create_dir(directory.path().join("chromium"))?;
    let error = refused(Some(directory.path().as_os_str()))?;
    if !matches!(&error, BootstrapError::LaneBrowserNotOnPath { program } if program == "chromium")
    {
        return Err(format!("expected chromium not on path, got {error:?}").into());
    }

    let error = refused(None)?;
    if !matches!(&error, BootstrapError::LaneBrowserNotOnPath { program } if program == "chromium")
    {
        return Err(format!("expected chromium not on absent path, got {error:?}").into());
    }
    Ok(())
}

#[test]
fn a_non_loopback_listen_is_refused() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let options = ServeOptions {
                listen: SocketAddr::from(([0, 0, 0, 0], 0)),
                data_dir: dir.path().to_path_buf(),
                ..ServeOptions::default()
            };
            let error = match supervise(options, std::future::pending::<()>()).await {
                Err(error) => error,
                Ok(_) => return Err("non-loopback listen accepted".into()),
            };
            if !matches!(&error, BootstrapError::NonLoopbackListen { .. }) {
                return Err(format!("expected NonLoopbackListen, got {error:?}").into());
            }
            if !error.to_string().contains("is not a loopback address") {
                return Err(
                    format!("refusal does not describe non-loopback address: {error}").into(),
                );
            }
            Ok(())
        })
}

#[test]
fn stop_reason_round_trips_through_its_wire_byte() {
    for reason in [
        StopReason::Signal,
        StopReason::Requested,
        StopReason::ServerExit,
    ] {
        let raw = reason as u8;
        assert_eq!(raw, reason.to_raw());
        assert_eq!(StopReason::from_raw(raw), reason);
    }
    assert_eq!(StopReason::from_raw(200), StopReason::ServerExit);
}

#[test]
fn drain_counts_aborted_tasks_after_the_deadline() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut tasks: JoinSet<()> = JoinSet::new();
            tasks.spawn(async {
                std::future::pending::<()>().await;
            });
            let report = drain(tasks, Duration::from_millis(10)).await?;
            let left = [
                report.accepted,
                report.timed_out,
                report.aborted,
                report.completed,
            ];
            let right = [1, 1, 1, 0];
            if left != right {
                return Err(format!(
                    "accepted/timed_out/aborted/completed: left={left:?}, right={right:?}"
                )
                .into());
            }
            Ok(())
        })
}

#[test]
fn drain_completes_tasks_that_finish_before_the_deadline() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let mut tasks: JoinSet<()> = JoinSet::new();
            tasks.spawn(async {});
            tasks.spawn(async {
                tokio::time::sleep(Duration::from_millis(1)).await;
            });
            let report = drain(tasks, Duration::from_secs(5)).await?;
            let left = [report.accepted, report.completed, report.aborted];
            let right = [2, 2, 0];
            if left != right {
                return Err(
                    format!("accepted/completed/aborted: left={left:?}, right={right:?}").into(),
                );
            }
            Ok(())
        })
}
