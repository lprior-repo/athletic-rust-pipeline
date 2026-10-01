use std::future::Future;
use std::sync::{Arc, Mutex};

use census_domain::school_directory::{Coordinates, ZipCode};
use census_domain::UsJurisdiction;

use super::google::{GeocodeOutcome, GeocodeQuery, GoogleGeocoder};
use super::key::SecretKey;
use super::transport::{Transport, TransportFailure};
use super::usps::{UspsValidator, ValidationOutcome, ValidationQuery};

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
    requests: Arc<Mutex<Vec<(String, Option<String>)>>>,
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

    fn request(&self) -> (String, Option<String>) {
        let requests = self.requests.lock().expect("stub requests");
        requests.first().cloned().expect("one request")
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
            .expect("stub requests")
            .push((url.to_string(), authorization.map(str::to_string)));
        match &self.reply {
            Reply::Body(body) => Ok(body.clone()),
            Reply::Failure(detail) => Err(TransportFailure {
                detail: detail.clone(),
            }),
        }
    }
}

fn run<F: Future>(future: F) -> F::Output {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("runtime")
        .block_on(future)
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
fn missing_credential_refusal_names_the_variable_once() {
    let missing =
        SecretKey::from_env_first(&["SCHOOL_ADDRESS_TEST_ABSENT_KEY"]).expect_err("absent key");
    assert_eq!(missing.name, "SCHOOL_ADDRESS_TEST_ABSENT_KEY");
    assert!(missing
        .to_string()
        .contains("SCHOOL_ADDRESS_TEST_ABSENT_KEY"));
}

#[test]
fn google_request_carries_the_composed_address_and_the_key() {
    let (geocoder, stub) = geocoder_replying(GOOGLE_OK);
    let outcome = run(geocoder.geocode(&springfield_query()));
    assert!(matches!(outcome, GeocodeOutcome::Found { .. }));

    let (url, authorization) = stub.request();
    assert!(authorization.is_none());
    let parsed = reqwest::Url::parse(url.as_str()).expect("absolute url");
    assert_eq!(parsed.host_str(), Some("maps.googleapis.com"));
    let params: std::collections::BTreeMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    assert_eq!(
        params.get("address").map(String::as_str),
        Some("100 Main St, Springfield, IL 62704")
    );
    assert_eq!(params.get("key").map(String::as_str), Some("alpha-key"));
}

#[test]
fn documented_ok_status_yields_typed_coordinates() {
    let (geocoder, _stub) = geocoder_replying(GOOGLE_OK);
    let outcome = run(geocoder.geocode(&springfield_query()));
    let GeocodeOutcome::Found {
        coordinates,
        formatted_address,
        location_type,
    } = outcome
    else {
        panic!("expected a found coordinate");
    };
    assert_eq!(
        coordinates,
        Coordinates::parse("39.7817210", "-89.6501480").expect("documented coordinate")
    );
    assert_eq!(
        formatted_address.as_deref(),
        Some("100 Main St, Springfield, IL 62704, USA")
    );
    assert_eq!(location_type.as_deref(), Some("ROOFTOP"));
}

#[test]
fn documented_google_statuses_map_to_distinct_outcomes() {
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
        let outcome = run(geocoder.geocode(&springfield_query()));
        assert_eq!(outcome, expected, "status {status}");
    }
}

