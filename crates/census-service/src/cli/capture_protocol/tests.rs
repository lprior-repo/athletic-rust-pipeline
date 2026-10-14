use super::*;
use clap::Parser;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn capture_protocol_parser_requires_a_supported_subcommand() -> TestResult {
    for command in ["init", "show"] {
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
