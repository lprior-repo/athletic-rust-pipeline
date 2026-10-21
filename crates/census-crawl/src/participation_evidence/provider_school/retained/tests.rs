use super::*;
use crate::participation_evidence::hash;
use census_domain::evidence_subject_attachment::{AttachmentDigestV1, AttachmentObjectReferenceV1};
use census_store::RetainedCaptureRootV1;

mod fixture;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn raw_packet_and_capture_serialization_preserve_exact_bytes_without_tags() -> TestResult {
    let original = fixture::packet()?;
    let bytes = original.encode()?;
    let retained = RetainedProviderSchoolPacketV1::from_root(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &bytes,
    )?;
    check!(eq; retained.encode()?, bytes);
    check!(eq; serde_json::to_vec(&retained)?, bytes);
    check!(eq; retained.reference()?, original.reference()?);
    check!(eq; serde_json::to_vec(&retained.directory())?, serde_json::to_vec(original.directory())?);
    check!(eq; serde_json::to_vec(&retained.roster())?, serde_json::to_vec(original.roster())?);
    check!(eq; retained.roster().body()?, fixture::BODY);
    check!(eq; retained.roster().requested_url(), fixture::ROSTER_URL);
    check!(eq; retained.roster().final_url(), fixture::ROSTER_URL);
    check!(eq; retained.roster().status(), 200);
    check!(eq; retained.roster().acquired_at()?, original.roster().acquired_at());
    check!(eq; retained.roster().original_body_reference()?, original.roster().original_body_reference()?);
    Ok(())
}

#[test]
fn raw_retained_parser_preserves_existing_noncanonical_literal_acceptance() -> TestResult {
    let original = fixture::packet()?;
    let canonical = original.encode()?;
    let mut spaced = canonical.clone();
    spaced.push(b' ');
    check!(ProviderSchoolPacketV1::decode(&spaced).is_err());
    let retained = RetainedProviderSchoolPacketV1::from_root(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &spaced,
    )?;
    check!(eq; retained.encode()?, canonical);
    check!(eq; retained.reference()?, original.reference()?);
    check!(retained.read_semantic_contract("unknown").is_err());
    Ok(())
}

#[test]
fn root_schema_does_not_relabel_raw_packet_as_observed() -> TestResult {
    let bytes = fixture::packet()?.encode()?;
    for schema in [
        RetainedCaptureRootSchemaV1::ProviderSchoolObservedPacketV1,
        RetainedCaptureRootSchemaV1::ProviderTargetPacketV1,
        RetainedCaptureRootSchemaV1::ParticipationPacketV1,
    ] {
        check!(RetainedProviderSchoolPacketV1::from_root(schema, &bytes).is_err());
    }
    Ok(())
}

#[test]
fn raw_capture_metadata_preserves_old_schema_fields_order_and_transport_exactly() -> TestResult {
    let raw = fixture::packet()?;
    let bytes = raw.encode()?;
    let packet = RetainedProviderSchoolPacketV1::from_root(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &bytes,
    )?;
    let root = root(&bytes, raw.reference()?.as_str())?;
    let roster = packet.roster();
    let metadata = roster.roster_metadata(&root, &bytes, fixture::BODY)?;
    let encoded = serde_json::to_vec(&metadata)?;
    let expected = format!(
        "{{\"schema\":\"roster-retained-capture-metadata-v1\",\"leaf_role\":\"provider_school.roster\",\"root\":{},\"body\":{{\"digest\":\"{}\",\"bytes\":{}}},\"requested_url\":\"{}\",\"observed_final_url\":\"{}\",\"status\":200,\"retained_acquired_at\":\"2026-10-16T00:49:43Z\",\"timestamp_provenance\":\"retained-packet-decoded-string; original-transport-spelling-unknown\",\"normalized_utc_day\":\"2026-10-16\",\"http_provenance\":{}}}",
        serde_json::to_string(&root)?, hash(fixture::BODY.as_bytes()), fixture::BODY.len(),
        fixture::ROSTER_URL, fixture::ROSTER_URL,
        serde_json::to_string(raw.roster().provenance())?,
    );
    check!(eq; encoded, expected.as_bytes());
    check!(!expected.contains("normalized_observed"));
    check!(roster
        .roster_metadata(&root, &bytes, "changed original")
        .is_err());
    Ok(())
}

#[test]
fn raw_metadata_retains_timestamp_spelling_and_refuses_changed_instant() -> TestResult {
    let raw = fixture::packet()?;
    let canonical = raw.encode()?;
    let alternate = String::from_utf8(canonical.clone())?
        .replace("2026-10-16T00:49:43Z", "2026-10-16T00:49:43+00:00")
        .into_bytes();
    check!(ne; alternate, canonical);
    let packet = RetainedProviderSchoolPacketV1::from_root(
        RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        &alternate,
    )?;
    let root = root(&alternate, raw.reference()?.as_str())?;
    let roster = packet.roster();
    let metadata =
        serde_json::to_value(roster.roster_metadata(&root, &alternate, fixture::BODY)?)?;
    check!(eq; metadata["retained_acquired_at"], "2026-10-16T00:49:43+00:00");
    let changed = String::from_utf8(alternate)?
        .replace("2026-10-16T00:49:43+00:00", "2026-10-16T00:49:44+00:00")
        .into_bytes();
    check!(roster
        .roster_metadata(&root, &changed, fixture::BODY)
        .is_err());
    Ok(())
}

fn root(bytes: &[u8], key: &str) -> TestResult<RetainedCaptureRootV1> {
    Ok(RetainedCaptureRootV1 {
        schema: RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
        key: key.into(),
        payload: AttachmentObjectReferenceV1 {
            digest: AttachmentDigestV1::parse(&hash(bytes))?,
            bytes: u64::try_from(bytes.len())?,
        },
    })
}