#[test]
fn unrecognized_google_status_is_reported_not_fabricated() {
    let (geocoder, _stub) = geocoder_replying(r#"{"status": "SOMETHING_NEW", "results": []}"#);
    let outcome = run(geocoder.geocode(&springfield_query()));
    assert_eq!(
        outcome,
        GeocodeOutcome::UnknownStatus {
            status: "SOMETHING_NEW".to_string()
        }
    );
}

#[test]
fn ok_without_a_geometry_location_is_unusable() {
    let (geocoder, _stub) =
        geocoder_replying(r#"{"status": "OK", "results": [{"formatted_address": "somewhere"}]}"#);
    let outcome = run(geocoder.geocode(&springfield_query()));
    assert!(matches!(outcome, GeocodeOutcome::Unusable { .. }));
}

#[test]
fn google_transport_failure_never_echoes_the_key() {
    let stub =
        StubTransport::failing("request to https://maps.googleapis.com?key=alpha-key failed");
    let geocoder = GoogleGeocoder::new(SecretKey::new("alpha-key"), stub);
    let outcome = run(geocoder.geocode(&springfield_query()));
    let GeocodeOutcome::Transport { detail } = outcome else {
        panic!("expected a transport failure");
    };
    assert!(!detail.contains("alpha-key"));
    assert!(detail.contains("<redacted>"));
}

#[test]
fn usps_request_carries_the_bearer_token_and_the_address_query() {
    let (validator, stub) = validator_replying(USPS_ADDRESS);
    let outcome = run(validator.validate(&validation_query()));
    assert!(matches!(outcome, ValidationOutcome::Validated { .. }));

    let (url, authorization) = stub.request();
    assert_eq!(authorization.as_deref(), Some("Bearer beta-token"));
    let parsed = reqwest::Url::parse(url.as_str()).expect("absolute url");
    assert_eq!(parsed.host_str(), Some("apis.usps.com"));
    let params: std::collections::BTreeMap<String, String> =
        parsed.query_pairs().into_owned().collect();
    assert_eq!(
        params.get("streetAddress").map(String::as_str),
        Some("100 Main St")
    );
    assert_eq!(params.get("city").map(String::as_str), Some("Springfield"));
    assert_eq!(params.get("state").map(String::as_str), Some("IL"));
    assert_eq!(params.get("zip").map(String::as_str), Some("62704"));
    assert!(!url.contains("beta-token"));
}

#[test]
fn documented_usps_address_body_yields_typed_fields() {
    let (validator, _stub) = validator_replying(USPS_ADDRESS);
    let outcome = run(validator.validate(&validation_query()));
    let ValidationOutcome::Validated {
        street,
        city,
        state,
        zip,
        delivery_point,
        confirmation,
    } = outcome
    else {
        panic!("expected a validated address");
    };
    assert_eq!(
        street.map(|street| street.as_str().to_string()).as_deref(),
        Some("100 Main St")
    );
    assert_eq!(
        city.map(|city| city.as_str().to_string()).as_deref(),
        Some("Springfield")
    );
    assert_eq!(state, Some(UsJurisdiction::Illinois));
    assert_eq!(
        zip,
        Some(ZipCode::of("62704", Some("1234")).expect("documented zip"))
    );
    assert_eq!(delivery_point.as_deref(), Some("123456789"));
    assert_eq!(confirmation.as_deref(), Some("Y"));
}

#[test]
fn documented_usps_error_body_yields_a_rejection() {
    let (validator, _stub) =
        validator_replying(r#"{"error": {"code": "400", "message": "Invalid address"}}"#);
    let outcome = run(validator.validate(&validation_query()));
    assert_eq!(
        outcome,
        ValidationOutcome::Rejected {
            code: Some("400".to_string()),
            message: Some("Invalid address".to_string())
        }
    );
}

#[test]
fn usps_body_without_address_or_error_is_unparsed() {
    let (validator, _stub) = validator_replying(r#"{"unexpected": true}"#);
    let outcome = run(validator.validate(&validation_query()));
    assert!(matches!(outcome, ValidationOutcome::Unparsed { .. }));
}

#[test]
fn usps_transport_failure_never_echoes_the_token() {
    let stub = StubTransport::failing("request rejected: Bearer beta-token was refused");
    let validator = UspsValidator::new(SecretKey::new("beta-token"), stub);
    let outcome = run(validator.validate(&validation_query()));
    let ValidationOutcome::Transport { detail } = outcome else {
        panic!("expected a transport failure");
    };
    assert!(!detail.contains("beta-token"));
    assert!(detail.contains("<redacted>"));
}

#[test]
fn empty_usps_address_fields_are_unparsed_not_validated() {
    let (validator, _stub) = validator_replying(
        r#"{"address": {"streetAddress": "", "city": "", "state": "", "ZIPCode": ""}}"#,
    );
    let outcome = run(validator.validate(&validation_query()));
    assert!(matches!(outcome, ValidationOutcome::Unparsed { .. }));
}
