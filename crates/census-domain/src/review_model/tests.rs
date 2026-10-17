use std::time::Duration;

use super::{ModelContentFailure, ModelEndpointFailure, ModelError, ModelOptions};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn options_json(endpoint: &str, model: &str) -> Result<String, serde_json::Error> {
    Ok(format!(
        r#"{{"endpoint":{},"model":{},"timeout":{{"secs":180,"nanos":0}},"max_tokens":1536,"response_format":"json_schema"}}"#,
        serde_json::to_string(endpoint)?,
        serde_json::to_string(model)?,
    ))
}

#[test]
fn deserialization_refuses_the_same_invalid_endpoints_as_local_construction() -> TestResult {
    for (endpoint, reason) in [
        ("http://example.com:52080", ModelEndpointFailure::IpRequired),
        (
            "http://192.168.1.1:52080",
            ModelEndpointFailure::LoopbackRequired,
        ),
        (
            "https://127.0.0.1:52080",
            ModelEndpointFailure::HttpRequired,
        ),
        (
            "http://127.0.0.1:52080?x=1",
            ModelEndpointFailure::QueryOrFragment,
        ),
        (
            "http://127.0.0.1:0",
            ModelEndpointFailure::NonZeroPortRequired,
        ),
        (
            "http://user@127.0.0.1:52080",
            ModelEndpointFailure::UserInfo,
        ),
        (
            "http://127.0.0.1:52080/v1",
            ModelEndpointFailure::RootPathRequired,
        ),
    ] {
        let expected = ModelError::InvalidEndpoint { reason };
        check!(eq; ModelOptions::local(endpoint, "model"), Err(expected));
        let wire = options_json(endpoint, "model")?;
        let error = match serde_json::from_str::<ModelOptions>(&wire) {
            Err(error) => error,
            Ok(_) => return Err(format!("accepted forged endpoint: {endpoint}").into()),
        };
        check!(eq; error.classify(), serde_json::error::Category::Data);
    }
    Ok(())
}

#[test]
fn deserialization_refuses_blank_and_257_byte_models_without_changing_local_errors() -> TestResult {
    for (model, reason) in [
        (String::new(), ModelEndpointFailure::EmptyModel),
        (
            " \t\r\n\u{2003}".to_string(),
            ModelEndpointFailure::EmptyModel,
        ),
        ("x".repeat(257), ModelEndpointFailure::ModelTooLong),
    ] {
        let expected = ModelError::InvalidEndpoint { reason };
        check!(eq; ModelOptions::local("http://127.0.0.1:52080", &model), Err(expected));
        let wire = options_json("http://127.0.0.1:52080", &model)?;
        let error = match serde_json::from_str::<ModelOptions>(&wire) {
            Err(error) => error,
            Ok(_) => return Err("accepted an invalid model name".into()),
        };
        check!(eq; error.classify(), serde_json::error::Category::Data);
    }
    Ok(())
}

#[test]
fn deserialization_preserves_the_256_byte_model_boundary_and_base_url_normalization() -> TestResult
{
    let model = "x".repeat(256);
    let wire = options_json("http://127.0.0.1:52080/", &model)?;
    let options: ModelOptions = serde_json::from_str(&wire)?;
    check!(eq; options.model_name(), model);
    check!(eq; options.completions_url(), "http://127.0.0.1:52080/v1/chat/completions");
    Ok(())
}

#[test]
fn deserialization_refuses_duplicate_missing_unknown_and_obsolete_fields() -> TestResult {
    let fields = [
        r#""endpoint":"http://127.0.0.1:52080""#,
        r#""model":"model""#,
        r#""timeout":{"secs":180,"nanos":0}"#,
        r#""max_tokens":1536"#,
        r#""response_format":"json_schema""#,
    ];
    for (index, field) in fields.iter().enumerate() {
        let duplicate = format!("{{{},{field}}}", fields.join(","));
        let missing = format!(
            "{{{}}}",
            fields
                .iter()
                .enumerate()
                .filter_map(|(other, value)| (other != index).then_some(*value))
                .collect::<Vec<_>>()
                .join(",")
        );
        for wire in [duplicate, missing] {
            let error = match serde_json::from_str::<ModelOptions>(&wire) {
                Err(error) => error,
                Ok(_) => return Err(format!("accepted invalid fields: {wire}").into()),
            };
            check!(eq; error.classify(), serde_json::error::Category::Data);
        }
    }
    for extra in [r#""extra":42"#, r#""timeout_secs":180"#] {
        let wire = format!("{{{},{extra}}}", fields.join(","));
        let error = match serde_json::from_str::<ModelOptions>(&wire) {
            Err(error) => error,
            Ok(_) => return Err("accepted an unknown options field".into()),
        };
        check!(eq; error.classify(), serde_json::error::Category::Data);
    }
    Ok(())
}

#[test]
fn serde_round_trip_preserves_every_duration_component() -> TestResult {
    for timeout in [
        Duration::ZERO,
        Duration::from_nanos(500),
        Duration::from_millis(100),
        Duration::new(240, 123_456_789),
        Duration::new(u64::MAX, 999_999_999),
    ] {
        let options = ModelOptions::local("http://[::1]:52080/", " model ")?
            .with_timeout(timeout)
            .with_max_tokens(4_097)
            .with_response_format(super::ModelResponseFormat::PromptJson);
        let wire = serde_json::to_vec(&options)?;
        let restored: ModelOptions = serde_json::from_slice(&wire)?;
        drop(wire);
        check!(eq; restored, options);
    }
    Ok(())
}

#[test]
fn known_errors_round_trip_with_the_original_variant_and_fields() -> TestResult {
    for error in [
        ModelError::InvalidEndpoint {
            reason: ModelEndpointFailure::HttpRequired,
        },
        ModelError::RequestTimeout,
        ModelError::RequestFailed,
        ModelError::Status { status: 422 },
        ModelError::Empty,
        ModelError::Content {
            reason: ModelContentFailure::ConfidenceRange,
        },
        ModelError::RequestTooLarge { bytes: 1_048_577 },
        ModelError::ResponseTooLarge { bytes: 262_145 },
    ] {
        let bytes = serde_json::to_vec(&error)?;
        let restored: ModelError = serde_json::from_slice(&bytes)?;
        drop(bytes);
        check!(eq; restored, error);
    }
    Ok(())
}

#[test]
fn unknown_error_reasons_are_refused_instead_of_becoming_generic_failures() -> TestResult {
    for wire in [
        r#"{"invalid_endpoint":{"reason":"remote_is_allowed"}}"#,
        r#"{"content":{"reason":"ignore_invalid_content"}}"#,
    ] {
        let error = match serde_json::from_str::<ModelError>(wire) {
            Err(error) => error,
            Ok(_) => return Err("accepted an unknown model error reason".into()),
        };
        check!(eq; error.classify(), serde_json::error::Category::Data);
    }
    Ok(())
}
