use super::*;
use crate::model::{SchoolId, SourceNamespace, SourceSubject, TeamId};
use serde_json::{json, Value};

type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn limits() -> TestResult<RosterAuthorityLimitsV1> {
    Ok(RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?)
}
fn reference(id: u32) -> Value {
    json!({"digest": format!("{id:064x}"), "bytes": 64})
}
fn contract() -> Value {
    json!({"version": 1, "revision": 1, "digest": "a".repeat(64)})
}
fn fixture() -> TestResult<Value> {
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, "10002".into())?;
    let program_owner = SourceSubject::new(SourceNamespace::MilesplitTeam, "10002".into())?;
    Ok(json!({
        "version": 1,
        "scope": {"provider":"milesplit", "owner":owner, "sport":"cross_country", "category":"girls",
            "policy":contract(), "relationship":"represents_school_program", "temporal":"exact_utc_capture_day"},
        "parser": {"artifact":reference(1), "interpretation":"milesplit-scoped-current-roster/v1",
            "schema":reference(2), "configuration":reference(3), "dependencies":reference(4)},
        "capture": {"occurrence":"b".repeat(64), "body":reference(5), "manifest":reference(6),
            "interpretation":reference(7), "acquired_at":"2026-10-07T23:30:00-05:00", "observed_day":"2026-10-08",
            "requested_url":"https://oh.milesplit.com/teams/10002-mason/roster", "final_url":{"status":"unknown"},
            "retained_source_admission":reference(8), "verification_grant":reference(9)},
        "binding": {"school":SchoolId::mint("sch", &["controlled-school"]),
            "program":TeamId::mint("team", &["controlled-program"]), "school_source":owner,
            "program_source":program_owner, "sport":"cross_country", "category":"girls", "school_year":2026,
            "year_convention":contract(), "projection_merge_contract":contract(), "school_projection":reference(10), "program_projection":reference(11),
            "provider_packet":reference(12), "physical_projection_inventory":reference(13), "store":"c".repeat(64), "snapshot_revision":1},
        "semantic_review_evidence":reference(14)
    }))
}
fn bytes(value: &Value) -> TestResult<Vec<u8>> {
    Ok(serde_json::to_vec(value)?)
}
fn field<'a>(value: &'a mut Value, path: &str) -> TestResult<&'a mut Value> {
    value
        .pointer_mut(path)
        .ok_or_else(|| format!("fixture field absent: {path}").into())
}
fn record() -> TestResult<Value> {
    Ok(
        json!({"version":1, "lifecycle":"issuance_only_v1", "operation":"controlled-issue-1", "proposal":fixture()?,
        "store":"c".repeat(64), "snapshot_revision":1, "parent_inventory":reference(15), "negative_inventory":reference(16),
        "negative_actions":0, "approval_envelope":reference(17), "review_provenance":reference(18), "issuance_receipt":reference(19)}),
    )
}

#[test]
fn controlled_claim_round_trip_preserves_utc_day_and_unknown_final_url() -> TestResult {
    let decoded = decode_roster_issuance_claim(&bytes(&fixture()?)?, limits()?)?;
    check!(eq; decoded.capture().observed_day().as_str(), "2026-10-08");
    check!(eq; decoded.capture().acquired_at(), "2026-10-07T23:30:00-05:00");
    check!(decoded.capture().observed_final_url().is_none());
    let encoded = decoded.encode(limits()?)?;
    check!(eq; decode_roster_issuance_claim(&encoded, limits()?)?, decoded);
    let claimed = decode_roster_record_claim(&bytes(&record()?)?, limits()?)?;
    check!(eq; decode_roster_record_claim(&claimed.encode(limits()?)?, limits()?)?, claimed);
    Ok(())
}

