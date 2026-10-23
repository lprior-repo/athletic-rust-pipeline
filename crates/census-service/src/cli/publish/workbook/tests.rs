use super::*;
use clap::Parser;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn arguments(extra: &[&str]) -> Vec<String> {
    [
        "census-service",
        "--store",
        "/stopped/store",
        "workbook",
        "--school-year",
        "2026",
        "--selected-store-identity",
        &"a".repeat(64),
        "--selected-profile-id",
        &"b".repeat(64),
    ]
    .into_iter()
    .chain(extra.iter().copied())
    .map(str::to_owned)
    .collect()
}

fn parsed(extra: &[&str]) -> TestResult<WorkbookArgs> {
    let cli = Cli::try_parse_from(arguments(extra))?;
    match cli.command {
        crate::cli::command::Command::Workbook(args) => Ok(args),
        _ => Err("wrong command".into()),
    }
}

#[test]
fn frozen_resume_requires_explicit_generation_and_selection() -> TestResult {
    check!(Cli::try_parse_from(arguments(&["--resume-frozen"])).is_err());
    let args = parsed(&["--resume-frozen", "--generation", "g0"])?;
    check!(args.resume_frozen);
    check!(!parsed(&["--generation", "g0"])?.resume_frozen);
    check!(Cli::try_parse_from([
        "census-service",
        "--store",
        "/stopped/store",
        "workbook",
        "--school-year",
        "2026",
        "--resume-frozen",
        "--generation",
        "g0",
    ])
    .is_err());
    Ok(())
}

#[test]
fn frozen_resume_uses_raw_native_request_options_before_profile_defaults() -> TestResult {
    let args = parsed(&["--resume-frozen", "--generation", "g0"])?;
    let request = publication_preflight::selected_request(
        args.selected_store_identity.as_deref(),
        args.selected_profile_id.as_deref(),
    )?;
    let options = workbook::Options {
        grad_year: None,
        out: None,
        limit: None,
        scope: scope_of(false),
        jurisdictions: Vec::new(),
        school_year: contact_school_year(2026)?,
    };
    let native = workbook_request(&options, &request);
    check!(native.grad_year.is_none());
    check!(native.out.is_none());
    check!(native.jurisdictions.is_empty());
    let key = workbook_request_key(&native, &ExportGeneration::parse("g0")?)?;
    check!(key.starts_with("workbook:all:all_sources:all:all:.:2026:selected_v1_"));
    check!(key.ends_with(":g0"));
    Ok(())
}

#[test]
fn derivation_requires_frozen_resume_and_preserves_original_request_root() -> TestResult {
    check!(Cli::try_parse_from(arguments(&[
        "--derive-out",
        "/publication/derived",
        "--generation",
        "g0"
    ]))
    .is_err());
    let args = parsed(&[
        "--resume-frozen",
        "--generation",
        "g0",
        "--out",
        "/publication/original",
        "--derive-out",
        "/publication/derived",
    ])?;
    check!(eq; args.out.as_deref(), Some(std::path::Path::new("/publication/original")));
    check!(eq; args.derive_out.as_deref(), Some(std::path::Path::new("/publication/derived")));
    Ok(())
}

#[tokio::test]
async fn frozen_resume_refuses_live_route_before_network_submission() -> TestResult {
    let mut argv = arguments(&["--resume-frozen", "--generation", "g0"]);
    argv.drain(1..3);
    let cli = Cli::try_parse_from(argv)?;
    let crate::cli::command::Command::Workbook(args) = &cli.command else {
        return Err("wrong command".into());
    };
    let error = run_workbook(&cli, args)
        .await
        .err()
        .ok_or("live resume accepted")?;
    check!(error.to_string().contains("requires explicit --store"));
    Ok(())
}

#[test]
fn job_recovery_requires_original_selection_output_and_excludes_other_publication_routes(
) -> TestResult {
    let recovery = [
        "--recover-job",
        "workbook:original-failed",
        "--out",
        "/original/publication",
    ];
    check!(eq; parsed(&recovery)?.recover_job.as_deref(), Some("workbook:original-failed"));
    check!(Cli::try_parse_from(arguments(&["--recover-job", "workbook:original-failed"])).is_err());
    for incompatible in [
        vec!["--generation", "new-generation"],
        vec!["--resume-frozen", "--generation", "old-generation"],
        vec!["--derive-out", "/other/publication"],
        vec!["--ingress", "http://127.0.0.1:8080"],
    ] {
        let extra: Vec<_> = recovery.iter().copied().chain(incompatible).collect();
        check!(Cli::try_parse_from(arguments(&extra)).is_err());
    }
    let mut missing_selection = arguments(&recovery);
    missing_selection.drain(6..10);
    check!(Cli::try_parse_from(missing_selection).is_err());
    for invalid in [
        "",
        "workbook:",
        "foreign:job",
        &format!("workbook:{}", "x".repeat(4096)),
    ] {
        check!(recovery_job(invalid).is_err());
    }
    Ok(())
}

#[tokio::test]
async fn job_recovery_refuses_live_route_before_network_or_store_access() -> TestResult {
    let mut argv = arguments(&[
        "--recover-job",
        "workbook:original-failed",
        "--out",
        "/original/publication",
    ]);
    argv.drain(1..3);
    let cli = Cli::try_parse_from(argv)?;
    let crate::cli::command::Command::Workbook(args) = &cli.command else {
        return Err("wrong command".into());
    };
    let error = run_workbook(&cli, args)
        .await
        .err()
        .ok_or("live recovery accepted")?;
    check!(error
        .to_string()
        .contains("--recover-job requires explicit --store"));
    Ok(())
}
