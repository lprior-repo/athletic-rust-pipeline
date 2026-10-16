use super::*;

#[test]
fn graph_manual_roundtrip_preserves_shared_semantics_and_distinct_evidence() -> TestResult {
    let value = wire();
    let record: LocalIdentityAdjudicationRecordV3 = serde_json::from_value(value.clone())?;
    let claims = vec![claim()];
    record.validate(&claims)?;
    check!(eq; serde_json::to_value(&record)?, value);
    let HistoryRequest::Split(request) = record.history_request(&claims)? else {
        return Err("split request".into());
    };
    check!(request
        .evidence
        .application_id
        .starts_with("local_adjudication_v3_"));
    let mut legacy = value.clone();
    legacy.as_object_mut().ok_or("object")?.remove("graph");
    replace(&mut legacy, "version", json!(2))?;
    replace(&mut legacy, "policy", json!(LOCAL_ADJUDICATION_POLICY))?;
    replace(&mut legacy, "authority_context", json!("a".repeat(64)))?;
    replace(&mut legacy, "affected_context", json!("b".repeat(64)))?;
    let old: LocalIdentityAdjudicationRecordV2 = serde_json::from_value(legacy.clone())?;
    old.validate(&claims)?;
    check!(serde_json::from_value::<LocalIdentityAdjudicationRecordV3>(legacy).is_err());
    check!(serde_json::from_value::<LocalIdentityAdjudicationRecordV2>(value).is_err());
    let mut rejected = record.clone();
    rejected
        .request
        .dispositions
        .first_mut()
        .ok_or("disposition")?
        .treatment = ClaimTreatmentV1::RetainAsContext;
    check!(rejected.validate(&claims).is_err());
    let mut old_rejected = old;
    old_rejected.request = rejected.request;
    check!(old_rejected.validate(&claims).is_err());
    Ok(())
}

#[test]
fn graph_manual_rejects_wrong_contract_mixed_context_and_oversize_members() -> TestResult {
    let value = wire();
    for (key, replacement) in [
        ("version", json!(2)),
        ("policy", json!(LOCAL_ADJUDICATION_POLICY)),
    ] {
        let mut changed = value.clone();
        replace(&mut changed, key, replacement)?;
        let record: LocalIdentityAdjudicationRecordV3 = serde_json::from_value(changed)?;
        check!(record.validate(&[claim()]).is_err());
    }
    for (key, replacement) in [
        ("authority_context", json!("old")),
        ("members", json!(vec![candidate("a"); 129])),
    ] {
        let mut changed = value.clone();
        replace(&mut changed, key, replacement)?;
        check!(serde_json::from_value::<LocalIdentityAdjudicationRecordV3>(changed).is_err());
    }
    Ok(())
}
