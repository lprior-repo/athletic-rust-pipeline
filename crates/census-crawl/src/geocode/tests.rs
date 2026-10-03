use std::future::Future;
use std::sync::{Arc, Mutex};

use census_domain::school_directory::{Coordinates, ZipCode};
use census_domain::UsJurisdiction;

use super::google::{GeocodeOutcome, GeocodeQuery, GoogleGeocoder};
use super::key::SecretKey;
use super::transport::{Transport, TransportFailure};
use super::usps::{UspsValidator, ValidationOutcome, ValidationQuery};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
type RecordedRequest = (String, Option<String>);
type RecordedRequests = Arc<Mutex<Vec<RecordedRequest>>>;

const GOOGLE_OK: &str = r#"{
  "status": "OK",
  "results": [
    {
      "formatted_address": "100 Main St, Springfield, IL 62704, USA",
      "geometry": {
        "location": { "lat": 39.781721, "lng": -89.650148 },
        "location_type": "ROOFTOP"
      }
    }
  ]
}"#;

const USPS_ADDRESS: &str = r#"{
  "firm": "",
  "address": {
    "streetAddress": "100 Main St",
    "city": "Springfield",
    "state": "IL",
    "ZIPCode": "62704",
    "ZIPPlus4": "1234"
  },
  "additionalInfo": { "deliveryPoint": "123456789", "DPVConfirmation": "Y" }
}"#;

#[derive(Clone)]
struct StubTransport {
    reply: Reply,
    requests: RecordedRequests,
}

#[derive(Clone)]
enum Reply {
    Body(String),
    Failure(String),
}

