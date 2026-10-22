use super::*;

#[test]
fn sdk_rejects_duplicate_plan_fields() -> TestResult {
    let encoded = encode(&fixture::empty()?)?;
    let body = std::str::from_utf8(&encoded)?
        .strip_prefix('{')
        .ok_or("fixture object")?;
    let mut bytes = Bytes::from(format!("{{\"version\":2,{body}"));
    json_error(decode(&mut bytes), "duplicate field `version`")
}

#[test]
fn sdk_rejects_unknown_plan_fields() -> TestResult {
    let mut bytes = fixture::mutate("remaining_credit", serde_json::json!(999))?;
    json_error(decode(&mut bytes), "unknown field `remaining_credit`")
}

#[test]
fn sdk_rejects_unknown_usage_fields() -> TestResult {
    let usage = serde_json::json!({
        "acquisition_roots":0,"acquisition_encoded_bytes":0,"receipt_probes":0,
        "present_receipts":0,"absent_target_probes":0,"distinct_targets":0,"credit":1,
    });
    let mut bytes = fixture::mutate("selection_usage", usage)?;
    json_error(decode(&mut bytes), "unknown field `credit`")
}

#[test]
fn sdk_rejects_non_v2_plan_and_unknown_policy() -> TestResult {
    let mut version = fixture::mutate("version", serde_json::json!(1))?;
    json_error(decode(&mut version), "ResultsPlanInvalid: version")?;
    let mut policy = fixture::mutate("policy", serde_json::json!("resume_latest"))?;
    json_error(decode(&mut policy), "unknown variant `resume_latest`")
}

#[test]
fn sdk_rejects_oversized_text_before_owned_string_growth() -> TestResult {
    let long = "x".repeat(MAX_TEXT_BYTES.checked_add(1).ok_or("fixture text length")?);
    let mut bytes = fixture::mutate("pending", serde_json::json!([long]))?;
    json_error(decode(&mut bytes), "text exceeds 4096 bytes")
}

#[test]
fn sdk_rejects_many_small_values_before_decoded_container_growth() -> TestResult {
    let entries = vec![""; 131_072];
    let mut bytes = fixture::mutate("pending", serde_json::json!(entries))?;
    limit(decode(&mut bytes), "decode allocation")
}

#[test]
fn sdk_rejects_usage_that_claims_unlisted_targets() -> TestResult {
    let usage = serde_json::json!({
        "acquisition_roots":0,"acquisition_encoded_bytes":0,"receipt_probes":0,
        "present_receipts":0,"absent_target_probes":1,"distinct_targets":1,
    });
    let mut bytes = fixture::mutate("selection_usage", usage)?;
    invalid(decode(&mut bytes), "selection usage manifest")
}

#[test]
fn sdk_rejects_incoherent_usage_counts() -> TestResult {
    let usage = serde_json::json!({
        "acquisition_roots":0,"acquisition_encoded_bytes":0,"receipt_probes":0,
        "present_receipts":1,"absent_target_probes":0,"distinct_targets":0,
    });
    let mut bytes = fixture::mutate("selection_usage", usage)?;
    invalid(decode(&mut bytes), "selection usage ranges")
}

#[test]
fn sdk_rejects_unknown_nested_history_fields() -> TestResult {
    let mut value = serde_json::to_value(fixture::empty()?.binding())?;
    let history = value
        .get_mut("history")
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("fixture history")?;
    history.insert("ignore_deadline".to_owned(), serde_json::json!(true));
    let mut bytes = fixture::mutate("binding", value)?;
    json_error(decode(&mut bytes), "unknown field `ignore_deadline`")
}

#[test]
fn sdk_rejects_thirteen_pins_during_bounded_sequence_decode() -> TestResult {
    use census_domain::model::{
        AcquisitionAttemptId, AcquisitionDigest, AcquisitionSlotId, AcquisitionWorkId,
        AttemptOrdinal,
    };
    let identity = fixture::binding()?.identity().clone();
    let target = census_domain::model::PreparedRecoveryTarget::new(
        identity.clone(),
        AcquisitionSlotId::parse(&"d".repeat(64))?,
        AcquisitionAttemptId::new(
            identity,
            AcquisitionWorkId::parse(&"e".repeat(64))?,
            AttemptOrdinal::new(1)?,
        ),
        AcquisitionDigest::parse(&"f".repeat(64))?,
    )?;
    let values = vec![serde_json::to_value(&target)?; 13];
    let mut bytes = fixture::mutate("pins", serde_json::Value::Array(values))?;
    json_error(decode(&mut bytes), "sequence limit")
}

#[test]
fn sdk_rejects_nested_option_growth_at_frontier_sequence_limit() -> TestResult {
    let discovery = serde_json::json!([{"ready": nested_options(4097)}]);
    let mut bytes = fixture::mutate("discovery", discovery)?;
    json_error(decode(&mut bytes), "sequence limit")
}

#[test]
fn sdk_rejects_aggregate_nested_options_at_decode_allocation_limit() -> TestResult {
    let discovery = serde_json::json!([
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
        {"ready": nested_options(4096)},
    ]);
    let mut bytes = fixture::mutate("discovery", discovery)?;
    check!(bytes.len() < MAX_PLAN_BYTES);
    limit(decode(&mut bytes), "decode allocation")
}

#[test]
fn nested_option_wire_preserves_frontier_limit_and_refuses_one_more() -> TestResult {
    let encoded = serde_json::to_vec(&nested_options(4096))?;
    let ready: ReadyDiscovery = serde_json::from_slice(&encoded)?;
    check!(eq; ready.options.urls.len(), 4096);
    check!(ready.options.urls.iter().all(|request| {
        request.url == "https://mi.milesplit.com/meets/1/results"
            && request.jurisdiction == census_domain::UsJurisdiction::Michigan
    }));
    let encoded = serde_json::to_vec(&nested_options(4097))?;
    match serde_json::from_slice::<ReadyDiscovery>(&encoded) {
        Err(error) => {
            check!(error.to_string().contains("sequence limit"));
            Ok(())
        }
        Ok(_) => Err("nested option wire accepted more than 4096 requests".into()),
    }
}

fn nested_options(count: usize) -> serde_json::Value {
    let options = vec![
        serde_json::json!({"url":"https://mi.milesplit.com/meets/1/results", "jurisdiction":"MI"});
        count
    ];
    serde_json::json!({"options":{"urls":options},"pages_read":1,"quarantines":[]})
}

#[test]
fn sdk_rejects_duplicate_nested_usage_field() -> TestResult {
    let raw = String::from_utf8(encode(&fixture::empty()?)?.to_vec())?;
    let duplicated = raw.replacen(
        "\"distinct_targets\":0",
        "\"distinct_targets\":0,\"distinct_targets\":0",
        1,
    );
    json_error(
        decode(&mut Bytes::from(duplicated)),
        "duplicate field `distinct_targets`",
    )
}
