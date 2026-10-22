use super::*;

#[test]
fn sdk_plan_round_trip_emits_the_selected_json_representation() -> TestResult {
    let original = fixture::empty()?;
    let mut bytes = encode(&original)?;
    check!(eq; bytes.as_ref(), serde_json::to_vec(&original)?.as_slice());
    let preserved = bytes.clone();
    let decoded = decode(&mut bytes)?;
    check!(eq; encode(&decoded)?, preserved);
    check!(eq; bytes, preserved);
    check!(eq; decoded.selection_usage(), original.selection_usage());
    Ok(())
}

#[test]
fn sdk_accepts_exact_raw_cap_including_whitespace() -> TestResult {
    let original = fixture::empty()?;
    let mut raw = encode(&original)?.to_vec();
    raw.resize(MAX_PLAN_BYTES, b' ');
    let decoded = decode(&mut Bytes::from(raw))?;
    check!(eq; encode(&decoded)?, encode(&original)?);
    Ok(())
}

#[test]
fn sdk_rejects_cap_plus_one_before_semantic_decoding() -> TestResult {
    let mut raw = vec![b' '; MAX_PLAN_BYTES.checked_add(1).ok_or("fixture length")?];
    let first = raw.first_mut().ok_or("fixture first byte")?;
    *first = b'!';
    limit(decode(&mut Bytes::from(raw)), "encoded plan bytes")
}

#[test]
fn sdk_accepts_a_semantically_cap_sized_plan() -> TestResult {
    let plan = fixture::full()?;
    let mut bytes = encode(&plan)?;
    check!(eq; bytes.len(), MAX_PLAN_BYTES);
    let decoded = decode(&mut bytes)?;
    check!(eq; decoded.pending, plan.pending);
    check!(eq; encode(&decoded)?.len(), MAX_PLAN_BYTES);
    Ok(())
}

#[test]
fn sdk_encoding_refuses_allocation_before_output_growth() -> TestResult {
    let mut plan = fixture::empty()?;
    let remaining = plan.allocation.remaining()?;
    plan.allocation.charge_u64(remaining)?;
    match encode(&plan) {
        Err(ResultsPlanError::Limit(resource)) => {
            check!(eq; resource, "allocation");
            Ok(())
        }
        other => Err(format!("expected allocation admission refusal, got {other:?}").into()),
    }
}

#[test]
fn plan_allocation_accepts_512_mib_and_refuses_one_more_byte() -> TestResult {
    check!(eq; MAX_PLAN_ALLOCATIONS, 536_870_912);
    let mut allocation = PlanAllocation::default();
    allocation.charge_u64(67_108_864)?;
    allocation.charge_u64(
        MAX_PLAN_ALLOCATIONS
            .checked_sub(67_108_864)
            .ok_or("fixture allocation")?,
    )?;
    check!(eq; allocation.remaining()?, 0);
    check!(matches!(
        allocation.charge_u64(1),
        Err(ResultsPlanError::Limit("allocation"))
    ));
    check!(eq; allocation.remaining()?, 0);
    Ok(())
}

#[test]
fn sdk_rejects_trailing_non_whitespace() -> TestResult {
    let mut bytes = encode(&fixture::empty()?)?.to_vec();
    bytes.extend_from_slice(b" false");
    json_error(decode(&mut Bytes::from(bytes)), "trailing characters")
}

#[test]
fn bounded_writer_does_not_extend_past_its_counted_size() -> TestResult {
    let plan = fixture::empty()?;
    let admitted = codec::writer::count(&plan)?
        .checked_sub(1)
        .ok_or("fixture count")?;
    match codec::writer::encode(&plan, admitted) {
        Err(ResultsPlanError::Json(error)) => {
            check!(error
                .to_string()
                .contains("encoding exceeded admitted count"));
            Ok(())
        }
        other => Err(format!("expected counted writer refusal, got {other:?}").into()),
    }
}

#[test]
fn selected_sort_key_matches_existing_domain_canonical_bytes() -> TestResult {
    let meet = census_domain::model::SourceMeetRef {
        id: "id\n\t\u{0001}".to_owned(),
        source: "fixture".to_owned(),
        source_meet_id: "1".to_owned(),
        jurisdiction: census_domain::UsJurisdiction::Michigan,
        season: "xc".to_owned(),
        year: 2026,
        name: "é \"quoted\" \\ path".to_owned(),
        date: Some("2026-10-01".to_owned()),
        venue: "venue".to_owned(),
        results_url: "https://example.org/results".to_owned(),
        observed_on: "2026-10-14".to_owned(),
    };
    let length = codec::writer::count(&meet)?;
    check!(eq; codec::writer::encode(&meet, length)?, census_domain::model::serialized_bytes(&meet)?);
    Ok(())
}

#[test]
fn selected_meet_order_uses_id_before_date_and_preserves_duplicates() -> TestResult {
    let make = |id: &str, date: &str| census_domain::model::SourceMeetRef {
        id: id.to_owned(),
        source: "fixture".to_owned(),
        source_meet_id: id.to_owned(),
        jurisdiction: census_domain::UsJurisdiction::Michigan,
        season: "xc".to_owned(),
        year: 2026,
        name: id.to_owned(),
        date: Some(date.to_owned()),
        venue: String::new(),
        results_url: "https://example.org/results".to_owned(),
        observed_on: "2026-10-14".to_owned(),
    };
    let early_id = make("a", "2026-10-02");
    let early_date = make("z", "2026-10-01");
    let builder = PlanBuilder::new(
        fixture::binding()?,
        Vec::new(),
        vec![early_date.clone(), early_id.clone(), early_id.clone()],
        Vec::new(),
    )?;
    check!(eq; builder.meets(), &[early_id.clone(), early_id, early_date]);
    Ok(())
}

#[test]
fn recovery_diagnostic_count_refuses_before_output_allocation() -> TestResult {
    let exact = "x".repeat(8192);
    check!(eq; diagnostic::admitted(&exact)?.len(), 8192);
    let too_large = "x".repeat(8193);
    check!(matches!(
        diagnostic::admitted(&too_large),
        Err(ResultsPlanError::Limit("diagnostic bytes"))
    ));
    Ok(())
}
