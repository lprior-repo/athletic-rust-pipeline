use super::*;

fn refused(value: ResultSchoolReviewCommandV1) -> TestResult {
    let mut grant = AttachmentReplayGrantV1::new();
    check!(CheckedResultSchoolCommandV1::check(value, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn malformed_school_scope_and_source_namespace_are_not_admitted() -> TestResult {
    for case in 0..6 {
        let mut value = command("controlled-invalid-scope")?;
        match case {
            0 => value.source.target.school = SchoolId::mint("ath", &["wrong-prefix"]),
            1 => value.source.target.sport = Sport::Unknown,
            2 => value.source.target.category = Gender::Unknown,
            3 => value.source.target.category = Gender::Mixed,
            4 => {
                value.source.target.source_school =
                    SourceSubject::new(SourceNamespace::MilesplitAthlete, "42".into())?
            }
            _ => {
                value.source.person.canonical_candidate =
                    AthleteCandidateId::mint("sch", &["wrong-person-prefix"])
            }
        }
        refused(value)?;
    }
    Ok(())
}

#[test]
fn malformed_calendar_dates_and_unknown_predecessor_capabilities_are_not_decoded() -> TestResult {
    for date in ["2026-02-30", "2026-13-01", "2026-5-15"] {
        let mut wire = serde_json::to_value(command("controlled-date-shape")?)?;
        *wire.pointer_mut("/source/target/day").ok_or("day field")? = serde_json::json!(date);
        check!(serde_json::from_value::<ResultSchoolReviewCommandV1>(wire).is_err());
    }
    for wire in [
        r#"{"IdentityOnly":{"qualification":"bad"}}"#,
        r#"{"Unknown":{"qualification":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"}}"#,
    ] {
        check!(serde_json::from_str::<ResultSchoolPriorCapabilityV1>(wire).is_err());
    }
    Ok(())
}

#[test]
fn exact_statement_text_requires_utf8_byte_length_and_raw_sha256() -> TestResult {
    let mut valid = command("controlled-utf8-statement")?;
    valid.source.statement.statement_text = text("École")?;
    valid.source.statement.statement_length = 6;
    valid.source.statement.statement_digest = raw_digest("École")?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(CheckedResultSchoolCommandV1::check(valid.clone(), &mut grant).is_ok());
    let mut character_count = valid.clone();
    character_count.source.statement.statement_length = 5;
    refused(character_count)?;
    valid.source.statement.statement_digest = digest("École")?;
    refused(valid)
}

#[test]
fn zero_oversized_and_outside_body_objects_and_spans_are_refused() -> TestResult {
    for case in 0..5 {
        let mut value = command("controlled-invalid-boundary")?;
        match case {
            0 => value.source.statement.body.bytes = 0,
            1 => value.source.statement.body.bytes = 4 * 1024 * 1024 + 1,
            2 => value.source.statement.row_length = 101,
            3 => value.source.statement.row_offset = u64::MAX,
            _ => value.source.statement.row_offset = 21,
        }
        refused(value)?;
    }
    Ok(())
}

#[test]
fn changed_record_digest_unknown_fields_and_duplicate_fields_fail_closed() -> TestResult {
    let original = record(command("controlled-wire-refusal")?)?;
    let mut altered = original.clone();
    altered.record_digest = digest("foreign-record")?;
    let mut extra = serde_json::to_value(&original)?;
    extra
        .as_object_mut()
        .ok_or("record object")?
        .insert("unknown".into(), serde_json::json!(true));
    let canonical = String::from_utf8(crate::model::serialized_bytes(&original)?)?;
    let mut duplicate = canonical.trim_end_matches('}').to_owned();
    duplicate.push_str(&format!(
        ",\"record_digest\":\"{}\"}}",
        original.record_digest.as_str()
    ));
    for bytes in [
        crate::model::serialized_bytes(&altered)?,
        crate::model::serialized_bytes(&extra)?,
        duplicate.into_bytes(),
    ] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(decode_result_school_record_v1(&bytes, &mut grant).is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}
