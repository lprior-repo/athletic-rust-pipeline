use census_domain::model::{ProfileId, ProfileSelectionV1, StoreIdentity};
use census_store::Store;
use serde_json::Value;

use super::fixture::{equal, profile, TestResult};
use super::harness::{inject, invoke, path_text, refusal, success, witness, Witness};

#[test]
fn profile_cli_prepares_a_selection_without_persisting_or_activating_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let document = directory.path().join("selection.json");
    let expected = profile()?;
    let identity = {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        let identity = store.store_identity()?;
        store.flush()?;
        identity
    };
    let before = witness(&root)?;
    let output = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--id", expected.id()?.as_str()],
        )?,
        "selection_prepared",
    )?;
    equal(
        output.get("runtime_activation"),
        Some(&serde_json::json!("unavailable")),
    )?;
    let selection: ProfileSelectionV1 = serde_json::from_value(
        output
            .get("selection")
            .ok_or("selection output missing")?
            .clone(),
    )?;
    equal(selection.store_identity(), &identity)?;
    equal(selection.profile_id(), &expected.id()?)?;
    equal(&witness(&root)?, &before)?;
    std::fs::write(&document, serde_json::to_vec(&selection)?)?;
    let rechecked = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--selection", path_text(&document)?],
        )?,
        "selection_prepared",
    )?;
    equal(rechecked, output)?;
    equal(&witness(&root)?, &before)
}

#[test]
fn profile_cli_select_does_not_create_a_missing_installation_identity() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let expected = profile()?;
    let before = {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        store.flush()?;
        Witness::capture(&store)?
    };
    equal(&before.lineage, &Err("missing"))?;
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--id", expected.id()?.as_str()],
        )?,
        "store_identity_missing",
    )?;
    equal(&witness(&root)?, &before)
}

#[test]
fn profile_cli_foreign_selection_refuses_even_when_the_payload_is_retained_locally() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let document = directory.path().join("foreign.json");
    let expected = profile()?;
    let actual = {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        let identity = store.store_identity()?;
        store.flush()?;
        identity
    };
    let alternate = if actual.as_str() == "b".repeat(64) {
        "c"
    } else {
        "b"
    };
    let foreign =
        ProfileSelectionV1::new(StoreIdentity::parse(&alternate.repeat(64))?, expected.id()?);
    std::fs::write(&document, serde_json::to_vec(&foreign)?)?;
    let before = witness(&root)?;
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--selection", path_text(&document)?],
        )?,
        "selection_foreign_store",
    )?;
    equal(&witness(&root)?, &before)
}

#[test]
fn profile_cli_missing_profile_is_a_failure_for_show_and_select() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let missing = ProfileId::parse(&"d".repeat(64))?;
    {
        let store = Store::open_with(&root, 8_388_608)?;
        store.store_identity()?;
        store.flush()?;
    }
    let before = witness(&root)?;
    for action in ["show", "select"] {
        refusal(
            invoke(
                directory.path(),
                Some(&root),
                &["profile", action, "--id", missing.as_str()],
            )?,
            "profile_missing",
        )?;
        equal(&witness(&root)?, &before)?;
    }
    Ok(())
}

#[test]
fn profile_cli_selection_refuses_a_spoofed_retained_payload_without_rewriting_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let expected = profile()?;
    let id = expected.id()?;
    {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        store.store_identity()?;
        store.flush()?;
    }
    let mut payload = serde_json::to_value(expected)?;
    payload
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("as_of".into(), Value::String("2026-10-10".into()));
    let raw =
        serde_json::to_vec(&serde_json::json!({"version":1,"id":id.as_str(),"payload":payload}))?;
    inject(&root, id.as_str(), &raw)?;
    let before = witness(&root)?;
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--id", id.as_str()],
        )?,
        "retained_profile_id_mismatch",
    )?;
    equal(super::harness::metadata(&root, id.as_str())?, raw)?;
    equal(&witness(&root)?, &before)
}

#[test]
fn profile_cli_preserves_historical_store_identity_case_and_refuses_case_rebinding() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let document = directory.path().join("selection.json");
    let expected = profile()?;
    let historical = StoreIdentity::parse(&"A".repeat(64))?;
    {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        store.journal_done("export-lineage-v1", "store-identity", &historical)?;
        store.flush()?;
    }
    let before = witness(&root)?;
    let output = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--id", expected.id()?.as_str()],
        )?,
        "selection_prepared",
    )?;
    let selected: ProfileSelectionV1 =
        serde_json::from_value(output.get("selection").ok_or("missing selection")?.clone())?;
    equal(selected.store_identity(), &historical)?;
    equal(&witness(&root)?, &before)?;
    let lowercase = ProfileSelectionV1::new(StoreIdentity::parse(&"a".repeat(64))?, expected.id()?);
    std::fs::write(&document, serde_json::to_vec(&lowercase)?)?;
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--selection", path_text(&document)?],
        )?,
        "selection_foreign_store",
    )?;
    equal(&witness(&root)?, &before)
}

#[test]
fn profile_cli_opaque_source_and_region_references_never_claim_runtime_activation() -> TestResult {
    use census_domain::model::{
        ContractRefV1, DiscoveryPriorityV1, QueryProfileV1, RegionalMembershipV1,
    };
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let mut input = profile()?.input().clone();
    let region = ContractRefV1::new(1, 4, &"b".repeat(64))?;
    input.regional_membership = RegionalMembershipV1::SourceBackedRegion(region.clone());
    input.discovery_priority = DiscoveryPriorityV1::SourceBackedRegionFirst(region);
    let expected = QueryProfileV1::new(input)?;
    {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        store.store_identity()?;
        store.flush()?;
    }
    let before = witness(&root)?;
    let output = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "select", "--id", expected.id()?.as_str()],
        )?,
        "selection_prepared",
    )?;
    equal(
        output.get("runtime_activation"),
        Some(&serde_json::json!("unavailable")),
    )?;
    let reason = output
        .get("reason")
        .and_then(Value::as_str)
        .ok_or("missing runtime reason")?;
    if !reason
        .contains("source, region, completion, and policy contract resolution is not performed")
    {
        return Err("selection conceals its unresolved runtime-contract boundary".into());
    }
    equal(&witness(&root)?, &before)
}
