use super::command::Command;
use super::review::{families_of, ReviewArgs};
use super::Cli;
use census_review::ModelResponseFormat;
use clap::{error::ErrorKind, Parser};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn review_args(values: &[&str]) -> TestResult<ReviewArgs> {
    let cli = Cli::try_parse_from(
        ["census-service", "review"]
            .into_iter()
            .chain(values.iter().copied()),
    )?;
    match cli.command {
        Command::Review(args) => Ok(args),
        other => Err(format!("expected review, got {other:?}").into()),
    }
}

#[test]
fn a_single_explicit_format_applies_to_both_endpoints() -> TestResult {
    let args = review_args(&[
        "--response-format",
        "prompt-json",
        "--model",
        "shared-model",
    ])?;
    let lanes = args.lane_options()?;
    check!(eq; lanes.len(), 2);
    check!(lanes
        .iter()
        .all(|lane| lane.response_format() == ModelResponseFormat::PromptJson));
    check!(lanes.iter().all(|lane| lane.model_name() == "shared-model"));
    Ok(())
}

#[test]
fn explicit_formats_and_models_follow_endpoint_order() -> TestResult {
    let args = review_args(&[
        "--endpoint",
        "http://127.0.0.1:18082",
        "--endpoint",
        "http://127.0.0.1:18081",
        "--model",
        "second-model",
        "--model",
        "first-model",
        "--response-format",
        "json-schema",
        "--response-format",
        "prompt-json",
    ])?;
    let lanes = args.lane_options()?;
    let bindings = lanes
        .iter()
        .map(|lane| {
            (
                lane.completions_url(),
                lane.model_name(),
                lane.response_format(),
            )
        })
        .collect::<Vec<_>>();
    check!(eq;
        bindings,
        vec![
            (
                "http://127.0.0.1:18082/v1/chat/completions".to_string(),
                "second-model",
                ModelResponseFormat::JsonSchema
            ),
            (
                "http://127.0.0.1:18081/v1/chat/completions".to_string(),
                "first-model",
                ModelResponseFormat::PromptJson
            ),
        ]
    );
    Ok(())
}

#[test]
fn unequal_explicit_format_counts_are_rejected_before_endpoint_construction() -> TestResult {
    for (endpoints, formats) in [(1, 2), (2, 3), (3, 2)] {
        let mut values = Vec::new();
        for _ in 0..endpoints {
            values.extend(["--endpoint", "http://external.invalid"]);
        }
        for _ in 0..formats {
            values.extend(["--response-format", "prompt-json"]);
        }
        let args = review_args(&values)?;
        let error = match args.validate_configuration() {
            Err(error) => error,
            Ok(_) => return Err("unequal format counts accepted".into()),
        };
        check!(error.to_string().contains("--response-format"), "{error}");
        let error = match args.lane_options() {
            Err(error) => error,
            Ok(_) => return Err("unequal format counts reached endpoint construction".into()),
        };
        check!(error.to_string().contains("--response-format"));
    }
    Ok(())
}

#[test]
fn unknown_response_formats_fail_during_cli_parsing() -> TestResult {
    for mode in ["json", "text", "automatic", "prompt_json", "JSON-SCHEMA"] {
        let error =
            match Cli::try_parse_from(["census-service", "review", "--response-format", mode]) {
                Err(error) => error,
                Ok(_) => return Err(format!("unsupported format {mode} accepted").into()),
            };
        check!(eq; error.kind(), ErrorKind::InvalidValue);
    }
    Ok(())
}

#[test]
fn mismatched_model_counts_are_rejected() -> TestResult {
    let args = review_args(&[
        "--model",
        "a",
        "--model",
        "b",
        "--model",
        "c",
        "--response-format",
        "json-schema",
    ])?;
    let error = match args.validate_configuration() {
        Err(error) => error,
        Ok(_) => return Err("mismatched model counts accepted".into()),
    };
    check!(error.to_string().contains("--model"), "{error}");
    Ok(())
}

#[test]
fn unknown_family_is_refused_by_name() -> TestResult {
    let error = match families_of(&["school-jurisdiction".to_string(), "postcode".to_string()]) {
        Err(error) => error,
        Ok(_) => return Err("unknown family accepted".into()),
    };
    check!(error.to_string().contains("postcode"), "{error}");
    Ok(())
}
