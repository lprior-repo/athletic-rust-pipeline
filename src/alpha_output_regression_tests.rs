use crate::alpha_normalize::SourceAthlete;
use crate::alpha_output::{validate_public_json, write_outputs, CoverageReport};
use serde_json::json;

#[test]
fn privacy_guard_rejects_postal_phone_email_and_address_values() {
    for value in [
        json!("90210"),
        json!("5551234"),
        json!("a@b"),
        json!("12 Main Street"),
    ] {
        assert!(validate_public_json(&value).is_err(), "accepted private value {value}");
    }
}

#[test]
fn output_uses_profile_list_fallback_and_creates_checkpoint() {
    let directory = tempfile::tempdir().expect("tempdir");
    let athlete = SourceAthlete {
        athlete_id: 90000001,
        athlete_name: "Test Runner".to_owned(),
        profile_urls: vec!["https://athletic.net/athlete/90000001".to_owned()],
        ..Default::default()
    };
    write_outputs(
        directory.path(),
        &[athlete],
        &[],
        &[],
        &CoverageReport::default(),
    )
    .expect("write outputs");
    let csv = std::fs::read_to_string(directory.path().join("athletes.csv")).expect("csv");
    assert!(csv.contains("https://athletic.net/athlete/90000001"));
    assert!(directory.path().join("checkpoint.jsonl").is_file());
}
