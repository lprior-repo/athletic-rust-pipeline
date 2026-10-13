use super::*;

#[test]
fn constructor_protocol_and_calendar_boundaries_refuse_invalid_inputs() -> TestResult {
    let payload = contract(vec![])?;
    let mut future = payload.input().clone();
    future.version = 2;
    check!(matches!(
        RegionMembershipContractV1::new(future),
        Err(RegionMembershipError::UnsupportedVersion { found: 2 })
    ));
    let mut zero = payload.input().clone();
    zero.revision = 0;
    check!(matches!(
        RegionMembershipContractV1::new(zero),
        Err(RegionMembershipError::ZeroRevision)
    ));
    let mut empty = payload.input().clone();
    empty.jurisdictions.clear();
    check!(matches!(
        RegionMembershipContractV1::new(empty),
        Err(RegionMembershipError::InvalidField {
            field: "jurisdictions"
        })
    ));
    for invalid in [
        "",
        ":",
        "up",
        "mhsaa:",
        ":up",
        "mhsaa:up:other",
        "MHSAA:up",
        "mhsaa:up\n",
    ] {
        check!(RegionKeyV1::parse(invalid).is_err());
    }
    for invalid in [
        "2026",
        "2026-2-01",
        "2026-02-30",
        "2027-02-29",
        "1899-12-31",
        "2101-01-01",
    ] {
        check!(ProfileDate::parse(invalid).is_err());
    }
    check!(ProfileDate::parse("2028-02-29").is_ok());
    check!(ProfileDateRangeV1::new(date("2026-10-02")?, date("2026-10-01")?).is_err());
    Ok(())
}

#[test]
fn raw_jurisdiction_and_assertion_caps_apply_before_deduplication() -> TestResult {
    let payload = contract(vec![])?;
    let mut at_cap = payload.input().clone();
    at_cap.jurisdictions = vec![UsJurisdiction::Michigan; MAX_REGION_JURISDICTIONS];
    check!(eq; RegionMembershipContractV1::new(at_cap)?.input().jurisdictions.len(), 1);
    let mut over_cap = payload.input().clone();
    let count = MAX_REGION_JURISDICTIONS
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("fixture overflow"))?;
    over_cap.jurisdictions = vec![UsJurisdiction::Michigan; count];
    check!(matches!(
        RegionMembershipContractV1::new(over_cap),
        Err(RegionMembershipError::TooManyEntries {
            field: "jurisdictions",
            ..
        })
    ));
    let fact = assertion(
        RegionSubjectV1::School(school("a", UsJurisdiction::Michigan)?),
        RegionStatementV1::Member,
        "member",
    )?;
    let mut over_assertions = payload.input().clone();
    let count = MAX_REGION_ASSERTIONS
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("fixture overflow"))?;
    over_assertions.assertions = vec![fact; count];
    check!(matches!(
        RegionMembershipContractV1::new(over_assertions),
        Err(RegionMembershipError::TooManyEntries {
            field: "assertions",
            ..
        })
    ));
    Ok(())
}

#[test]
fn encoded_and_aggregate_payload_caps_refuse_without_silent_truncation() -> TestResult {
    let exact = vec![b' '; MAX_REGION_PAYLOAD_BYTES];
    admit_region_payload_bytes_v1(&exact)?;
    let count = MAX_REGION_PAYLOAD_BYTES
        .checked_add(1)
        .ok_or_else(|| std::io::Error::other("fixture overflow"))?;
    check!(matches!(
        admit_region_payload_bytes_v1(&vec![b' '; count]),
        Err(RegionMembershipError::PayloadTooLarge { .. })
    ));
    let fact = assertion(
        RegionSubjectV1::School(school("a", UsJurisdiction::Michigan)?),
        RegionStatementV1::Member,
        "member",
    )?;
    let mut input = fact.input().clone();
    let mut provenance = input.provenance.input().clone();
    provenance.evidence.note = Some("x".repeat(4096));
    input.provenance = RegionEvidenceV1::new(provenance)?;
    let fat_fact = RegionAssertionV1::new(input);
    let mut payload = contract(vec![])?.input().clone();
    payload.assertions = vec![fat_fact; 300];
    check!(matches!(
        RegionMembershipContractV1::new(payload),
        Err(RegionMembershipError::PayloadTooLarge { .. })
    ));
    Ok(())
}

