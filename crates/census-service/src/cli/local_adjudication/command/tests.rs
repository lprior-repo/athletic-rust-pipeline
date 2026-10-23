use super::*;
use crate::cli::{Cli, Command};
use census_store::{PublicIdentityError, StoreError};
use clap::{error::ErrorKind, Parser};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn frozen_identity_demand_cli_is_explicitly_offline_and_has_no_cap_override() -> TestResult {
    let cli = Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "measure-frozen-public-identity",
    ])?;
    let Command::LocalIdentityAdjudication(ref args) = cli.command else {
        return Err("wrong diagnostic command".into());
    };
    check!(matches!(
        args.command,
        LocalCommand::MeasureFrozenPublicIdentity
    ));
    let error = run(&cli, args)
        .err()
        .ok_or("missing explicit store was accepted")?;
    check!(error.to_string().contains("requires explicit --store"));
    for extra in [
        "--yes",
        "--publish",
        "--max-bytes",
        "--operation",
        "--ingress",
    ] {
        check!(Cli::try_parse_from([
            "census-service",
            "--store",
            "synthetic-stopped-store",
            "local-identity-adjudication",
            "measure-frozen-public-identity",
            extra,
        ])
        .is_err());
    }
    Ok(())
}

#[test]
fn retained_demand_cli_requires_original_operation_and_explicit_year() -> TestResult {
    let cli = Cli::try_parse_from([
        "census-service",
        "--store",
        "synthetic-offline-store",
        "local-identity-adjudication",
        "measure-retained-source-cohort",
        "--operation",
        "original-operation",
        "--graduation-year",
        "2027",
    ])?;
    check!(matches!(
        cli.command,
        Command::LocalIdentityAdjudication(LocalAdjudicationArgs {
            command: LocalCommand::MeasureRetainedSourceCohort {
                graduation_year: 2027,
                ..
            }
        })
    ));
    check!(Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "measure-retained-source-cohort",
        "--operation",
        "original-operation",
    ])
    .is_err());
    Ok(())
}

#[test]
fn local_identity_cli_registers_inspect_split_and_receipt_without_live_ingress() -> TestResult {
    let canonical = format!("ath_public_v1_{}", "1".repeat(64));
    let cli = Cli::try_parse_from([
        "census-service",
        "--store",
        "synthetic-offline-store",
        "local-identity-adjudication",
        "inspect",
        "--operation",
        "synthetic-split",
        "--canonical",
        &canonical,
    ])?;
    check!(eq; cli.store.as_deref(), Some(Path::new("synthetic-offline-store")));
    check!(matches!(
        cli.command,
        Command::LocalIdentityAdjudication(LocalAdjudicationArgs {
            command: LocalCommand::Inspect { .. }
        })
    ));
    let cli = Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "split",
        "--operation",
        "synthetic-split",
        "--file",
        "synthetic-request.json",
    ])?;
    check!(
        matches!(cli.command, Command::LocalIdentityAdjudication(LocalAdjudicationArgs {
        command: LocalCommand::Split { operation, file }
    }) if operation == "synthetic-split" && file == Path::new("synthetic-request.json"))
    );
    let cli = Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "receipt",
        "--operation",
        "synthetic-split",
        "--approval-digest",
        &"a".repeat(64),
    ])?;
    check!(matches!(
        cli.command,
        Command::LocalIdentityAdjudication(LocalAdjudicationArgs {
            command: LocalCommand::Receipt { .. }
        })
    ));
    Ok(())
}

#[test]
fn local_identity_cli_refuses_automation_and_remote_approval_switches() -> TestResult {
    for extra in [
        "--yes",
        "--approve",
        "--approval-digest",
        "--ingress",
        "--actor",
    ] {
        let error = match Cli::try_parse_from([
            "census-service",
            "local-identity-adjudication",
            "split",
            "--operation",
            "synthetic-split",
            "--file",
            "synthetic-request.json",
            extra,
        ]) {
            Err(error) => error,
            Ok(_) => return Err(format!("unsupported bypass accepted: {extra}").into()),
        };
        check!(eq; error.kind(), ErrorKind::UnknownArgument);
    }
    Ok(())
}

#[test]
fn local_identity_cli_requires_an_operation_and_request_file() -> TestResult {
    for [flag, value] in [
        ["--operation", "synthetic-split"],
        ["--file", "synthetic-request.json"],
    ] {
        let error = match Cli::try_parse_from([
            "census-service",
            "local-identity-adjudication",
            "split",
            flag,
            value,
        ]) {
            Err(error) => error,
            Ok(_) => return Err("incomplete split command accepted".into()),
        };
        check!(eq; error.kind(), ErrorKind::MissingRequiredArgument);
    }
    Ok(())
}

