use crate::public_identity::{codec, witness, PublicIdentityError};
use crate::public_identity_tests::support::{fixture, subject, TestResult};
use crate::{Receipt, StoreError, Table};
use census_domain::model::{serialized_bytes, serialized_digest};
use census_domain::public_identity_qualification::PublicIdentityQualificationVersion;

const HISTORICAL_WITNESS_V1: &[u8] = br#"{"version":1,"operation":"historical-codec-fixture","candidate":"ath_0123456789abcdef","request_digest":"1111111111111111111111111111111111111111111111111111111111111111","evidence_generation":7,"derived_generation":2,"snapshot_sequence":19,"record_digest":"2222222222222222222222222222222222222222222222222222222222222222","decision_id":"source_bound:ath_0123456789abcdef:p1:3333333333333333333333333333333333333333333333333333333333333333","decision_digest":"4444444444444444444444444444444444444444444444444444444444444444","public_id":"ath_public_v1_5555555555555555555555555555555555555555555555555555555555555555"}"#;
const HISTORICAL_RECEIPT_DIGEST: &str =
    "5076194cf4b46943c3aaaa55ec75a8f753d7f345048c4bacb3d609c12ff0371e";
const HISTORICAL_RECEIPT_V1: &[u8] = br#"{"operation":"public_identity_v1:historical-codec-fixture","digest":"5076194cf4b46943c3aaaa55ec75a8f753d7f345048c4bacb3d609c12ff0371e","at":"2026-10-14T12:00:00Z","appended":1,"credited":false}"#;

#[test]
fn historical_v1_witness_round_trip_keeps_exact_bytes_and_omits_new_context_field() -> TestResult {
    let retained: witness::Witness =
        codec::decode("historical-codec-fixture", HISTORICAL_WITNESS_V1)?;
    check!(eq; retained.version(), 1);
    check!(eq; retained.authority_context, None);
    check!(eq; retained.operation, "historical-codec-fixture");
    check!(eq; retained.evidence_generation, 7);
    check!(eq; retained.derived_generation, 2);
    check!(eq; retained.snapshot_sequence, 19);
    retained.require_qualification_version(PublicIdentityQualificationVersion::SourceBoundV1)?;
    check!(eq; codec::encode(&retained, 65_536)?, HISTORICAL_WITNESS_V1);
    check!(eq; serialized_bytes(&retained)?, HISTORICAL_WITNESS_V1);
    check!(eq; serde_json::to_value(&retained)?.get("authority_context"), None);
    Ok(())
}

#[test]
fn historical_v1_receipt_keeps_its_pinned_witness_hash_after_optional_field_addition() -> TestResult
{
    let retained: witness::Witness =
        codec::decode("historical-codec-fixture", HISTORICAL_WITNESS_V1)?;
    let receipt: Receipt = codec::decode("historical-receipt-fixture", HISTORICAL_RECEIPT_V1)?;
    check!(eq; serialized_digest(&retained)?, HISTORICAL_RECEIPT_DIGEST);
    check!(eq; receipt.digest, HISTORICAL_RECEIPT_DIGEST);
    check!(eq; receipt.digest, serialized_digest(&retained)?);
    check!(eq; receipt.operation, witness::receipt_key(&retained.operation));
    check!(eq; receipt.at, "2026-10-14T12:00:00Z");
    check!(eq; receipt.appended, 1);
    check!(eq; receipt.credited, false);
    check!(eq; codec::encode(&receipt, 65_536)?, HISTORICAL_RECEIPT_V1);
    Ok(())
}

#[test]
fn historical_v1_witness_is_not_retained_context_qualification() -> TestResult {
    let retained: witness::Witness =
        codec::decode("historical-codec-fixture", HISTORICAL_WITNESS_V1)?;
    match retained.require_qualification_version(
        PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
    ) {
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(detail))) => {
            check!(eq; detail, "witness qualification version mismatch");
        }
        other => {
            return Err(format!("expected historical qualification refusal, got {other:?}").into())
        }
    }
    check!(eq; codec::encode(&retained, 65_536)?, HISTORICAL_WITNESS_V1);
    check!(eq; serialized_digest(&retained)?, HISTORICAL_RECEIPT_DIGEST);
    Ok(())
}

#[test]
fn context_presence_must_agree_with_the_witness_qualification_version() -> TestResult {
    for (version, context, qualification) in [
        (
            1,
            Some("a".repeat(64)),
            PublicIdentityQualificationVersion::SourceBoundV1,
        ),
        (
            2,
            None,
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        ),
        (
            2,
            Some("malformed-context".into()),
            PublicIdentityQualificationVersion::RetainedAuthorityContextV2,
        ),
    ] {
        let mut retained: witness::Witness =
            codec::decode("historical-codec-fixture", HISTORICAL_WITNESS_V1)?;
        retained.version = version;
        retained.authority_context = context;
        match retained.require_qualification_version(qualification) {
            Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(detail))) => {
                check!(eq; detail, "authority context version binding");
            }
            other => return Err(format!("expected witness context refusal, got {other:?}").into()),
        }
    }
    Ok(())
}

#[test]
fn newly_committed_graph_receipt_hash_binds_the_complete_retained_graph() -> TestResult {
    let (_directory, store, writer) = fixture()?;
    let athlete = subject("witness-context-hash", "9299")?;
    store.append(Table::Athletes, &athlete)?;
    let committed = writer.establish_source_bound(
        &store,
        "witness-context-hash",
        &athlete.id.cast(),
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    check!(eq; committed.head().sequence, 1);
    let key = witness::key(1);
    let raw = store.journal.get(&key)?.ok_or("current graph witness")?;
    let mut retained = crate::public_identity::graph_witness::HeldGraphWitness::decode(&key, &raw)?;
    check!(eq; retained.version, 8);
    check!(matches!(
        &retained.payload,
        crate::public_identity::graph_witness::Payload::Source { .. }
    ));
    let original_graph = retained.graph.clone();
    let receipt_key = witness::receipt_key("witness-context-hash");
    let raw_receipt = store
        .receipts
        .get(&receipt_key)?
        .ok_or("current graph receipt")?;
    let receipt: Receipt = codec::decode(&receipt_key, &raw_receipt)?;
    check!(eq; receipt.digest, serialized_digest(&retained)?);
    let mut omitted = serde_json::to_value(&retained)?;
    omitted
        .as_object_mut()
        .ok_or("witness object")?
        .remove("graph")
        .ok_or("graph field")?;
    check!(matches!(
        crate::public_identity::graph_witness::HeldGraphWitness::decode(
            "missing graph",
            &serde_json::to_vec(&omitted)?
        ),
        Err(StoreError::Decode { .. })
    ));
    let later = subject("witness-alternative-graph", "9300")?;
    store.append(Table::Athletes, &later)?;
    let (alternative, _) = crate::public_identity::graph_operation::inspect(
        &writer,
        &store,
        "witness-alternative-graph",
    )?;
    check!(ne; alternative, original_graph);
    retained.graph = alternative;
    check!(ne; serialized_digest(&retained)?, receipt.digest);
    check!(eq; store.journal.get(&key)?.ok_or("retained graph witness")?, raw);
    check!(eq; store.receipts.get(&receipt_key)?.ok_or("retained graph receipt")?, raw_receipt);
    Ok(())
}
