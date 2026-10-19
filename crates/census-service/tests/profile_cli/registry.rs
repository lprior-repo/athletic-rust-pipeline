use census_domain::model::{GradYear, QueryProfileV1};
use census_store::{Store, Table, MAX_PROFILE_RECORD_BYTES};
use serde_json::Value;

use super::fixture::{equal, profile, TestResult};
use super::harness::{
    inject, invoke, manifest, metadata, open_current, path_text, refusal, success, witness, Witness,
};

#[test]
fn profile_cli_registration_retains_full_payload_and_equivalent_repeat_is_read_only() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let file = directory.path().join("profile.json");
    let expected = profile()?;
    let id = expected.id()?;
    Store::create_current_qualification_v3(&root)?;
    let before = {
        let store = open_current(&root)?;
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
    let registered_state = {
        let store = open_current(&root)?;
        equal(store.profile(&id)?, expected.clone())?;
        Witness::capture(&store)?
    };
    before.same_evidence(&registered_state)?;
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
    equal(witness(&root)?, retained)?;
    equal(open_current(&root)?.profile(&id)?, expected)
}

#[test]
fn profile_cli_show_returns_each_retained_payload_without_mutation() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let baseline = profile()?;
    let mut input = baseline.input().clone();
    input.graduation_years = vec![GradYear::new(2028).ok_or("fixture cohort")?];
    let changed = QueryProfileV1::new(input)?;
    Store::create_public_identity_qualification_v3(&root)?;
    {
        let store = open_current(&root)?;
        store.register_profile(&baseline)?;
        store.register_profile(&changed)?;
        store.flush()?;
    }
    let before = witness(&root)?;
    for expected in [&changed, &baseline, &changed] {
        let id = expected.id()?;
        let shown = success(
            invoke(
                directory.path(),
                Some(&root),
                &["profile", "show", "--id", id.as_str()],
            )?,
            "shown",
        )?;
        equal(shown.get("profile_id"), Some(&serde_json::json!(id)))?;
        let payload: QueryProfileV1 =
            serde_json::from_value(shown.get("payload").ok_or("missing payload")?.clone())?;
        equal(&payload, expected)?;
        equal(&witness(&root)?, &before)?;
    }
    Ok(())
}

#[test]
fn profile_cli_cohort_change_adds_configuration_without_rebinding_the_run() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let file = directory.path().join("profile.json");
    let baseline = profile()?;
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    let before = {
        let store = open_current(&root)?;
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
    let store = open_current(&root)?;
    equal(store.profile(&baseline.id()?)?, baseline)?;
    equal(store.profile(&changed.id()?)?, changed)
}

#[test]
fn profile_cli_locked_store_refuses_without_falling_back_or_opening_another_root() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("owned-store");
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    let expected = profile()?;
    let store = open_current(&root)?;
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
    equal(&Witness::capture(&store)?, &before)?;
    drop(store);
    equal(witness(&root)?, before)?;
    equal(directory.path().join("var").exists(), false)
}

#[test]
fn profile_cli_reports_retained_corruption_distinctly_and_never_rewrites_it() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("store");
    let expected = profile()?;
    let file = directory.path().join("profile.json");
    std::fs::write(&file, serde_json::to_vec(&expected)?)?;
    let id = expected.id()?;
    census_store::Store::create_public_identity_qualification_v3(&root)?;
    {
        let store = open_current(&root)?;
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
        for command in [
            vec!["profile", "register", "--file", path_text(&file)?],
            vec!["profile", "show", "--id", id.as_str()],
            vec!["profile", "select", "--id", id.as_str()],
        ] {
            refusal(invoke(directory.path(), Some(&root), &command)?, code)?;
            equal(metadata(&root, id.as_str())?, raw.clone())?;
            equal(&witness(&root)?, &before)?;
        }
    }
    Ok(())
}

#[test]
fn profile_cli_operations_refuse_uninitialized_roots_without_creating_store_state() -> TestResult {
    let directory = tempfile::tempdir()?;
    let uninitialized = directory.path().join("uninitialized");
    std::fs::create_dir(&uninitialized)?;
    let sentinel = uninitialized.join("retained");
    std::fs::write(&sentinel, b"unrelated retained bytes")?;
    let file = directory.path().join("profile.json");
    let expected = profile()?;
    let id = expected.id()?;
    std::fs::write(&file, serde_json::to_vec(&expected)?)?;
    for (root, existed) in [
        (directory.path().join("absent"), false),
        (uninitialized, true),
    ] {
        for command in [
            vec!["profile", "register", "--file", path_text(&file)?],
            vec!["profile", "show", "--id", id.as_str()],
            vec!["profile", "select", "--id", id.as_str()],
        ] {
            refusal(
                invoke(directory.path(), Some(&root), &command)?,
                "store_failed",
            )?;
            equal(root.exists(), existed)?;
            if existed {
                let entries = std::fs::read_dir(&root)?
                    .map(|entry| entry.map(|entry| entry.file_name()))
                    .collect::<Result<Vec<_>, _>>()?;
                equal(entries, vec![std::ffi::OsString::from("retained")])?;
                equal(
                    std::fs::read(&sentinel)?,
                    b"unrelated retained bytes".to_vec(),
                )?;
            }
        }
    }
    equal(directory.path().join("var").exists(), false)
}
