use super::*;
use crate::participation_evidence::recorded::{
    decode_provider_school_packet, read_original_provider_school,
};

#[test]
fn original_raw_school_keeps_literal_bytes_semantic_v1_and_exact_decode_costs() -> TestResult {
    let (_directory, store) = fresh_store()?;
    let packet = packet(DIRECTORY)?;
    let bytes = packet.encode()?;
    let reference = packet.reference()?;
    let expected = packet.read_semantic_contract_v1()?;
    let recording = Recording::new();
    record_provider_school_intake(&store, &recording, &packet)?;
    crate::recording::prepare_recorded(recording.drain(), "original-raw-school-reader")?
        .apply(&store)?;
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let root = snapshot
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1,
            reference.as_str(),
            &mut grant,
        )?
        .ok_or("original Raw school root")?;
    let literal = root.payload().get().to_owned();
    check!(eq; serde_json::from_str::<serde_json::Value>(&literal)?, serde_json::to_value(&packet)?);
    let before = grant.usage();
    let actual = read_original_provider_school(&root, &mut grant)?;
    check!(eq; grant.usage(), before);
    check!(eq; actual.semantic_contract(), crate::participation_evidence::provider_school::PROVIDER_SCHOOL_SEMANTIC_CONTRACT_V1);
    check!(eq; actual.reference(), expected.reference());
    check!(eq; actual.school(), expected.school());
    check!(eq; actual.observation(), expected.observation());
    check!(eq; actual.directory_proof().row(), expected.directory_proof().row());
    check!(eq; actual.directory_proof().row_sha256(), expected.directory_proof().row_sha256());
    let whitespace = format!(" \n{}\n ", root.payload().get());
    let decoded =
        decode_provider_school_packet(root.root().schema, whitespace.as_bytes(), &mut grant)?;
    check!(eq; decoded.encode()?, bytes);
    check!(eq; decoded.reference()?, reference);
    check!(eq; grant.usage(), before);
    check!(eq; root.payload().get(), literal);
    Ok(())
}
