use super::*;

fn withholding_wire() -> TestResult<Value> {
    let mut value = wire();
    value
        .as_object_mut()
        .ok_or("object")?
        .remove("evidence_basis");
    replace(&mut value, "version", json!(2))?;
    replace(
        &mut value,
        "policy",
        json!(LOCAL_MEMBERSHIP_WITHHOLDING_GRAPH_POLICY),
    )?;
    replace(
        &mut value,
        "dispositions",
        json!([{"claim":claim(),"treatment":"RejectIdentityCorroboration","rationale":"retained membership correction"}]),
    )?;
    replace(
        &mut value,
        "request",
        json!({"case_id":"case:one","canonical":FIRST,
        "attachment":{"sequence":1,"digest":"a".repeat(64),"membership_id":"membership:one"},
        "expected_withheld_memberships":[],"affected_record_keys":["performance:one"],
        "rationale":"retained membership correction"}),
    )?;
    Ok(value)
}

#[test]
fn graph_withholding_roundtrip_and_historical_semantic_parity() -> TestResult {
    let value = withholding_wire()?;
    let record: LocalMembershipWithholdingRecordV2 = serde_json::from_value(value.clone())?;
    let claims = vec![claim()];
    record.validate(&claims)?;
    check!(eq; serde_json::to_value(&record)?, value);
    let HistoryRequest::Revoke(request) = record.history_request(&claims)? else {
        return Err("revocation request".into());
    };
    check!(request
        .evidence
        .application_id
        .starts_with("local_membership_withholding_v2_"));
    let mut legacy = value.clone();
    legacy.as_object_mut().ok_or("object")?.remove("graph");
    replace(&mut legacy, "version", json!(1))?;
    replace(
        &mut legacy,
        "policy",
        json!(LOCAL_MEMBERSHIP_WITHHOLDING_POLICY),
    )?;
    replace(&mut legacy, "authority_context", json!("a".repeat(64)))?;
    replace(&mut legacy, "affected_context", json!("b".repeat(64)))?;
    let old: LocalMembershipWithholdingRecordV1 = serde_json::from_value(legacy.clone())?;
    old.validate(&claims)?;
    check!(serde_json::from_value::<LocalMembershipWithholdingRecordV2>(legacy).is_err());
    check!(serde_json::from_value::<LocalMembershipWithholdingRecordV1>(value).is_err());
    let mut rejected = record;
    rejected.request.attachment.membership_id = "foreign".into();
    check!(rejected.validate(&claims).is_err());
    let mut old_rejected = old;
    old_rejected.request = rejected.request;
    check!(old_rejected.validate(&claims).is_err());
    Ok(())
}

#[test]
fn graph_withholding_refuses_legacy_policy_and_mixed_wire() -> TestResult {
    let value = withholding_wire()?;
    let mut record: LocalMembershipWithholdingRecordV2 = serde_json::from_value(value.clone())?;
    record.policy = LOCAL_MEMBERSHIP_WITHHOLDING_POLICY.into();
    check!(record.validate(&[claim()]).is_err());
    for (key, replacement) in [
        ("affected_context", json!("old")),
        ("members", json!(vec![candidate("a"); 129])),
    ] {
        let mut changed = value.clone();
        replace(&mut changed, key, replacement)?;
        check!(serde_json::from_value::<LocalMembershipWithholdingRecordV2>(changed).is_err());
    }
    Ok(())
}
