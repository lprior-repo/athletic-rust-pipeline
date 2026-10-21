use super::*;
use crate::model::{ProfileDate, PublicSourceResourceV1, SchoolYear};
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const TARGET: &str = "https://mi.milesplit.com/athletes/13749840-brennan-smith";
const WHOLE_MEET: &str = "https://mi.milesplit.com/api/v1/meets/123/performances?fields=first";
const ORIGINAL: &str = r#"{"version":1,"slot":{"version":1,"store_identity":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","owner_key":"jurisdiction:MI:2026-27:1","stage":"results","historical_year":2026,"source":"milesplit","source_site":"MI","source_meet_id":"123","resource_role":"structured_whole_meet"},"run":{"season":2026,"revision":1},"run_manifest_digest":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","history":{"first_calendar_year":2023,"last_calendar_year":2026,"as_of":"2026-10-14"},"source_admission":{"kind":"legacy_restate_source_selection_v1","state_key":"state","plan_fingerprint":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb","selected_source":"milesplit","selected_arm":"milesplit_results","durable_role":"structured_whole_meet"},"request":{"method":"GET","original_url":"https://mi.milesplit.com/api/v1/meets/123/performances?fields=first","body":"empty","representation_headers":[],"transport_recipe":"direct_http_one_shot_v1","effective_user_agent":"Census/1.0"},"cache_intent":"reuse_eligible","protocol":"source_acquisition_attempt_v1"}"#;

fn identity() -> TestResult<StoreIdentity> {
    Ok(StoreIdentity::parse(&"A".repeat(64))?)
}

