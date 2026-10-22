use std::path::PathBuf;
use std::time::Duration;

use census_service::bootstrap::{MemoryAccounting, ServeCaptureProtocol, ServeOptions};
use clap::Parser;

use super::super::{Cli, Command};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn options_for(arguments: &[&str]) -> anyhow::Result<ServeOptions> {
    let cli = Cli::try_parse_from(arguments.iter().copied())?;
    match &cli.command {
        Command::Serve(args) => args.options(cli.store.as_deref()),
        _ => anyhow::bail!("fixture did not select serve"),
    }
}

#[test]
fn serve_defaults_preserve_bootstrap_defaults_without_enrollment() -> TestResult {
    let options = options_for(&["census-service", "serve"])?;
    let defaults = ServeOptions::default();
    check!(eq; options.listen, defaults.listen);
    check!(eq; options.data_dir, defaults.data_dir);
    check!(eq; options.capture_protocol, None);
    check!(eq; options.max_concurrent, defaults.max_concurrent);
    check!(eq; options.drain_timeout, defaults.drain_timeout);
    check!(eq; options.memory_budget_bytes, defaults.memory_budget_bytes);
    check!(eq; options.memory_accounting, defaults.memory_accounting);
    check!(options.lane.is_none());
    Ok(())
}

#[test]
fn serve_forwards_all_endpoint_flags_without_shell_interpretation() -> TestResult {
    let options = options_for(&[
        "census-service",
        "serve",
        "--listen",
        "[::1]:9081",
        "--data-dir",
        "owned store;not-a-command",
        "--capture-protocol",
        "fresh-qualification-v3",
        "--max-concurrent",
        "7",
        "--drain-timeout",
        "41",
        "--memory-budget-gib",
        "3",
        "--memory-cgroup",
        "/census.slice",
        "--browser-profile",
        "browser profile",
        "--browser-executable",
        "browser executable",
        "--browser-headless",
    ])?;
    check!(eq; options.listen, "[::1]:9081".parse::<std::net::SocketAddr>()?);
    check!(eq; options.data_dir, PathBuf::from("owned store;not-a-command"));
    check!(eq; options.capture_protocol, Some(ServeCaptureProtocol::FreshQualificationV3));
    check!(eq; options.max_concurrent, 7);
    check!(eq; options.drain_timeout, Duration::from_secs(41));
    check!(eq; options.memory_budget_bytes, 3 * 1024 * 1024 * 1024);
    check!(eq; options.memory_accounting, MemoryAccounting::CgroupV2 {
        scope: PathBuf::from("/census.slice"),
    });
    let lane = options.lane.ok_or("browser lane missing")?;
    check!(eq; lane.profile_dir, std::path::absolute("browser profile")?);
    check!(eq; lane.executable, std::path::absolute("browser executable")?);
    check!(!lane.headed);
    Ok(())
}

#[test]
fn serve_accepts_the_existing_global_store_selection_on_either_side() -> TestResult {
    for arguments in [
        ["census-service", "--store", "selected root", "serve"],
        ["census-service", "serve", "--store", "selected root"],
    ] {
        let options = options_for(&arguments)?;
        check!(eq; options.data_dir, PathBuf::from("selected root"));
        check!(eq; options.capture_protocol, None);
    }
    Ok(())
}

#[test]
fn serve_refuses_conflicting_store_selectors_before_starting() -> TestResult {
    for arguments in [
        [
            "census-service",
            "--store",
            "first root",
            "serve",
            "--data-dir",
            "second root",
        ],
        [
            "census-service",
            "serve",
            "--store",
            "first root",
            "--data-dir",
            "second root",
        ],
    ] {
        check!(options_for(&arguments).is_err());
    }
    let cli = Cli::try_parse_from(["census-service", "serve", "--data-dir", "second root"])?;
    let Command::Serve(args) = &cli.command else {
        return Err("fixture did not select serve".into());
    };
    check!(args
        .options(Some(std::path::Path::new("first root")))
        .is_err());
    Ok(())
}

#[test]
fn serve_reuses_bootstrap_protocol_resource_and_lane_refusals() -> TestResult {
    for arguments in [
        [
            "census-service",
            "serve",
            "--capture-protocol",
            "production",
        ],
        ["census-service", "serve", "--max-concurrent", "0"],
        ["census-service", "serve", "--max-concurrent", "1025"],
        ["census-service", "serve", "--memory-budget-gib", "0"],
        ["census-service", "serve", "--memory-budget-gib", "1025"],
        ["census-service", "serve", "--drain-timeout", "not-seconds"],
        ["census-service", "serve", "--listen", "not-an-address"],
        [
            "census-service",
            "serve",
            "--browser-executable",
            "chromium",
        ],
    ] {
        check!(options_for(&arguments).is_err());
    }
    check!(options_for(&["census-service", "serve", "--browser-headless"]).is_err());
    Ok(())
}

#[test]
fn serve_help_names_current_options_and_unknown_flags_are_refused() -> TestResult {
    let error = Cli::try_parse_from(["census-service", "serve", "--help"])
        .err()
        .ok_or("help did not return a display response")?;
    check!(eq; error.kind(), clap::error::ErrorKind::DisplayHelp);
    let help = error.to_string();
    for flag in [
        "--listen",
        "--data-dir",
        "--capture-protocol",
        "--max-concurrent",
        "--drain-timeout",
        "--memory-budget-gib",
        "--memory-cgroup",
        "--browser-profile",
        "--browser-executable",
        "--browser-headless",
    ] {
        check!(help.contains(flag));
    }
    check!(Cli::try_parse_from(["census-service", "serve", "--future-worker-flag"]).is_err());
    Ok(())
}