impl StubTransport {
    fn replying(body: &str) -> Self {
        Self {
            reply: Reply::Body(body.to_string()),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn failing(detail: &str) -> Self {
        Self {
            reply: Reply::Failure(detail.to_string()),
            requests: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn request(&self) -> TestResult<RecordedRequest> {
        let requests = self
            .requests
            .lock()
            .map_err(|error| format!("stub requests poisoned: {error}"))?;
        Ok(requests.first().cloned().ok_or("one request")?)
    }
}

impl Transport for StubTransport {
    async fn get_json(
        &self,
        url: &str,
        authorization: Option<&str>,
    ) -> Result<String, TransportFailure> {
        self.requests
            .lock()
            .map_err(|error| TransportFailure {
                detail: format!("stub requests poisoned: {error}"),
            })?
            .push((url.to_string(), authorization.map(str::to_string)));
        match &self.reply {
            Reply::Body(body) => Ok(body.clone()),
            Reply::Failure(detail) => Err(TransportFailure {
                detail: detail.clone(),
            }),
        }
    }
}

fn run<F: Future>(future: F) -> TestResult<F::Output> {
    Ok(tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(future))
}

fn springfield_query() -> GeocodeQuery {
    GeocodeQuery {
        street: "100 Main St".to_string(),
        city: "Springfield".to_string(),
        state: "IL".to_string(),
        postal_code: Some("62704".to_string()),
    }
}

fn validation_query() -> ValidationQuery {
    ValidationQuery {
        street: "100 Main St".to_string(),
        city: "Springfield".to_string(),
        state: "IL".to_string(),
        postal_code: "62704".to_string(),
    }
}

fn geocoder_replying(body: &str) -> (GoogleGeocoder<StubTransport>, StubTransport) {
    let stub = StubTransport::replying(body);
    (
        GoogleGeocoder::new(SecretKey::new("alpha-key"), stub.clone()),
        stub,
    )
}

fn validator_replying(body: &str) -> (UspsValidator<StubTransport>, StubTransport) {
    let stub = StubTransport::replying(body);
    (
        UspsValidator::new(SecretKey::new("beta-token"), stub.clone()),
        stub,
    )
}

#[test]
fn secret_key_debug_is_redacted() {
    let key = SecretKey::new("super-secret-key");
    assert_eq!(format!("{key:?}"), "SecretKey(<redacted>)");
    assert!(!format!("{key:?}").contains("super-secret-key"));
}

#[test]
fn missing_credential_refusal_names_the_variable_once() -> TestResult {
    let missing = match SecretKey::from_env_first(&["SCHOOL_ADDRESS_TEST_ABSENT_KEY"]) {
        Err(error) => error,
        Ok(_) => return Err("absent key unexpectedly present".into()),
    };
    check!(eq; missing.name, "SCHOOL_ADDRESS_TEST_ABSENT_KEY");
    check!(missing
        .to_string()
        .contains("SCHOOL_ADDRESS_TEST_ABSENT_KEY"));
    Ok(())
}

#[test]
fn google_request_carries_the_composed_address_and_the_key() -> TestResult {
    let (geocoder, stub) = geocoder_replying(GOOGLE_OK);
    let outcome = run(geocoder.geocode(&springfield_query()))?;
    check!(matches!(outcome, GeocodeOutcome::Found { .. }));

    let (url, authorization) = stub.request()?;
    check!(authorization.is_none());
    let parsed = reqwest::Url::parse(url.as_str())?;
    check!(eq; parsed.host_str(), Some("maps.googleapis.com"));
    let params: std::collections::BTreeMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    check!(eq;
        params.get("address").map(String::as_str),
        Some("100 Main St, Springfield, IL 62704")
    );
    check!(eq; params.get("key").map(String::as_str), Some("alpha-key"));
    Ok(())
}

#[test]
fn documented_ok_status_yields_typed_coordinates() -> TestResult {
    let (geocoder, _stub) = geocoder_replying(GOOGLE_OK);
    let outcome = run(geocoder.geocode(&springfield_query()))?;
    let GeocodeOutcome::Found {
        coordinates,
        formatted_address,
        location_type,
    } = outcome
    else {
        return Err("expected a found coordinate".into());
    };
    check!(eq;
        coordinates,
        Coordinates::parse("39.7817210", "-89.6501480")?
    );
    check!(eq;
        formatted_address.as_deref(),
        Some("100 Main St, Springfield, IL 62704, USA")
    );
    check!(eq; location_type.as_deref(), Some("ROOFTOP"));
    Ok(())
}

#[test]
fn documented_google_statuses_map_to_distinct_outcomes() -> TestResult {
    let cases = [
        ("ZERO_RESULTS", GeocodeOutcome::ZeroResults),
        ("REQUEST_DENIED", GeocodeOutcome::Denied),
        ("INVALID_REQUEST", GeocodeOutcome::InvalidRequest),
        ("OVER_QUERY_LIMIT", GeocodeOutcome::OverQueryLimit),
        ("OVER_DAILY_LIMIT", GeocodeOutcome::OverDailyLimit),
    ];
    for (status, expected) in cases {
        let (geocoder, _stub) =
            geocoder_replying(format!(r#"{{"status": "{status}", "results": []}}"#).as_str());
        let outcome = run(geocoder.geocode(&springfield_query()))?;
        check!(eq; outcome, expected, "status {status}");
    }
    Ok(())
}

#[test]
fn unrecognized_google_status_is_reported_not_fabricated() -> TestResult {
    let (geocoder, _stub) = geocoder_replying(r#"{"status": "SOMETHING_NEW", "results": []}"#);
    let outcome = run(geocoder.geocode(&springfield_query()))?;
    check!(eq;
        outcome,
        GeocodeOutcome::UnknownStatus {
            status: "SOMETHING_NEW".to_string()
        }
    );
    Ok(())
}

#[test]
fn ok_without_a_geometry_location_is_unusable() -> TestResult {
    let (geocoder, _stub) =
        geocoder_replying(r#"{"status": "OK", "results": [{"formatted_address": "somewhere"}]}"#);
    let outcome = run(geocoder.geocode(&springfield_query()))?;
    check!(matches!(outcome, GeocodeOutcome::Unusable { .. }));
    Ok(())
}

#[test]
fn google_transport_failure_never_echoes_the_key() -> TestResult {
    let stub =
        StubTransport::failing("request to https://maps.googleapis.com?key=alpha-key failed");
    let geocoder = GoogleGeocoder::new(SecretKey::new("alpha-key"), stub);
    let outcome = run(geocoder.geocode(&springfield_query()))?;
    let GeocodeOutcome::Transport { detail } = outcome else {
        return Err("expected a transport failure".into());
    };
    check!(!detail.contains("alpha-key"));
    check!(detail.contains("<redacted>"));
    Ok(())
}

#[test]
fn usps_request_carries_the_bearer_token_and_the_address_query() -> TestResult {
    let (validator, stub) = validator_replying(USPS_ADDRESS);
    let outcome = run(validator.validate(&validation_query()))?;
    check!(matches!(outcome, ValidationOutcome::Validated { .. }));

    let (url, authorization) = stub.request()?;
    check!(eq; authorization.as_deref(), Some("Bearer beta-token"));
    let parsed = reqwest::Url::parse(url.as_str())?;
    check!(eq; parsed.host_str(), Some("apis.usps.com"));
    let params: std::collections::BTreeMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    check!(eq;
        params.get("streetAddress").map(String::as_str),
        Some("100 Main St")
    );
    check!(eq; params.get("city").map(String::as_str), Some("Springfield"));
    check!(eq; params.get("state").map(String::as_str), Some("IL"));
    check!(eq; params.get("zip").map(String::as_str), Some("62704"));
    check!(!url.contains("beta-token"));
    Ok(())
}

#[test]
fn documented_usps_address_body_yields_typed_fields() -> TestResult {
    let (validator, _stub) = validator_replying(USPS_ADDRESS);
    let outcome = run(validator.validate(&validation_query()))?;
    let ValidationOutcome::Validated {
        street,
        city,
        state,
        zip,
        delivery_point,
        confirmation,
    } = outcome
    else {
        return Err("expected a validated address".into());
    };
    check!(eq;
        street.map(|street| street.as_str().to_string()).as_deref(),
        Some("100 Main St")
    );
    check!(eq;
        city.map(|city| city.as_str().to_string()).as_deref(),
        Some("Springfield")
    );
    check!(eq; state, Some(UsJurisdiction::Illinois));
    check!(eq;
        zip,
        Some(ZipCode::of("62704", Some("1234"))?)
    );
    check!(eq; delivery_point.as_deref(), Some("123456789"));
    check!(eq; confirmation.as_deref(), Some("Y"));
    Ok(())
}

#[test]
fn documented_usps_error_body_yields_a_rejection() -> TestResult {
    let (validator, _stub) =
        validator_replying(r#"{"error": {"code": "400", "message": "Invalid address"}}"#);
    let outcome = run(validator.validate(&validation_query()))?;
    check!(eq;
        outcome,
        ValidationOutcome::Rejected {
            code: Some("400".to_string()),
            message: Some("Invalid address".to_string())
        }
    );
    Ok(())
}

#[test]
fn usps_body_without_address_or_error_is_unparsed() -> TestResult {
    let (validator, _stub) = validator_replying(r#"{"unexpected": true}"#);
    let outcome = run(validator.validate(&validation_query()))?;
    check!(matches!(outcome, ValidationOutcome::Unparsed { .. }));
    Ok(())
}

#[test]
fn usps_transport_failure_never_echoes_the_token() -> TestResult {
    let stub = StubTransport::failing("request rejected: Bearer beta-token was refused");
    let validator = UspsValidator::new(SecretKey::new("beta-token"), stub);
    let outcome = run(validator.validate(&validation_query()))?;
    let ValidationOutcome::Transport { detail } = outcome else {
        return Err("expected a transport failure".into());
    };
    check!(!detail.contains("beta-token"));
    check!(detail.contains("<redacted>"));
    Ok(())
}

#[test]
fn empty_usps_address_fields_are_unparsed_not_validated() -> TestResult {
    let (validator, _stub) = validator_replying(
        r#"{"address": {"streetAddress": "", "city": "", "state": "", "ZIPCode": ""}}"#,
    );
    let outcome = run(validator.validate(&validation_query()))?;
    check!(matches!(outcome, ValidationOutcome::Unparsed { .. }));
    Ok(())
}
