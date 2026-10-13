use std::collections::BTreeMap;
use std::error::Error;

use super::SchoolAddressJoinRequest;
use crate::restate_services::wire::NationalRequest;
use crate::school_address::{Acquisition, Overrides};

type TestResult = Result<(), Box<dyn Error>>;

fn encodes_as(request: &SchoolAddressJoinRequest, expected: &str) -> TestResult {
    let bytes = serde_json::to_vec(request)?;
    check!(eq; bytes.as_slice(), expected.as_bytes());
    Ok(())
}

fn roundtrips_as(raw: &str, expected: &str) -> TestResult {
    let request: SchoolAddressJoinRequest = serde_json::from_str(raw)?;
    encodes_as(&request, expected)?;
    let restored: SchoolAddressJoinRequest =
        serde_json::from_slice(&serde_json::to_vec(&request)?)?;
    encodes_as(&restored, expected)
}

fn rejects_data(raw: &str) -> TestResult {
    let error = match serde_json::from_str::<SchoolAddressJoinRequest>(raw) {
        Ok(request) => return Err(format!("accepted refused request: {raw}: {request:?}").into()),
        Err(error) => error,
    };
    check!(eq; error.classify(), serde_json::error::Category::Data, "{raw}: {error}");
    Ok(())
}

fn rejects_with(raw: &str, detail: &str) -> TestResult {
    let error = match serde_json::from_str::<SchoolAddressJoinRequest>(raw) {
        Ok(request) => return Err(format!("accepted refused request: {raw}: {request:?}").into()),
        Err(error) => error,
    };
    check!(eq; error.classify(), serde_json::error::Category::Data, "{raw}: {error}");
    check!(error.to_string().contains(detail), "{raw}: {error}");
    Ok(())
}

#[test]
fn source_maps_preserve_flat_shape_and_exact_serialization_order() -> TestResult {
    roundtrips_as(
        r#"{"expected_digest":"digest-7","dates":{"nces-ccd":"2026-10-12"},"urls":{"nces-ccd":"https://nces.ed.gov/ccd"},"generation":"generation-7"}"#,
        r#"{"generation":"generation-7","urls":{"nces-ccd":"https://nces.ed.gov/ccd"},"dates":{"nces-ccd":"2026-10-12"},"expected_digest":"digest-7"}"#,
    )
}

#[test]
fn source_maps_serialize_selectors_in_deterministic_sorted_order() -> TestResult {
    roundtrips_as(
        r#"{"dates":{"nces-pss":"2026-10-11","nces-ccd":"2026-10-12"},"urls":{"nces-pss":"https://nces.ed.gov/pss","nces-ccd":"https://nces.ed.gov/ccd"}}"#,
        r#"{"generation":null,"urls":{"nces-ccd":"https://nces.ed.gov/ccd","nces-pss":"https://nces.ed.gov/pss"},"dates":{"nces-ccd":"2026-10-12","nces-pss":"2026-10-11"}}"#,
    )
}

#[test]
fn acquisition_vectors_preserve_flat_shape_and_exact_serialization_order() -> TestResult {
    roundtrips_as(
        r#"{"expected_digest":"digest-7","acquisitions":[{"url":"https://nces.ed.gov/ccd","path":"ccd.csv","provider":"nces-ccd","entry":7,"retrieved_at":"2026-10-12"}],"generation":"generation-7"}"#,
        r#"{"generation":"generation-7","acquisitions":[{"entry":7,"provider":"nces-ccd","path":"ccd.csv","url":"https://nces.ed.gov/ccd","retrieved_at":"2026-10-12"}],"expected_digest":"digest-7"}"#,
    )
}

#[test]
fn source_pair_constructor_serializes_old_wire_shape() -> TestResult {
    let request = SchoolAddressJoinRequest {
        generation: Some("generation-7".to_string()),
        overrides: Overrides::from_source_pairs(
            BTreeMap::from([(
                "nces-ccd".to_string(),
                "https://nces.ed.gov/ccd".to_string(),
            )]),
            BTreeMap::from([("nces-ccd".to_string(), "2026-10-12".to_string())]),
        )?,
        expected_digest: None,
    };
    encodes_as(
        &request,
        r#"{"generation":"generation-7","urls":{"nces-ccd":"https://nces.ed.gov/ccd"},"dates":{"nces-ccd":"2026-10-12"}}"#,
    )
}

#[test]
fn acquisition_constructor_serializes_new_wire_shape() -> TestResult {
    let request = SchoolAddressJoinRequest {
        generation: None,
        overrides: Overrides::from_acquisitions(vec![Acquisition {
            entry: 7,
            provider: "nces-ccd".to_string(),
            path: "ccd.csv".to_string(),
            url: "https://nces.ed.gov/ccd".to_string(),
            retrieved_at: None,
        }])?,
        expected_digest: Some("digest-7".to_string()),
    };
    encodes_as(
        &request,
        r#"{"generation":null,"acquisitions":[{"entry":7,"provider":"nces-ccd","path":"ccd.csv","url":"https://nces.ed.gov/ccd"}],"expected_digest":"digest-7"}"#,
    )
}

