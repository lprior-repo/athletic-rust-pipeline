use super::*;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn selected_export_job() -> Value {
    json!({
        "contract": "selected_publication_v1",
        "selection": {
            "version": 1,
            "store_identity": "a".repeat(64),
            "profile_id": "b".repeat(64),
        },
    })
}

fn legacy() -> Value {
    json!({
        "season": 2026, "revision": 1,
        "history": {"first_calendar_year": 2023, "last_calendar_year": 2026, "as_of": "2026-10-12"},
        "export_job": selected_export_job(),
        "jurisdictions": ["WI"], "refresh": false, "limit_per_state": null,
        "concurrency": 4, "observed_on": null, "authorized_hosts": [],
        "source_parallelism": 1, "school_address": null,
        "pass_budget": 17, "pass_delay_seconds": 41
    })
}

#[test]
fn missing_publication_contract_defaults_to_selected_v2() -> TestResult {
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
fn selected_contract_carries_a_versioned_profile_and_survives_binding() -> TestResult {
    let mut original = legacy();
    original.as_object_mut().ok_or("request object")?.insert(
        "publication_contract".into(),
        json!("selected_publication_v2"),
    );
    let request: NationalRequest = serde_json::from_value(original.clone())?;
    check!(eq; request.publication_contract, PublicationContract::SelectedPublicationV2);
    let binding = NationalRequestBinding::Bound(Box::new(request));
    let encoded = serde_json::to_value(binding)?;
    check!(eq; encoded, json!({"status": "bound", "request": original}));
    let NationalRequestBinding::Bound(decoded) = serde_json::from_value(encoded)? else {
        return Err("bound request lost its selected contract".into());
    };
    check!(eq; serde_json::to_value(decoded)?, original);
    Ok(())
}

#[test]
fn removed_contracts_are_not_accepted_as_selected() -> TestResult {
    for contract in [
        "legacy_separate_v1",
        "managed_bundle_v2",
        "selected_publication_v1",
    ] {
        let mut original = legacy();
        original
            .as_object_mut()
            .ok_or("request object")?
            .insert("publication_contract".into(), json!(contract));
        check!(serde_json::from_value::<NationalRequest>(original).is_err());
    }
    Ok(())
}

#[test]
fn old_bindings_are_refused_without_an_invented_request() -> TestResult {
    check!(
        serde_json::from_value::<NationalRequestBinding>(json!({"status": "unbound_legacy"}))
            .is_err()
    );
    check!(serde_json::from_value::<NationalRequestBinding>(json!({"status": "bound"})).is_err());
    check!(serde_json::from_value::<NationalRequestBinding>(
        json!({"status": "unbound_legacy", "request": legacy()})
    )
    .is_err());
    Ok(())
}

#[test]
fn unknown_publication_contract_is_not_accepted_as_selected() -> TestResult {
    let mut original = legacy();
    original
        .as_object_mut()
        .ok_or("request object")?
        .insert("publication_contract".into(), json!("managed_bundle_v3"));
    check!(serde_json::from_value::<NationalRequest>(original).is_err());
    Ok(())
}

#[test]
fn missing_export_job_is_refused() -> TestResult {
    let mut original = legacy();
    original
        .as_object_mut()
        .ok_or("request object")?
        .remove("export_job");
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
fn capabilities_advertise_only_the_selected_contract() -> TestResult {
    let current = PublicationCapabilities::current();
    check!(eq; current.publication_contracts, vec![PublicationContract::SelectedPublicationV2]);
    check!(current.supports_national_generation());
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
