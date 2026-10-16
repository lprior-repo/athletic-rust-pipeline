use super::*;
use crate::StoreError;

mod parity;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn reference() -> String {
    format!(r#"{{"digest":"{}","bytes":1}}"#, "1".repeat(64))
}

fn head() -> String {
    format!(r#"{{"sequence":1,"digest":"{}"}}"#, "2".repeat(64))
}

fn witness(public_head: &str, group: &str) -> String {
    format!(
        r#"{{"schema":"retained-evidence-subject-attachment-target-witness/v1","store":"{}","qualification":"retained_authority_graph_v3","public_head":{public_head},"evidence_generation":1,"derived_generation":0,"group":{group},"capture_binding":{},"input_revisions":{}}}"#,
        "3".repeat(64),
        reference(),
        reference()
    )
}

fn group(established: &str) -> String {
    format!(
        r#"{{"schema":"retained-evidence-subject-attachment-group/v1","store":"{}","qualification":"retained_authority_graph_v3","public_id":"controlled-public","representative":"controlled-candidate","established":{established},"claims":[{{ "opaque" : [ 1, 2 ] }}]}}"#,
        "3".repeat(64)
    )
}

#[test]
fn canonical_historical_metadata_preserves_opaque_claim_bytes() -> TestResult {
    let witness_raw = witness(&head(), &reference());
    let witness: Witness<'_> = serde_json::from_str(&witness_raw)?;
    let group_raw = group(r#"{"sequence":1,"position":0}"#);
    let group: Group<'_> = serde_json::from_str(&group_raw)?;
    let mut grant = AttachmentReplayGrantV1::new();
    encoding::require_canonical(&witness, witness_raw.as_bytes(), &mut grant)?;
    encoding::require_canonical(&group, group_raw.as_bytes(), &mut grant)?;
    check!(eq; group.claims.first().ok_or("opaque claim absent")?.get(), r#"{ "opaque" : [ 1, 2 ] }"#);
    check!(eq; grant.usage().traversal_steps(), 2);
    check!(eq; grant.usage().encoded_bytes(), 0);
    Ok(())
}

#[test]
fn positional_head_and_witness_reference_refuse_canonical_boundary() -> TestResult {
    let positional_head = format!(r#"[1,"{}"]"#, "2".repeat(64));
    let positional_reference = format!(r#"["{}",1]"#, "1".repeat(64));
    for raw in [
        witness(&positional_head, &reference()),
        witness(&head(), &positional_reference),
    ] {
        let value: Witness<'_> = serde_json::from_str(&raw)?;
        let mut grant = AttachmentReplayGrantV1::new();
        check!(matches!(
            encoding::require_canonical(&value, raw.as_bytes(), &mut grant),
            Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
                "noncanonical private attachment wire"
            )))
        ));
        check!(matches!(
            grant.ensure_available(),
            Err(EvidenceSubjectAttachmentError::Poisoned)
        ));
    }
    Ok(())
}

#[test]
fn positional_establishment_refuses_canonical_boundary() -> TestResult {
    let raw = group("[1,0]");
    let value: Group<'_> = serde_json::from_str(&raw)?;
    let mut grant = AttachmentReplayGrantV1::new();
    check!(matches!(
        encoding::require_canonical(&value, raw.as_bytes(), &mut grant),
        Err(StoreError::EvidenceSubjectAttachment(Error::Corrupt(
            "noncanonical private attachment wire"
        )))
    ));
    check!(matches!(
        grant.ensure_available(),
        Err(EvidenceSubjectAttachmentError::Poisoned)
    ));
    Ok(())
}

#[test]
fn duplicate_head_establishment_and_reference_fields_refuse() -> TestResult {
    let duplicate_head = format!(
        r#"{{"sequence":1,"sequence":1,"digest":"{}"}}"#,
        "2".repeat(64)
    );
    let duplicate_reference = format!(r#"{{"digest":"{}","bytes":1,"bytes":1}}"#, "1".repeat(64));
    check!(serde_json::from_str::<Witness<'_>>(&witness(&duplicate_head, &reference())).is_err());
    check!(serde_json::from_str::<Witness<'_>>(&witness(&head(), &duplicate_reference)).is_err());
    check!(serde_json::from_str::<Group<'_>>(&group(
        r#"{"sequence":1,"position":0,"position":0}"#
    ))
    .is_err());
    Ok(())
}