#[test]
fn missing_evidence_defaults_to_empty_acquisition_shape() -> TestResult {
    encodes_as(
        &SchoolAddressJoinRequest::default(),
        r#"{"generation":null,"acquisitions":[]}"#,
    )?;
    roundtrips_as(r#"{}"#, r#"{"generation":null,"acquisitions":[]}"#)
}

#[test]
fn empty_source_map_presence_retains_source_pair_shape() -> TestResult {
    for raw in [
        r#"{"urls":{}}"#,
        r#"{"dates":{}}"#,
        r#"{"dates":{},"urls":{}}"#,
    ] {
        roundtrips_as(raw, r#"{"generation":null,"urls":{},"dates":{}}"#)?;
    }
    Ok(())
}

#[test]
fn optional_null_fields_are_allowed_and_absent_digest_is_omitted() -> TestResult {
    roundtrips_as(
        r#"{"expected_digest":null,"generation":null,"acquisitions":[]}"#,
        r#"{"generation":null,"acquisitions":[]}"#,
    )?;
    roundtrips_as(
        r#"{"expected_digest":null,"generation":null,"urls":{},"dates":{}}"#,
        r#"{"generation":null,"urls":{},"dates":{}}"#,
    )
}

#[test]
fn duplicate_optional_fields_are_rejected_even_when_first_value_is_null() -> TestResult {
    for raw in [
        r#"{"generation":null,"generation":null}"#,
        r#"{"generation":null,"generation":"generation-7"}"#,
        r#"{"generation":"generation-7","generation":null}"#,
    ] {
        rejects_with(raw, "duplicate field `generation`")?;
    }
    for raw in [
        r#"{"expected_digest":null,"expected_digest":null}"#,
        r#"{"expected_digest":null,"expected_digest":"digest-7"}"#,
        r#"{"expected_digest":"digest-7","expected_digest":null}"#,
    ] {
        rejects_with(raw, "duplicate field `expected_digest`")?;
    }
    Ok(())
}

#[test]
fn duplicate_evidence_fields_are_rejected_including_empty_values() -> TestResult {
    for raw in [
        r#"{"urls":{},"urls":{}}"#,
        r#"{"dates":{},"dates":{}}"#,
        r#"{"acquisitions":[],"acquisitions":[]}"#,
    ] {
        rejects_data(raw)?;
    }
    Ok(())
}

#[test]
fn mixed_evidence_shapes_are_rejected_including_empty_values_and_either_order() -> TestResult {
    for raw in [
        r#"{"urls":{},"acquisitions":[]}"#,
        r#"{"acquisitions":[],"urls":{}}"#,
        r#"{"dates":{},"acquisitions":[]}"#,
        r#"{"acquisitions":[],"dates":{}}"#,
        r#"{"generation":null,"urls":{},"dates":{},"acquisitions":[],"expected_digest":null}"#,
    ] {
        rejects_data(raw)?;
    }
    Ok(())
}

#[test]
fn explicit_null_evidence_is_rejected_instead_of_becoming_empty() -> TestResult {
    for raw in [
        r#"{"urls":null}"#,
        r#"{"dates":null}"#,
        r#"{"acquisitions":null}"#,
        r#"{"urls":{},"dates":null}"#,
        r#"{"dates":{},"urls":null}"#,
    ] {
        rejects_data(raw)?;
    }
    Ok(())
}

#[test]
fn wrong_evidence_and_optional_field_types_are_rejected() -> TestResult {
    for raw in [
        r#"{"generation":7}"#,
        r#"{"expected_digest":false}"#,
        r#"{"urls":[]}"#,
        r#"{"dates":[]}"#,
        r#"{"acquisitions":{}}"#,
        r#"{"urls":{"nces-ccd":null}}"#,
        r#"{"dates":{"nces-ccd":7}}"#,
    ] {
        rejects_data(raw)?;
    }
    Ok(())
}

#[test]
fn unknown_top_level_fields_and_nested_overrides_are_rejected() -> TestResult {
    for raw in [
        r#"{"unknown":{}}"#,
        r#"{"overrides":{"acquisitions":[]}}"#,
        r#"{"generation":null,"acquisitions":[],"unexpected":null}"#,
    ] {
        rejects_with(raw, "unknown field")?;
    }
    Ok(())
}

#[test]
fn sixth_key_is_refused_without_reading_its_value() -> TestResult {
    for raw in [
        r#"{"generation":null,"urls":{},"dates":{},"acquisitions":[],"expected_digest":null,"generation":null}"#,
        r#"{"generation":null,"urls":{},"dates":{},"acquisitions":[],"expected_digest":null,"generation":["#,
    ] {
        rejects_with(raw, "accepts at most five fields")?;
    }
    Ok(())
}

#[test]
fn non_object_requests_are_rejected() -> TestResult {
    for raw in [r#"null"#, r#"[]"#, r#"7"#, r#""request""#] {
        rejects_data(raw)?;
    }
    Ok(())
}

#[test]
fn national_request_preserves_nested_old_address_shape() -> TestResult {
    let request: NationalRequest = serde_json::from_str(
        r#"{"season":2026,"revision":1,"history":{"first_calendar_year":2023,"last_calendar_year":2026,"as_of":"2026-10-12"},"school_address":{"generation":"generation-7","urls":{"nces-ccd":"https://nces.ed.gov/ccd"},"dates":{"nces-ccd":"2026-10-12"},"expected_digest":"digest-7"}}"#,
    )?;
    let Some(address) = request.school_address.as_ref() else {
        return Err("national request lost its school address request".into());
    };
    encodes_as(
        address,
        r#"{"generation":"generation-7","urls":{"nces-ccd":"https://nces.ed.gov/ccd"},"dates":{"nces-ccd":"2026-10-12"},"expected_digest":"digest-7"}"#,
    )?;
    let restored: NationalRequest = serde_json::from_slice(&serde_json::to_vec(&request)?)?;
    let Some(address) = restored.school_address.as_ref() else {
        return Err("national roundtrip lost its school address request".into());
    };
    encodes_as(
        address,
        r#"{"generation":"generation-7","urls":{"nces-ccd":"https://nces.ed.gov/ccd"},"dates":{"nces-ccd":"2026-10-12"},"expected_digest":"digest-7"}"#,
    )
}

#[test]
fn national_request_rejects_nested_mixed_address_shapes() -> TestResult {
    let raw = r#"{"season":2026,"revision":1,"history":{"first_calendar_year":2023,"last_calendar_year":2026,"as_of":"2026-10-12"},"school_address":{"urls":{},"acquisitions":[]}}"#;
    let error = match serde_json::from_str::<NationalRequest>(raw) {
        Ok(_) => return Err("national request accepted mixed school address evidence".into()),
        Err(error) => error,
    };
    check!(eq; error.classify(), serde_json::error::Category::Data);
    Ok(())
}

#[test]
fn acquisition_vectors_retain_published_order_and_omit_absent_dates() -> TestResult {
    let raw = r#"{"generation":null,"acquisitions":[{"entry":9,"provider":"nces-ccd","path":"later.csv","url":"https://nces.ed.gov/later"},{"entry":2,"provider":"nces-ccd","path":"earlier.csv","url":"https://nces.ed.gov/earlier"}]}"#;
    roundtrips_as(raw, raw)
}

#[test]
fn shared_evidence_validation_is_applied_before_wire_request_acceptance() -> TestResult {
    for raw in [
        r#"{"urls":{"nces-ccd":"file:///ccd.csv"}}"#,
        r#"{"dates":{"nces-ccd":"2026-02-30"}}"#,
        r#"{"urls":{"nces-ccd":"https://nces.ed.gov/first","nces-ccd":"https://nces.ed.gov/second"}}"#,
        r#"{"acquisitions":[{"entry":7,"provider":"nces-ccd","path":"ccd.csv","url":"file:///ccd.csv"}]}"#,
        r#"{"acquisitions":[{"entry":7,"provider":"nces-ccd","path":"ccd.csv","url":"https://nces.ed.gov/ccd","retrieved_at":"2026-02-30"}]}"#,
        r#"{"acquisitions":[{"entry":7,"provider":"nces-ccd","path":"first.csv","url":"https://nces.ed.gov/first"},{"entry":7,"provider":"nces-ccd","path":"second.csv","url":"https://nces.ed.gov/second"}]}"#,
    ] {
        rejects_data(raw)?;
    }
    Ok(())
}

#[test]
fn national_request_preserves_nested_new_address_shape() -> TestResult {
    let request: NationalRequest = serde_json::from_str(
        r#"{"season":2026,"revision":1,"history":{"first_calendar_year":2023,"last_calendar_year":2026,"as_of":"2026-10-12"},"school_address":{"generation":"generation-7","acquisitions":[{"entry":7,"provider":"nces-ccd","path":"ccd.csv","url":"https://nces.ed.gov/ccd"}]}}"#,
    )?;
    let Some(address) = request.school_address.as_ref() else {
        return Err("national request lost its school address request".into());
    };
    encodes_as(
        address,
        r#"{"generation":"generation-7","acquisitions":[{"entry":7,"provider":"nces-ccd","path":"ccd.csv","url":"https://nces.ed.gov/ccd"}]}"#,
    )
}
