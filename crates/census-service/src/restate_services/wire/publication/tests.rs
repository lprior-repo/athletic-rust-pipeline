use super::*;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn legacy() -> Value {
    json!({
        "season": 2026, "revision": 1,
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "jurisdictions": ["WI"], "refresh": false, "limit_per_state": null,
        "concurrency": 4, "observed_on": null, "authorized_hosts": [],
        "source_parallelism": 1, "school_address": null,
        "pass_budget": 17, "pass_delay_seconds": 41
    })
}

#[test]
fn missing_publication_contract_defaults_to_selected_v2_without_inventing_legacy() -> TestResult {
    let original = legacy();
    let request: NationalRequest = serde_json::from_value(original.clone())?;
    check!(eq; request.publication_contract, PublicationContract::SelectedPublicationV2);
    let mut expected = original.clone();
    expected.as_object_mut().ok_or("request object")?.insert(
        "publication_contract".into(),
        json!("selected_publication_v2"),
    );
    check!(eq; serde_json::to_value(request)?, expected);
    Ok(())
}

#[test]
fn managed_contract_is_explicit_and_survives_original_request_binding() -> TestResult {
    let mut original = legacy();
    original
        .as_object_mut()
        .ok_or("request object")?
        .insert("publication_contract".into(), json!("managed_bundle_v2"));
    let request: NationalRequest = serde_json::from_value(original.clone())?;
    let binding = NationalRequestBinding::Bound(Box::new(request));
    let encoded = serde_json::to_value(binding)?;
    check!(eq; encoded, json!({"status": "bound", "request": original}));
    let NationalRequestBinding::Bound(decoded) = serde_json::from_value(encoded)? else {
        return Err("bound request became legacy".into());
    };
    check!(eq; serde_json::to_value(decoded)?, original);
    Ok(())
}

#[test]
fn selected_contract_carries_a_versioned_profile_without_changing_legacy_shape() -> TestResult {
    let mut original = legacy();
    let selection = census_domain::model::ProfileSelectionV1::new(
        census_domain::model::StoreIdentity::parse(&"a".repeat(64))?,
        census_domain::model::ProfileId::parse(&"b".repeat(64))?,
    );
    let object = original.as_object_mut().ok_or("request object")?;
    object.insert(
        "publication_contract".into(),
        json!("selected_publication_v1"),
    );
    object.insert(
        "export_job".into(),
        serde_json::to_value(
            census_report::export::ExportJobRequest::SelectedPublicationV1 {
                selection: selection.clone(),
            },
        )?,
    );
    let request: NationalRequest = serde_json::from_value(original.clone())?;
    check!(eq; request.publication_contract, PublicationContract::SelectedPublicationV1);
    check!(eq; request.export_job, census_report::export::ExportJobRequest::SelectedPublicationV1 { selection });
    check!(eq; serde_json::to_value(request)?, original);
    Ok(())
}

#[test]
fn unbound_legacy_has_no_invented_request_or_contract() -> TestResult {
    check!(eq; serde_json::to_value(NationalRequestBinding::UnboundLegacy)?, json!({"status": "unbound_legacy"}));
    check!(serde_json::from_value::<NationalRequestBinding>(json!({"status": "bound"})).is_err());
    check!(serde_json::from_value::<NationalRequestBinding>(
        json!({"status": "unbound_legacy", "request": legacy()})
    )
    .is_err());
    Ok(())
}

#[test]
fn unknown_publication_contract_is_not_accepted_as_legacy() -> TestResult {
    let mut original = legacy();
    original
        .as_object_mut()
        .ok_or("request object")?
        .insert("publication_contract".into(), json!("managed_bundle_v3"));
    check!(serde_json::from_value::<NationalRequest>(original).is_err());
    Ok(())
}

#[test]
fn publication_capabilities_refuse_malformed_or_unknown_contracts() -> TestResult {
    for malformed in [
        json!({}),
        json!({"publication_contracts": ["unknown"]}),
        json!({"publication_contracts": [], "fallback": true}),
    ] {
        check!(serde_json::from_value::<PublicationCapabilities>(malformed).is_err());
    }
    Ok(())
}

#[test]
fn pending_preflight_exposes_no_resumable_request() -> TestResult {
    let encoded = serde_json::to_value(NationalRequestBinding::PendingPreflight)?;
    check!(eq; encoded, json!({"status": "pending_preflight"}));
    check!(matches!(
        serde_json::from_value::<NationalRequestBinding>(encoded)?,
        NationalRequestBinding::PendingPreflight
    ));
    check!(serde_json::from_value::<NationalRequestBinding>(
        json!({"status": "pending_preflight", "request": legacy()})
    )
    .is_err());
    Ok(())
}

#[test]
fn explicit_legacy_contract_survives_request_and_binding_round_trips() -> TestResult {
    let mut original = legacy();
    original
        .as_object_mut()
        .ok_or("request object")?
        .insert("publication_contract".into(), json!("legacy_separate_v1"));
    let request: NationalRequest = serde_json::from_value(original.clone())?;
    check!(eq; request.publication_contract, PublicationContract::LegacySeparateV1);
    let encoded = serde_json::to_value(&request)?;
    check!(eq; encoded, original);
    let decoded: NationalRequest = serde_json::from_value(encoded)?;
    check!(eq; decoded.publication_contract, PublicationContract::LegacySeparateV1);
    let binding = NationalRequestBinding::Bound(Box::new(request));
    let encoded = serde_json::to_value(binding)?;
    check!(eq; encoded, json!({"status": "bound", "request": original}));
    let NationalRequestBinding::Bound(decoded) = serde_json::from_value(encoded)? else {
        return Err("bound request lost its original contract".into());
    };
    check!(eq; decoded.publication_contract, PublicationContract::LegacySeparateV1);
    Ok(())
}
