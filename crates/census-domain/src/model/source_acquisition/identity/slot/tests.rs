use super::*;
use serde_json::json;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const ORIGINAL: &str = r#"{"version":1,"store_identity":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA","owner_key":"jurisdiction:MI:2026-27:1","stage":"results","historical_year":2026,"source":"milesplit","source_site":"MI","source_meet_id":"123","resource_role":"structured_whole_meet"}"#;
const ORIGINAL_ID: &str = "1de63a0311cc7e8b968ac5f235c5e4e57ce8f7623432442c6ed6234cc2e63472";

fn original() -> TestResult<AcquisitionSlotV1> {
    Ok(AcquisitionSlotV1::new(
        StoreIdentity::parse(&"A".repeat(64))?,
        "jurisdiction:MI:2026-27:1",
        2026,
        UsJurisdiction::Michigan,
        "123",
    )?)
}

fn public() -> TestResult<AcquisitionSlotV1> {
    Ok(AcquisitionSlotV1::for_public_resource(
        StoreIdentity::parse(&"A".repeat(64))?,
        "jurisdiction:MI:2026-27:1",
        UsJurisdiction::Michigan,
        PublicSourceResourceV1::milesplit_athlete_profile("13749840")?,
    )?)
}

#[test]
fn original_whole_meet_slot_retains_literal_wire_and_independent_digest() -> TestResult {
    let slot = original()?;
    check!(eq; serde_json::to_string(&slot)?, ORIGINAL);
    check!(eq; slot.id()?.as_str(), ORIGINAL_ID);
    check!(eq; slot.historical_year(), Some(2026));
    check!(eq; slot.source_meet_id(), Some("123"));
    check!(slot.public_resource().is_none());
    let reopened: AcquisitionSlotV1 = serde_json::from_str(ORIGINAL)?;
    check!(eq; reopened, slot);
    check!(eq; reopened.id()?.as_str(), ORIGINAL_ID);
    Ok(())
}

#[test]
fn public_slot_contains_native_resource_without_year_request_or_selection_binding() -> TestResult {
    let slot = public()?;
    let wire = serde_json::to_value(&slot)?;
    check!(eq; wire.get("stage"), Some(&json!("source_discovery")));
    check!(eq; wire.get("source"), Some(&json!("public_web")));
    check!(eq; wire.get("resource_role"), Some(&json!("public_resource")));
    check!(eq; wire.get("public_resource"), Some(&json!({
        "kind": "mile_split_athlete_profile", "athlete_id": "13749840"
    })));
    for absent in [
        "historical_year",
        "source_meet_id",
        "as_of",
        "profile",
        "plan",
        "retry",
        "target_url",
        "request",
        "public_resource_obligation",
    ] {
        check!(wire.get(absent).is_none(), "unexpected slot field {absent}");
    }
    check!(slot.historical_year().is_none());
    check!(slot.source_meet_id().is_none());
    check!(eq; slot.public_resource().and_then(PublicSourceResourceV1::athlete_id), Some("13749840"));
    let reopened: AcquisitionSlotV1 = serde_json::from_slice(&serde_json::to_vec(&slot)?)?;
    check!(eq; reopened, slot);
    check!(eq; reopened.id()?, slot.id()?);
    check!(ne; slot.id()?, original()?.id()?);
    Ok(())
}

#[test]
fn every_mixed_stage_source_and_resource_role_is_refused_for_both_slot_shapes() -> TestResult {
    for (slot, expected) in [
        (
            original()?,
            ("results", "milesplit", "structured_whole_meet"),
        ),
        (
            public()?,
            ("source_discovery", "public_web", "public_resource"),
        ),
    ] {
        for stage in ["results", "source_discovery"] {
            for source in ["milesplit", "public_web"] {
                for role in ["structured_whole_meet", "public_resource"] {
                    let mut wire = serde_json::to_value(&slot)?;
                    wire["stage"] = json!(stage);
                    wire["source"] = json!(source);
                    wire["resource_role"] = json!(role);
                    let accepted = serde_json::from_value::<AcquisitionSlotV1>(wire);
                    check!(eq; accepted.is_ok(), (stage, source, role) == expected);
                }
            }
        }
    }
    Ok(())
}

#[test]
fn old_and_public_slot_payloads_cannot_be_combined_or_omit_their_required_identity() -> TestResult {
    let old = serde_json::to_value(original()?)?;
    let new = serde_json::to_value(public()?)?;
    for field in ["historical_year", "source_meet_id"] {
        let mut missing = old.clone();
        missing
            .as_object_mut()
            .ok_or("old slot object")?
            .remove(field);
        check!(serde_json::from_value::<AcquisitionSlotV1>(missing).is_err());
        let mut substituted = new.clone();
        substituted[field] = old.get(field).ok_or("old identity field")?.clone();
        check!(serde_json::from_value::<AcquisitionSlotV1>(substituted).is_err());
    }
    let mut added = old;
    added["public_resource"] = new.get("public_resource").ok_or("public resource")?.clone();
    check!(serde_json::from_value::<AcquisitionSlotV1>(added).is_err());
    let mut missing = new;
    missing
        .as_object_mut()
        .ok_or("public slot object")?
        .remove("public_resource");
    check!(serde_json::from_value::<AcquisitionSlotV1>(missing).is_err());
    Ok(())
}

#[test]
fn slot_decoding_never_accepts_caller_supplied_budget_or_retry_identity() -> TestResult {
    for slot in [original()?, public()?] {
        for field in ["as_of", "profile", "plan", "retry", "target_url", "unknown"] {
            let mut wire = serde_json::to_value(&slot)?;
            wire[field] = json!("replacement");
            check!(serde_json::from_value::<AcquisitionSlotV1>(wire).is_err());
        }
        let wire = serde_json::to_string(&slot)?;
        let duplicate = wire.replacen('{', "{\"owner_key\":\"replacement\",", 1);
        check!(serde_json::from_str::<AcquisitionSlotV1>(&duplicate).is_err());
    }
    Ok(())
}

#[test]
fn public_slot_distinguishes_store_owner_jurisdiction_native_identity_and_resource_role(
) -> TestResult {
    let baseline = public()?.id()?;
    let athlete = PublicSourceResourceV1::milesplit_athlete_profile("13749840")?;
    for (store, owner, site, resource) in [
        (
            "B",
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            athlete.clone(),
        ),
        (
            "A",
            "jurisdiction:MI:2026-27:2",
            UsJurisdiction::Michigan,
            athlete.clone(),
        ),
        (
            "A",
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Minnesota,
            athlete,
        ),
        (
            "A",
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            PublicSourceResourceV1::milesplit_athlete_profile("13749841")?,
        ),
        (
            "A",
            "jurisdiction:MI:2026-27:1",
            UsJurisdiction::Michigan,
            PublicSourceResourceV1::milesplit_team_roster("13749840")?,
        ),
    ] {
        let changed = AcquisitionSlotV1::for_public_resource(
            StoreIdentity::parse(&store.repeat(64))?,
            owner,
            site,
            resource,
        )?;
        check!(ne; changed.id()?, baseline);
    }
    Ok(())
}
