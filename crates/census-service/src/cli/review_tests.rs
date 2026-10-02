use super::command::Command;
use super::review::{families_of, ReviewArgs};
use super::Cli;
use clap::{error::ErrorKind, Parser};

fn review_args(values: &[&str]) -> ReviewArgs {
    let cli = Cli::try_parse_from(
        ["census-service", "review"]
            .into_iter()
            .chain(values.iter().copied()),
    )
    .expect("review arguments parse");
    match cli.command {
        Command::Review(args) => args,
        other => panic!("expected review, got {other:?}"),
    }
}

#[test]
fn unequal_explicit_format_counts_are_rejected_before_endpoint_construction() {
    for (endpoints, formats) in [(1, 2), (2, 3), (3, 2)] {
        let mut values = Vec::new();
        for _ in 0..endpoints {
            values.extend(["--endpoint", "http://external.invalid"]);
        }
        for _ in 0..formats {
            values.extend(["--response-format", "prompt-json"]);
        }
        let args = review_args(&values);
        let error = args
            .validate_configuration()
            .expect_err("unequal counts rejected");
        assert!(error.to_string().contains("--response-format"), "{error}");
        assert!(args
            .lane_options()
            .expect_err("fails before endpoint parsing")
            .to_string()
            .contains("--response-format"));
    }
}

#[test]
fn unknown_response_formats_fail_during_cli_parsing() {
    for mode in ["json", "text", "automatic", "prompt_json", "JSON-SCHEMA"] {
        let error = Cli::try_parse_from(["census-service", "review", "--response-format", mode])
            .expect_err("only explicit supported modes are admitted");
        assert_eq!(error.kind(), ErrorKind::InvalidValue);
    }
}

#[test]
fn mismatched_model_counts_are_rejected() {
    let args = review_args(&[
        "--model",
        "a",
        "--model",
        "b",
        "--model",
        "c",
        "--response-format",
        "json-schema",
    ]);
    let error = args
        .validate_configuration()
        .expect_err("three models for two endpoints");
    assert!(error.to_string().contains("--model"), "{error}");
}

#[test]
fn unknown_family_is_refused_by_name() {
    let error = families_of(&["school-jurisdiction".to_string(), "postcode".to_string()])
        .expect_err("unknown family");
    assert!(error.to_string().contains("postcode"), "{error}");
}