#[test]
fn provenance_constructor_refuses_invalid_capture_dates_hashes_methods_and_urls() -> TestResult {
    let valid = evidence("member")?;
    for invalid_url in [
        "file:///etc/passwd",
        "https://user:password@example.invalid/",
        "https://example.invalid:0/",
        "https://example.invalid/a b",
        "",
    ] {
        let mut invalid = valid.input().clone();
        invalid.evidence.source.url = Some(invalid_url.to_string());
        check!(RegionEvidenceV1::new(invalid).is_err());
    }
    for invalid_date in [
        "2026",
        "2026-2-01",
        "2026-02-30",
        "1900-01-01T00:00:00",
        "2101-01-01T00:00:00Z",
    ] {
        let mut invalid = valid.input().clone();
        invalid.evidence.observed_on = invalid_date.to_string();
        check!(RegionEvidenceV1::new(invalid).is_err());
    }
    for invalid_digest in ["a".repeat(63), "A".repeat(64), "g".repeat(64)] {
        let mut invalid = valid.input().clone();
        invalid.capture_sha256 = invalid_digest;
        check!(RegionEvidenceV1::new(invalid).is_err());
    }
    let mut fetched = valid.input().clone();
    fetched.evidence.method = crate::model::EvidenceMethod::Fetched;
    check!(RegionEvidenceV1::new(fetched).is_err());
    let mut missing = valid.input().clone();
    missing.evidence.source.url = None;
    check!(RegionEvidenceV1::new(missing).is_err());
    Ok(())
}

#[test]
fn provenance_raw_text_caps_and_unknown_nested_fields_cannot_bypass_checked_serde() -> TestResult {
    let fixture = super::fixtures::CONTRACT;
    for changed in [
        fixture.replacen("\"version\": 1", "\"version\": 2", 1),
        fixture.replacen("\"revision\": 7", "\"revision\": 0", 1),
        fixture.replacen("\"region\":", "\"unrecognized\": true, \"region\":", 1),
        fixture.replacen("\"source\": {", "\"source\": {\"unrecognized\": true,", 1),
        fixture.replacen(
            "\"method\": \"parsed\"",
            "\"unrecognized\": true, \"method\": \"parsed\"",
            1,
        ),
        fixture.replacen("\"context\": {", "\"context\": {\"unrecognized\": true,", 1),
        fixture.replacen(
            "\"jurisdiction\": \"MI\"",
            "\"unrecognized\": true, \"jurisdiction\": \"MI\"",
            1,
        ),
        fixture.replacen("\"from\": \"2026-08-01\"", "\"from\": \"2027-08-01\"", 1),
        fixture.replacen("sch_0000000000000001", "team_0000000000000001", 1),
        fixture.replacen("\"statement\": \"member\"", "\"statement\": \"absent\"", 1),
    ] {
        check!(serde_json::from_str::<RegionMembershipContractV1>(&changed).is_err());
    }
    let valid = evidence("member")?;
    let mut huge = valid.input().clone();
    huge.statement = "x".repeat(513);
    check!(RegionEvidenceV1::new(huge.clone()).is_err());
    check!(serde_json::from_value::<RegionEvidenceV1>(serde_json::to_value(huge)?).is_err());
    let mut huge = valid.input().clone();
    huge.evidence.source.id = "x".repeat(129);
    check!(serde_json::from_value::<RegionEvidenceV1>(serde_json::to_value(huge)?).is_err());
    Ok(())
}

#[test]
fn bounded_sequence_refuses_hostile_extra_element_before_decoding_it() -> TestResult {
    let payload = contract(vec![])?;
    let mut value = serde_json::to_value(payload)?;
    let object = value
        .as_object_mut()
        .ok_or_else(|| std::io::Error::other("missing fixture object"))?;
    let mut jurisdictions =
        vec![serde_json::Value::String("MI".to_string()); MAX_REGION_JURISDICTIONS];
    jurisdictions.push(serde_json::json!({"hostile": [[[[["not-a-jurisdiction"]]]]]}));
    object.insert(
        "jurisdictions".to_string(),
        serde_json::Value::Array(jurisdictions),
    );
    let encoded = serde_json::to_string(&value)?;
    let error = serde_json::from_str::<RegionMembershipContractV1>(&encoded)
        .err()
        .ok_or_else(|| std::io::Error::other("oversized fixture unexpectedly admitted"))?;
    check!(error.to_string().contains("raw-entry ceiling"));
    Ok(())
}

#[test]
fn query_unknown_classification_and_wrong_identifier_kinds_are_refused() -> TestResult {
    let school = school("a", UsJurisdiction::Michigan)?;
    for context in [
        context(Sport::Unknown, Gender::Boys, 2026)?,
        context(Sport::CrossCountry, Gender::Unknown, 2026)?,
    ] {
        check!(RegionMembershipQueryV1::new(
            RegionMembershipTargetV1::School(school.clone()),
            context,
            date("2026-10-01")?
        )
        .is_err());
    }
    let wrong_school: SchoolId = serde_json::from_str("\"team_0000000000000001\"")?;
    check!(RegionSchoolV1::new(wrong_school, UsJurisdiction::Michigan).is_err());
    let empty_school: SchoolId = serde_json::from_str("\"sch_\"")?;
    check!(RegionSchoolV1::new(empty_school, UsJurisdiction::Michigan).is_err());
    let wrong_program: TeamId = serde_json::from_str("\"sch_0000000000000001\"")?;
    check!(RegionProgramV1::new(wrong_program, RegionCompositionV1::unknown()).is_err());
    Ok(())
}
