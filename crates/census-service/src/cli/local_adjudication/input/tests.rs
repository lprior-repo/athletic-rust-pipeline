use super::*;
use serde_json::{json, Value};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn request() -> Value {
    json!({
        "version": 1,
        "case_id": "synthetic-manual-case",
        "canonical": format!("ath_public_v1_{}", "1".repeat(64)),
        "supersedes": [{"sequence": 3, "digest": "2".repeat(64)}],
        "partitions": [{"binding_ids": ["synthetic-binding"], "representative": "ath_0123456789abcdef"}],
        "unresolved_bindings": ["synthetic-uncertain-binding"],
        "affected_record_keys": ["synthetic-result"],
        "dispositions": [{
            "claim": {
                "candidate": "ath_0123456789abcdef", "section": "Athlete", "ordinal": 0,
                "row_digest": "3".repeat(64), "claim_path": "/source_links/0",
                "claim_digest": "4".repeat(64), "retained_value": {"synthetic": true}
            },
            "treatment": "RejectIdentityCorroboration",
            "rationale": "Synthetic published profile link joined distinct athletes"
        }],
        "rationale": "Synthetic reviewed split preserves unresolved observations"
    })
}

#[test]
fn local_request_parser_preserves_every_review_field_without_granting_authority() -> TestResult {
    let input = request();
    let parsed = parse_request(&serde_json::to_vec(&input)?)?;
    let mut expected = input;
    expected
        .as_object_mut()
        .ok_or("fixture object")?
        .remove("version");
    check!(eq; serde_json::to_value(&parsed)?, expected);
    check!(eq; parsed.dispositions.first().ok_or("disposition")?.treatment,
        ClaimTreatmentV1::RejectIdentityCorroboration);
    Ok(())
}

#[test]
fn local_request_parser_refuses_supplied_actor_approval_and_unknown_nested_fields() -> TestResult {
    for pointer in [
        "",
        "/supersedes/0",
        "/partitions/0",
        "/dispositions/0",
        "/dispositions/0/claim",
    ] {
        let mut input = request();
        let object = input
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .ok_or("fixture object")?;
        object.insert("actor".into(), json!("local_operator"));
        check!(matches!(
            parse_request(&serde_json::to_vec(&input)?),
            Err(InputError::Json(_))
        ));
    }
    let mut input = request();
    input
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("approval_digest".into(), json!("a".repeat(64)));
    check!(matches!(
        parse_request(&serde_json::to_vec(&input)?),
        Err(InputError::Json(_))
    ));
    Ok(())
}

#[test]
fn local_request_parser_requires_operator_and_each_claim_rationale() -> TestResult {
    for pointer in ["", "/dispositions/0"] {
        let mut input = request();
        input
            .pointer_mut(pointer)
            .and_then(Value::as_object_mut)
            .ok_or("fixture object")?
            .remove("rationale");
        check!(matches!(
            parse_request(&serde_json::to_vec(&input)?),
            Err(InputError::Json(_))
        ));
    }
    Ok(())
}

#[test]
fn local_request_parser_refuses_other_versions_invalid_public_ids_and_duplicate_keys() -> TestResult
{
    let mut input = request();
    input
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("version".into(), json!(2));
    check!(matches!(
        parse_request(&serde_json::to_vec(&input)?),
        Err(InputError::Version(2))
    ));
    input
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("version".into(), json!(1));
    input
        .as_object_mut()
        .ok_or("fixture object")?
        .insert("canonical".into(), json!("forged"));
    check!(matches!(
        parse_request(&serde_json::to_vec(&input)?),
        Err(InputError::PublicId(HistoryError::Invalid("public ID")))
    ));
    let serialized = serde_json::to_string(&request())?;
    let duplicate = serialized.replacen('{', "{\"version\":1,", 1);
    check!(matches!(
        parse_request(duplicate.as_bytes()),
        Err(InputError::Json(_))
    ));
    Ok(())
}

#[test]
fn local_request_parser_refuses_unrecognized_claim_and_treatment_kinds() -> TestResult {
    for pointer in ["/dispositions/0/treatment", "/dispositions/0/claim/section"] {
        let mut input = request();
        *input.pointer_mut(pointer).ok_or("fixture field")? = json!("model_approved");
        check!(matches!(
            parse_request(&serde_json::to_vec(&input)?),
            Err(InputError::Json(_))
        ));
    }
    Ok(())
}

#[test]
fn local_request_reader_accepts_regular_json_and_refuses_directories_and_oversized_input(
) -> TestResult {
    let directory = tempfile::tempdir()?;
    let file = directory.path().join("synthetic-request.json");
    let bytes = serde_json::to_vec(&request())?;
    std::fs::write(&file, &bytes)?;
    check!(eq; read_request(&file)?, parse_request(&bytes)?);
    check!(matches!(
        read_request(directory.path()),
        Err(InputError::Read(_))
    ));
    let oversized = vec![b' '; MAX_REQUEST_BYTES.checked_add(1).ok_or("input bound")?];
    check!(matches!(
        parse_request(&oversized),
        Err(InputError::Oversized)
    ));
    std::fs::write(&file, oversized)?;
    check!(matches!(read_request(&file), Err(InputError::Read(_))));
    Ok(())
}

#[test]
fn local_request_parser_accepts_the_exact_byte_limit_and_rejects_the_next_byte() -> TestResult {
    let mut bytes = serde_json::to_vec(&request())?;
    bytes.resize(MAX_REQUEST_BYTES, b' ');
    check!(eq; parse_request(&bytes)?.case_id, "synthetic-manual-case");
    bytes.push(b' ');
    check!(matches!(parse_request(&bytes), Err(InputError::Oversized)));
    Ok(())
}
