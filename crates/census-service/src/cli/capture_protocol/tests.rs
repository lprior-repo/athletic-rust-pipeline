use super::*;
use clap::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn capture_protocol_parser_requires_a_supported_subcommand() -> TestResult {
    for command in ["init", "init-v2", "show"] {
        let cli = Cli::try_parse_from([
            "census-service",
            "--store",
            "chosen",
            "capture-protocol",
            command,
        ])?;
        check!(matches!(
            cli.command,
            super::super::Command::CaptureProtocol(_)
        ));
    }
    check!(Cli::try_parse_from(["census-service", "capture-protocol", "adopt"]).is_err());
    check!(Cli::try_parse_from(["census-service", "capture-protocol", "init", "--json"]).is_err());
    let error = run_to(
        None,
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Init,
        },
        &mut Vec::new(),
    );
    check!(error.is_err());
    Ok(())
}

#[test]
fn explicit_cli_init_and_show_preserve_the_same_capability() -> TestResult {
    let root = tempfile::tempdir()?;
    let path = root.path().join("fresh");
    let mut initialized = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Init,
        },
        &mut initialized,
    )?;
    let mut shown = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Show,
        },
        &mut shown,
    )?;
    check!(eq; initialized, shown);
    check!(Store::open(&path).is_err());
    check!(run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Init
        },
        &mut Vec::new()
    )
    .is_err());
    Ok(())
}

#[test]
fn explicit_cli_v2_init_and_show_preserve_exact_record_and_refuse_reinitialization() -> TestResult {
    let parent = tempfile::tempdir()?;
    let path = parent.path().join("fresh-v2");
    let mut initialized = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::InitV2,
        },
        &mut initialized,
    )?;
    let record: census_domain::model::CaptureQualificationV2 =
        serde_json::from_slice(&initialized)?;
    let mut expected = serde_json::to_vec_pretty(&record)?;
    expected.push(b'\n');
    check!(eq; initialized, expected);
    let mut shown = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Show,
        },
        &mut shown,
    )?;
    check!(eq; initialized, shown);
    check!(matches!(
        Store::open(&path),
        Err(census_store::StoreError::Capture(
            census_store::CaptureError::CaptureQualificationOptInRequired
        ))
    ));
    check!(matches!(
        Store::open_capture_qualification_v1(&path),
        Err(census_store::StoreError::Capture(
            census_store::CaptureError::CaptureProtocolUnsupported
        ))
    ));
    for command in [CaptureProtocolCommand::Init, CaptureProtocolCommand::InitV2] {
        let mut output = Vec::new();
        let error = run_to(Some(&path), &CaptureProtocolArgs { command }, &mut output)
            .err()
            .ok_or("existing capture root was reinitialized")?;
        check!(matches!(
            error.downcast_ref::<census_store::StoreError>(),
            Some(census_store::StoreError::Capture(
                census_store::CaptureError::CaptureInitExistingStore
            ))
        ));
        check!(output.is_empty());
    }
    let mut after = Vec::new();
    run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::Show,
        },
        &mut after,
    )?;
    check!(eq; initialized, after);
    Ok(())
}

#[test]
fn explicit_v2_cli_does_not_upgrade_existing_v1_qualification() -> TestResult {
    let parent = tempfile::tempdir()?;
    let path = parent.path().join("historical-v1");
    let original = Store::create_capture_qualification_v1(&path)?;
    let mut output = Vec::new();
    let error = run_to(
        Some(&path),
        &CaptureProtocolArgs {
            command: CaptureProtocolCommand::InitV2,
        },
        &mut output,
    )
    .err()
    .ok_or("v1 root was upgraded")?;
    check!(matches!(
        error.downcast_ref::<census_store::StoreError>(),
        Some(census_store::StoreError::Capture(
            census_store::CaptureError::CaptureInitExistingStore
        ))
    ));
    check!(output.is_empty());
    check!(eq; Store::capture_qualification_control_existing(&path)?.show()?, CaptureQualification::V1(original));
    drop(Store::open_capture_qualification_v1(&path)?);
    Ok(())
}
