use super::*;

mod parity;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn reference() -> String {
    format!(r#"{{"digest":"{}","bytes":1}}"#, "1".repeat(64))
}

fn entry(inputs: &str, introduced: &str) -> String {
    format!(
        r#"{{"schema":"{SCHEMA}","record":{{ "opaque" : [ 1, 2 ] }},"introduced":{introduced},"inputs":{inputs}}}"#
    )
}

fn inputs() -> String {
    format!(r#"{{"capture":{},"token":{}}}"#, reference(), reference())
}

#[test]
fn canonical_entry_keeps_existing_domain_record_raw_value_bytes() -> TestResult {
    let raw = entry(&inputs(), "[]");
    let mut grant = AttachmentReplayGrantV1::new();
    let decoded = parse(raw.as_bytes(), &mut grant)?;
    check!(eq; decoded.record.get(), r#"{ "opaque" : [ 1, 2 ] }"#);
    check!(eq; serde_json::to_vec(&decoded)?, raw.as_bytes());
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(eq; grant.usage().encoded_bytes(), raw.len());
    Ok(())
}

#[test]
fn positional_input_and_nested_reference_metadata_refuse() -> TestResult {
    let positional_inputs = format!("[{},{}]", reference(), reference());
    let positional_reference = format!(
        r#"{{"capture":["{}",1],"token":{}}}"#,
        "1".repeat(64),
        reference()
    );
    let positional_material = format!(
        r#"[{{"storage":"inline","reference":["{}",1],"data":"eA=="}}]"#,
        "1".repeat(64)
    );
    for raw in [
        entry(&positional_inputs, "[]"),
        entry(&positional_reference, "[]"),
        entry(&inputs(), &positional_material),
    ] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            parse(raw.as_bytes(), &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                "noncanonical private attachment wire"
            )))
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(domain::EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn duplicate_input_and_reference_fields_refuse_during_decode() -> TestResult {
    let duplicate_input = format!(
        r#"{{"capture":{},"capture":{},"token":{}}}"#,
        reference(),
        reference(),
        reference()
    );
    let duplicate_reference = format!(
        r#"{{"capture":{{"digest":"{}","bytes":1,"bytes":1}},"token":{}}}"#,
        "1".repeat(64),
        reference()
    );
    for raw in [
        entry(&duplicate_input, "[]"),
        entry(&duplicate_reference, "[]"),
    ] {
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            parse(raw.as_bytes(), &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
                domain::EvidenceSubjectAttachmentError::Decoding(_)
            )))
        ));
        check!(eq; grant.usage().traversal_steps(), 1);
    }
    Ok(())
}

#[test]
fn reordered_outer_metadata_and_raw_wrapper_inputs_are_refused() -> TestResult {
    let raw = format!(
        r#"{{"record":{{}},"schema":"{SCHEMA}","introduced":[],"inputs":{}}}"#,
        inputs()
    );
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        parse(raw.as_bytes(), &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "noncanonical private attachment wire"
        )))
    ));
    let wrapped =
        census_domain::model::serialized_bytes(&serde_json::from_str::<&RawValue>(&inputs())?)?;
    let raw = entry(std::str::from_utf8(&wrapped)?, "[]");
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        parse(raw.as_bytes(), &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Domain(
            domain::EvidenceSubjectAttachmentError::Decoding(_)
        )))
    ));
    Ok(())
}
