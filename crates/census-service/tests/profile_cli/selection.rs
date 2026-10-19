use census_domain::model::{
    GradYear, ProfileId, ProfileSelectionV1, QueryProfileV1, StoreIdentity,
};
use serde_json::Value;

use super::fixture::{equal, profile, TestResult};
use super::harness::{inject, invoke, open_current, path_text, refusal, success, witness};

#[test]
fn profile_cli_prepares_a_selection_without_persisting_or_activating_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let document = directory.path().join("selection.json");
    let baseline = profile()?;
    let mut input = baseline.input().clone();
    input.graduation_years = vec![GradYear::new(2028).ok_or("fixture cohort")?];
    let expected = QueryProfileV1::new(input)?;
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    let identity = {
        let store = open_current(&root)?;
        store.register_profile(&baseline)?;
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
    equal(
        open_current(&root)?.selected_profile(&selection)?,
        expected.clone(),
    )?;
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
    equal(open_current(&root)?.selected_profile(&selection)?, expected)?;
    equal(&witness(&root)?, &before)
}

#[test]
fn profile_cli_foreign_selection_refuses_even_when_the_payload_is_retained_locally() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let document = directory.path().join("foreign.json");
    let expected = profile()?;
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    let actual = {
        let store = open_current(&root)?;
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
    census_store::Store::create_public_identity_qualification_v3(&root)?;
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
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    {
        let store = open_current(&root)?;
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
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    {
        let store = open_current(&root)?;
        store.register_profile(&expected)?;
        store.store_identity()?;
        equal(store.profile(&expected.id()?)?, expected.clone())?;
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
    let selection: ProfileSelectionV1 =
        serde_json::from_value(output.get("selection").ok_or("missing selection")?.clone())?;
    let store = open_current(&root)?;
    equal(selection.store_identity(), &store.store_identity()?)?;
    equal(selection.profile_id(), &expected.id()?)?;
    equal(store.selected_profile(&selection)?, expected)?;
    drop(store);
    equal(&witness(&root)?, &before)
}