fn results_scope(first: u16, as_of: &str, plan: &str) -> TestResult<ResultsCaptureScopeV1> {
    Ok(ResultsCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
        AcquisitionDigest::parse(&"a".repeat(64))?,
        SelectedHistoryV1::new(HistoryWindow::new(first, 2026, as_of)?, 2026)?,
        SourceAdmissionV1::milesplit_results(&plan.repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?)
}

fn public_scope(first: u16, as_of: &str, plan: &str) -> TestResult<PublicResourceCaptureScopeV1> {
    Ok(PublicResourceCaptureScopeV1::new(
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        CensusRun::new(SchoolYear::DEFAULT, 1).ok_or("run")?,
        AcquisitionDigest::parse(&"a".repeat(64))?,
        HistoryWindow::new(first, 2026, as_of)?,
        AcquisitionDigest::parse(&plan.repeat(64))?,
        CaptureCacheIntent::ReuseEligible,
    )?)
}

fn obligation(as_of: &str) -> TestResult<PublicResourceObligationV1> {
    Ok(PublicResourceObligationV1::new(
        PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
        ProfileDate::parse(as_of)?,
    ))
}

fn normalized(target: &str) -> TestResult<RequestRepresentationV1> {
    Ok(RequestRepresentationV1::normalized_get(
        target,
        vec![
            ("accept-encoding".into(), "identity".into()),
            ("user-agent".into(), "Census/1.0".into()),
        ],
        "Census/1.0",
    )?)
}

fn public(first: u16, as_of: &str, plan: &str, target: &str) -> TestResult<AcquisitionWorkV1> {
    Ok(AcquisitionWorkV1::for_public_resource(
        identity()?,
        &public_scope(first, as_of, plan)?,
        UsJurisdiction::Michigan,
        obligation(as_of)?,
        normalized(target)?,
    )?)
}

fn original() -> TestResult<AcquisitionWorkV1> {
    Ok(AcquisitionWorkV1::for_owned_meet(
        identity()?,
        &results_scope(2023, "2026-10-14", "b")?,
        UsJurisdiction::Michigan,
        "123",
        RequestRepresentationV1::get(WHOLE_MEET, vec![], "Census/1.0")?,
    )?)
}

#[test]
fn original_whole_meet_work_retains_literal_wire_and_independent_digest() -> TestResult {
    let work = original()?;
    check!(eq; serde_json::to_string(&work)?, ORIGINAL);
    check!(eq; work.id()?.as_str(), "ce42766f7a8db53b4fee01d6e848536b81744f030feabb3c0f29ef0d918aed94");
    check!(work.public_resource_obligation().is_none());
    check!(!work.request().observed_http_required());
    let reopened: AcquisitionWorkV1 = serde_json::from_str(ORIGINAL)?;
    check!(eq; reopened, work);
    check!(eq; reopened.id()?, work.id()?);
    Ok(())
}

#[test]
fn public_native_slot_is_stable_across_exact_request_as_of_plan_and_history_changes() -> TestResult
{
    let baseline = public(2023, "2026-10-14", "b", TARGET)?;
    for changed in [
        public(
            2023,
            "2026-10-14",
            "b",
            "https://mi.milesplit.com/athletes/13749840-new-slug",
        )?,
        public(2023, "2026-10-21", "b", TARGET)?,
        public(2023, "2026-10-14", "c", TARGET)?,
        public(2022, "2026-10-14", "b", TARGET)?,
    ] {
        check!(eq; changed.slot(), baseline.slot());
        check!(eq; changed.slot().id()?, baseline.slot().id()?);
        check!(ne; changed.id()?, baseline.id()?);
    }
    Ok(())
}

#[test]
fn public_work_roundtrip_retains_exact_obligation_and_normalized_request() -> TestResult {
    let work = public(2023, "2026-10-14", "b", TARGET)?;
    let bytes = serde_json::to_vec(&work)?;
    let reopened: AcquisitionWorkV1 = serde_json::from_slice(&bytes)?;
    check!(eq; reopened, work);
    check!(eq; reopened.id()?, work.id()?);
    check!(eq; reopened.request().original_url(), TARGET);
    check!(reopened.request().observed_http_required());
    let retained = reopened
        .public_resource_obligation()
        .ok_or("public obligation")?;
    check!(eq; retained, &obligation("2026-10-14")?);
    check!(eq; reopened.slot().public_resource(), Some(retained.resource()));
    check!(eq; serde_json::to_vec(&reopened)?, bytes);
    Ok(())
}

#[test]
fn public_constructor_refuses_obligation_cutoff_different_from_selected_history() -> TestResult {
    check!(matches!(
        AcquisitionWorkV1::for_public_resource(
            identity()?,
            &public_scope(2023, "2026-10-14", "b")?,
            UsJurisdiction::Michigan,
            obligation("2026-10-21")?,
            normalized(TARGET)?,
        ),
        Err(CaptureDomainError::Invalid {
            field: "work resource obligation"
        })
    ));
    Ok(())
}

#[test]
fn deserialization_refuses_missing_changed_or_cross_resource_public_obligations() -> TestResult {
    let original = serde_json::to_value(public(2023, "2026-10-14", "b", TARGET)?)?;
    for (field, value) in [
        ("as_of", json!("2026-10-21")),
        (
            "resource",
            json!({"kind":"mile_split_athlete_profile","athlete_id":"13749841"}),
        ),
        (
            "resource",
            json!({"kind":"mile_split_team_roster","team_id":"13749840"}),
        ),
    ] {
        let mut changed = original.clone();
        changed["public_resource_obligation"][field] = value;
        check!(serde_json::from_value::<AcquisitionWorkV1>(changed).is_err());
    }
    let mut missing = original.clone();
    missing
        .as_object_mut()
        .ok_or("work object")?
        .remove("public_resource_obligation");
    check!(serde_json::from_value::<AcquisitionWorkV1>(missing).is_err());
    let mut changed = original;
    changed["slot"]["public_resource"]["athlete_id"] = json!("13749841");
    check!(serde_json::from_value::<AcquisitionWorkV1>(changed).is_err());
    Ok(())
}

#[test]
fn source_admission_and_slot_must_belong_to_the_same_resource_family() -> TestResult {
    let old = original()?;
    let new = public(2023, "2026-10-14", "b", TARGET)?;
    for (work, admission) in [
        (&old, new.source_admission()),
        (&new, old.source_admission()),
    ] {
        let mut changed = serde_json::to_value(work)?;
        changed["source_admission"] = serde_json::to_value(admission)?;
        check!(serde_json::from_value::<AcquisitionWorkV1>(changed).is_err());
    }
    let mut changed = serde_json::to_value(old)?;
    changed["public_resource_obligation"] = serde_json::to_value(obligation("2026-10-14")?)?;
    check!(serde_json::from_value::<AcquisitionWorkV1>(changed).is_err());
    Ok(())
}

#[test]
fn old_and_normalized_request_recipes_cannot_cross_resource_families() -> TestResult {
    let old_scope = results_scope(2023, "2026-10-14", "b")?;
    let public_scope = public_scope(2023, "2026-10-14", "b")?;
    check!(AcquisitionWorkV1::for_owned_meet(
        identity()?,
        &old_scope,
        UsJurisdiction::Michigan,
        "123",
        normalized(WHOLE_MEET)?,
    )
    .is_err());
    check!(AcquisitionWorkV1::for_public_resource(
        identity()?,
        &public_scope,
        UsJurisdiction::Michigan,
        obligation("2026-10-14")?,
        RequestRepresentationV1::get(TARGET, vec![], "Census/1.0")?,
    )
    .is_err());
    Ok(())
}

#[test]
fn public_constructor_authenticates_target_native_identity_and_endpoint_kind() -> TestResult {
    for target in [
        "https://mi.milesplit.com/athletes/13749841-brennan-smith",
        "https://mn.milesplit.com/athletes/13749840-brennan-smith",
        "https://mi.milesplit.com/teams/13749840-brennan-smith/roster",
        "https://mi.milesplit.com/athletes/13749840-brennan-smith?retry=2",
    ] {
        check!(public(2023, "2026-10-14", "b", target).is_err());
    }
    Ok(())
}

#[test]
fn source_admission_rejects_all_mixed_old_and_public_role_combinations() -> TestResult {
    let baseline = SourceAdmissionV1::milesplit_results(&"b".repeat(64))?;
    for kind in [
        "legacy_restate_source_selection_v1",
        "profile_bound_public_frontier_v1",
    ] {
        for arm in ["milesplit_results", "milesplit_public_resources"] {
            for role in ["structured_whole_meet", "public_resource"] {
                let mut wire = serde_json::to_value(&baseline)?;
                wire["kind"] = json!(kind);
                wire["selected_arm"] = json!(arm);
                wire["durable_role"] = json!(role);
                let expected = (kind, arm, role)
                    == (
                        "legacy_restate_source_selection_v1",
                        "milesplit_results",
                        "structured_whole_meet",
                    )
                    || (kind, arm, role)
                        == (
                            "profile_bound_public_frontier_v1",
                            "milesplit_public_resources",
                            "public_resource",
                        );
                check!(eq; serde_json::from_value::<SourceAdmissionV1>(wire).is_ok(), expected);
            }
        }
    }
    Ok(())
}

#[test]
fn unmodeled_profile_retry_or_target_fields_cannot_create_new_work_authority() -> TestResult {
    for work in [original()?, public(2023, "2026-10-14", "b", TARGET)?] {
        for field in ["profile", "retry", "target_url", "unknown"] {
            let mut wire = serde_json::to_value(&work)?;
            wire[field] = json!("replacement");
            check!(serde_json::from_value::<AcquisitionWorkV1>(wire).is_err());
        }
    }
    Ok(())
}
