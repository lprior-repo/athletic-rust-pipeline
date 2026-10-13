use census_domain::model::{GradYear, QueryProfileV1};
use census_store::{Store, Table, MAX_PROFILE_RECORD_BYTES};
use serde_json::Value;

use super::fixture::{equal, profile, TestResult};
use super::harness::{
    inject, invoke, manifest, metadata, path_text, refusal, success, witness, Witness,
};

#[test]
fn profile_cli_register_show_and_equivalent_repeat_preserve_existing_evidence_and_run() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let file = directory.path().join("profile.json");
    let expected = profile()?;
    let id = expected.id()?;
    let before = {
        let store = Store::open_with(&root, 8_388_608)?;
        store.append(
            Table::Schools,
            &serde_json::json!({"id":"retained-school","name":"Retained"}),
        )?;
        store.bind_run(&manifest(&store)?)?;
        store.flush()?;
        Witness::capture(&store)?
    };
    std::fs::write(&file, serde_json::to_vec(&expected)?)?;
    let registered = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "register", "--file", path_text(&file)?],
        )?,
        "registered",
    )?;
    equal(
        registered.get("profile_id"),
        Some(&Value::String(id.to_string())),
    )?;
    let registered_state = witness(&root)?;
    before.same_evidence(&registered_state)?;
    equal(
        registered_state.sequence,
        before
            .sequence
            .checked_add(1)
            .ok_or("fixture sequence overflow")?,
    )?;
    let shown = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "show", "--id", id.as_str()],
        )?,
        "shown",
    )?;
    let payload: QueryProfileV1 =
        serde_json::from_value(shown.get("payload").ok_or("missing payload")?.clone())?;
    equal(payload, expected.clone())?;
    let retained_bytes = metadata(&root, id.as_str())?;
    let retained = witness(&root)?;
    let mut equivalent = expected.input().clone();
    equivalent.jurisdictions.reverse();
    equivalent
        .jurisdictions
        .push(census_domain::UsJurisdiction::Michigan);
    std::fs::write(&file, serde_json::to_vec(&equivalent)?)?;
    let repeated = success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "register", "--file", path_text(&file)?],
        )?,
        "repeated",
    )?;
    equal(
        repeated.get("profile_id"),
        Some(&Value::String(id.to_string())),
    )?;
    equal(metadata(&root, id.as_str())?, retained_bytes)?;
    equal(witness(&root)?, retained)
}

#[test]
fn profile_cli_cohort_change_adds_configuration_without_rebinding_the_legacy_run() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let file = directory.path().join("profile.json");
    let baseline = profile()?;
    let before = {
        let store = Store::open_with(&root, 8_388_608)?;
        store.bind_run(&manifest(&store)?)?;
        store.register_profile(&baseline)?;
        store.flush()?;
        Witness::capture(&store)?
    };
    let mut input = baseline.input().clone();
    input.graduation_years = vec![GradYear::new(2028).ok_or("fixture cohort")?];
    let changed = QueryProfileV1::new(input)?;
    std::fs::write(&file, serde_json::to_vec(&changed)?)?;
    success(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "register", "--file", path_text(&file)?],
        )?,
        "registered",
    )?;
    before.same_evidence(&witness(&root)?)?;
    let store = Store::open_with(&root, 8_388_608)?;
    equal(store.profile(&baseline.id()?)?, baseline)?;
    equal(store.profile(&changed.id()?)?, changed)
}

#[test]
fn profile_cli_locked_store_refuses_without_falling_back_or_opening_another_root() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("owned-store");
    let store = Store::open_with(&root, 8_388_608)?;
    let expected = profile()?;
    store.register_profile(&expected)?;
    store.flush()?;
    let before = Witness::capture(&store)?;
    refusal(
        invoke(
            directory.path(),
            Some(&root),
            &["profile", "show", "--id", expected.id()?.as_str()],
        )?,
        "store_open_failed",
    )?;
    equal(Witness::capture(&store)?, before)?;
    equal(directory.path().join("var").exists(), false)
}

#[test]
fn profile_cli_reports_retained_corruption_distinctly_and_never_rewrites_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let expected = profile()?;
    let id = expected.id()?;
    {
        let store = Store::open_with(&root, 8_388_608)?;
        store.register_profile(&expected)?;
        store.flush()?;
    }
    let extra = MAX_PROFILE_RECORD_BYTES
        .checked_add(1)
        .ok_or("fixture size overflow")?;
    for (raw, code) in [
        (b"bad JSON".to_vec(), "retained_profile_malformed"),
        (
            br#"{"version":2}"#.to_vec(),
            "retained_profile_version_unsupported",
        ),
        (vec![b' '; extra], "retained_profile_oversized"),
    ] {
        inject(&root, id.as_str(), &raw)?;
        let before = witness(&root)?;
        refusal(
            invoke(
                directory.path(),
                Some(&root),
                &["profile", "show", "--id", id.as_str()],
            )?,
            code,
        )?;
        equal(metadata(&root, id.as_str())?, raw)?;
        equal(witness(&root)?, before)?;
    }
    Ok(())
}