#[test]
fn graph_inventory_requires_explicit_operation_before_staging() -> TestResult {
    let canonical = format!("ath_public_v1_{}", "1".repeat(64));
    let error = match Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "inspect",
        "--canonical",
        &canonical,
    ]) {
        Err(error) => error,
        Ok(_) => return Err("graph inventory accepted an implicit operation".into()),
    };
    check!(eq; error.kind(), ErrorKind::MissingRequiredArgument);
    Ok(())
}

#[test]
fn local_identity_cli_requires_explicit_existing_store_without_initializing_a_root() -> TestResult {
    let error = match require_root(None) {
        Err(error) => error,
        Ok(_) => return Err("implicit store accepted".into()),
    };
    check!(error.to_string().contains("requires explicit --store"));
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("absent");
    let error = match open(&missing) {
        Err(error) => error,
        Ok(_) => return Err("missing store initialized".into()),
    };
    check!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Io { .. })
    ));
    check!(!missing.exists());
    Ok(())
}

#[test]
fn local_identity_cli_refuses_legacy_store_and_an_existing_owner() -> TestResult {
    let directory = tempfile::tempdir()?;
    let legacy = directory.path().join("legacy");
    drop(Store::open(&legacy)?);
    let error = match open(&legacy) {
        Err(error) => error,
        Ok(_) => return Err("legacy store admitted".into()),
    };
    check!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::PublicIdentity(
            PublicIdentityError::OptInRequired
        ))
    ));
    let qualified = directory.path().join("qualified");
    Store::create_public_identity_qualification_v3(&qualified)?;
    let (store, writer) = open(&qualified)?;
    let head = writer.head(&store)?;
    let error = match open(&qualified) {
        Err(error) => error,
        Ok(_) => return Err("simultaneous owner admitted".into()),
    };
    check!(matches!(
        error.downcast_ref::<StoreError>(),
        Some(StoreError::Open { .. })
    ));
    check!(eq; writer.head(&store)?, head);
    Ok(())
}

#[test]
fn local_identity_cli_rejects_malformed_request_before_opening_a_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let missing = directory.path().join("absent");
    let input = directory.path().join("invalid.json");
    std::fs::write(&input, b"{\"actor\":\"model\"}")?;
    let error = match split(&missing, "synthetic-split", &input) {
        Err(error) => error,
        Ok(()) => return Err("malformed request accepted".into()),
    };
    check!(matches!(
        error.downcast_ref::<input::InputError>(),
        Some(input::InputError::Json(_))
    ));
    check!(!missing.exists());
    Ok(())
}

#[test]
fn graph_enrollment_is_explicit_and_does_not_alias_historical_command() -> TestResult {
    let cli = Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "enroll-graph-admission",
    ])?;
    check!(matches!(
        cli.command,
        Command::LocalIdentityAdjudication(LocalAdjudicationArgs {
            command: LocalCommand::EnrollGraphAdmission,
        })
    ));
    let error = match Cli::try_parse_from([
        "census-service",
        "local-identity-adjudication",
        "enroll-context-budget",
    ]) {
        Err(error) => error,
        Ok(_) => return Err("historical enrollment command was aliased".into()),
    };
    check!(eq; error.kind(), ErrorKind::InvalidSubcommand);
    Ok(())
}

#[test]
fn graph_enrollment_repeats_without_charging_or_changing_head() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("graph-enrollment");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = open(&root)?;
    let head = writer.head(&store)?;
    enroll_graph_admission(&store, &writer)?;
    let sequence = store.snapshot().sequence();
    let revisions = store.snapshot().public_identity_input_revisions_v2()?;
    check!(matches!(
        revisions,
        census_store::PublicIdentityInputRevisionsV2::Enabled(_)
    ));
    enroll_graph_admission(&store, &writer)?;
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, revisions);
    check!(eq; writer.head(&store)?, head);
    Ok(())
}

#[test]
fn stale_graph_enrollment_refuses_then_retries_same_store() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("graph-enrollment");
    Store::create_public_identity_qualification_v3(&root)?;
    let (store, writer) = open(&root)?;
    let stale = store.snapshot();
    let (school, _) = census_domain::model::CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Retained enrollment school",
        "retained enrollment school",
        None,
    );
    store.append(census_store::Table::Schools, &school)?;
    let after_source = store.snapshot().sequence();
    check!(matches!(
        writer.enroll_graph_admission_v1(&store, stale.revision()),
        Err(StoreError::RevisionMoved { .. })
    ));
    check!(eq; store.snapshot().sequence(), after_source);
    check!(eq; store.snapshot().public_identity_input_revisions_v2()?, census_store::PublicIdentityInputRevisionsV2::Disabled);
    enroll_graph_admission(&store, &writer)?;
    check!(matches!(
        store.snapshot().public_identity_input_revisions_v2()?,
        census_store::PublicIdentityInputRevisionsV2::Enabled(_)
    ));
    check!(eq; writer.head(&store)?.sequence, 0);
    Ok(())
}