#[test]
fn capture_day_and_semantic_modes_are_not_inferred() -> TestResult {
    for (path, replacement) in [
        ("/capture/observed_day", "2026-10-07"),
        ("/capture/observed_day", "2026-10-09"),
        ("/capture/acquired_at", "not-an-instant"),
        ("/scope/relationship", "enrolled_at_school"),
        ("/scope/temporal", "explicit_ongoing_from"),
        ("/scope/temporal", "closed_interval"),
        ("/scope/sport", "unknown"),
        ("/scope/category", "unknown"),
        ("/scope/provider", "official_school"),
        ("/parser/interpretation", "unreviewed-parser/v2"),
    ] {
        let mut raw = fixture()?;
        *field(&mut raw, path)? = json!(replacement);
        check!(decode_roster_issuance_claim(&bytes(&raw)?, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn mismatched_existing_binding_is_not_repaired() -> TestResult {
    for (path, replacement) in [
        ("/binding/sport", json!("outdoor_track")),
        ("/binding/category", json!("boys")),
        ("/binding/program", json!("")),
        ("/binding/school", json!(" ")),
        ("/binding/snapshot_revision", json!(0)),
        (
            "/binding/program_source",
            serde_json::to_value(SourceSubject::new(
                SourceNamespace::MilesplitTeam,
                "99999".into(),
            )?)?,
        ),
        (
            "/binding/school_source",
            serde_json::to_value(SourceSubject::new(
                SourceNamespace::MilesplitAthlete,
                "10002".into(),
            )?)?,
        ),
    ] {
        let mut raw = fixture()?;
        *field(&mut raw, path)? = replacement;
        check!(decode_roster_issuance_claim(&bytes(&raw)?, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn record_rejects_foreign_context_and_unsupported_negative_actions() -> TestResult {
    for (path, replacement) in [
        ("/version", json!(2)),
        ("/lifecycle", json!("revocation_v1")),
        ("/negative_actions", json!(1)),
        ("/snapshot_revision", json!(2)),
        ("/store", json!("d".repeat(64))),
        ("/operation", json!("contains space")),
    ] {
        let mut raw = record()?;
        *field(&mut raw, path)? = replacement;
        check!(decode_roster_record_claim(&bytes(&raw)?, limits()?).is_err());
    }
    Ok(())
}

#[test]
fn duplicate_unknown_and_missing_fields_refuse() -> TestResult {
    let raw = String::from_utf8(bytes(&fixture()?)?)?;
    let duplicate = raw.replacen("\"version\":1", "\"version\":1,\"version\":1", 1);
    check!(decode_roster_issuance_claim(duplicate.as_bytes(), limits()?).is_err());
    let mut unknown = fixture()?;
    unknown
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("another_capture".into(), json!({}));
    check!(decode_roster_issuance_claim(&bytes(&unknown)?, limits()?).is_err());
    let mut missing = fixture()?;
    missing
        .as_object_mut()
        .ok_or("fixture object")?
        .remove("semantic_review_evidence");
    check!(decode_roster_issuance_claim(&bytes(&missing)?, limits()?).is_err());
    Ok(())
}

#[test]
fn original_component_budget_and_record_boundary_are_enforced() -> TestResult {
    let raw = bytes(&fixture()?)?;
    let exact = RosterAuthorityLimitsV1::new(raw.len(), 14 * 64)?;
    check!(decode_roster_issuance_claim(&raw, exact).is_ok());
    check!(decode_roster_issuance_claim(
        &raw,
        RosterAuthorityLimitsV1::new(raw.len(), 14 * 64 - 1)?
    )
    .is_err());
    let shorter = raw.len().checked_sub(1).ok_or("length")?;
    check!(decode_roster_issuance_claim(
        &raw,
        RosterAuthorityLimitsV1::new(shorter, 8 * 1024 * 1024)?
    )
    .is_err());
    check!(RosterAuthorityLimitsV1::new(0, 1).is_err());
    check!(RosterAuthorityLimitsV1::new(64 * 1024 + 1, 1).is_err());
    check!(RosterAuthorityLimitsV1::new(1, 8 * 1024 * 1024 + 1).is_err());
    Ok(())
}

#[test]
fn object_limits_and_same_digest_different_length_refuse() -> TestResult {
    for (path, replacement) in [
        ("/capture/body/bytes", json!(4 * 1024 * 1024 + 1)),
        ("/capture/manifest/bytes", json!(64 * 1024 + 1)),
        ("/capture/body/bytes", json!(0)),
        ("/capture/body/digest", json!("A".repeat(64))),
        ("/capture/body/digest", json!("a".repeat(63))),
    ] {
        let mut raw = fixture()?;
        *field(&mut raw, path)? = replacement;
        check!(decode_roster_issuance_claim(&bytes(&raw)?, limits()?).is_err());
    }
    let mut conflicting = fixture()?;
    *field(&mut conflicting, "/capture/manifest/digest")? = json!(format!("{:064x}", 5));
    *field(&mut conflicting, "/capture/manifest/bytes")? = json!(65);
    check!(matches!(
        decode_roster_issuance_claim(&bytes(&conflicting)?, limits()?),
        Err(RosterAuthorityError::ConflictingObjectReference)
    ));
    Ok(())
}

#[test]
fn exact_object_limits_and_duplicate_bytes_have_bounded_accounting() -> TestResult {
    let mut raw = fixture()?;
    *field(&mut raw, "/capture/body/bytes")? = json!(4 * 1024 * 1024);
    *field(&mut raw, "/capture/manifest/bytes")? = json!(64 * 1024);
    check!(decode_roster_issuance_claim(&bytes(&raw)?, limits()?).is_ok());
    let mut shared = fixture()?;
    *field(&mut shared, "/capture/manifest")? = reference(5);
    check!(decode_roster_issuance_claim(
        &bytes(&shared)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 13 * 64)?
    )
    .is_ok());
    Ok(())
}

#[test]
fn invalid_or_unbounded_url_and_timestamp_claims_refuse() -> TestResult {
    for (path, replacement) in [
        ("/capture/requested_url", "http://example.test/roster"),
        (
            "/capture/requested_url",
            "https://user:secret@example.test/roster",
        ),
        (
            "/capture/requested_url",
            "https://example.test/roster#fragment",
        ),
        ("/capture/requested_url", "https://example.test/with space"),
        ("/capture/acquired_at", "2026-10-07"),
    ] {
        let mut raw = fixture()?;
        *field(&mut raw, path)? = json!(replacement);
        check!(decode_roster_issuance_claim(&bytes(&raw)?, limits()?).is_err());
    }
    check!(RosterAuthorityTextV1::parse(&"x".repeat(4097)).is_err());
    Ok(())
}

#[test]
fn unpublished_projection_wire_requires_explicit_merge_contract_and_new_names() -> TestResult {
    let mut missing = fixture()?;
    missing.get_mut("binding").and_then(Value::as_object_mut)
        .ok_or("binding object")?.remove("projection_merge_contract");
    check!(decode_roster_issuance_claim(&bytes(&missing)?, limits()?).is_err());
    let mut old = fixture()?;
    let binding = old.get_mut("binding").and_then(Value::as_object_mut).ok_or("binding object")?;
    let projection = binding.remove("school_projection").ok_or("school projection")?;
    binding.insert("school_row".into(), projection);
    check!(decode_roster_issuance_claim(&bytes(&old)?, limits()?).is_err());
    Ok(())
}
