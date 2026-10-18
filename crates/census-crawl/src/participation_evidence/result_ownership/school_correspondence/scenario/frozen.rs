use super::*;
use census_domain::model::{SchoolId, StoreIdentity};
use census_store::{replay_result_school_source_proof_v1, RetainedCaptureRootSchemaV1};

#[path = "frozen/retention.rs"]
mod retention;

pub(super) struct Capture {
    bytes: Vec<u8>,
    reference: AttachmentObjectReferenceV1,
    store: StoreIdentity,
    sequence: u64,
}

pub(super) fn capture(store: &Store) -> TestResult<Capture> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_school_correspondences_v1(&attachments, &mut grant)?;
    let identity = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&identity, &mut grant)?;
    Ok(Capture {
        bytes: proof.bytes().to_vec(),
        reference: proof.reference().clone(),
        store: current.store_identity().clone(),
        sequence: current.snapshot_sequence(),
    })
}

pub(super) fn observe(
    proof: &Capture,
    store: &Store,
    input: &setup::Input,
    expected: Option<&SchoolId>,
) -> TestResult {
    check!(eq; store.snapshot().store_identity()?, proof.store);
    let before = setup::state(store)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = replay_result_school_source_proof_v1(
        &proof.bytes,
        &proof.reference,
        &proof.store,
        proof.sequence,
        &mut grant,
    )?;
    check!(eq; current.store_identity(), &proof.store);
    check!(eq; current.snapshot_sequence(), proof.sequence);
    check!(!current.records().is_empty());
    let bytes = grant.usage().encoded_bytes();
    let active = current.active_records(&mut grant)?.count();
    check!(eq; grant.usage().encoded_bytes(), bytes);
    check!(eq; current.active_records(&mut grant)?.count(), active);
    check!(eq; grant.usage().encoded_bytes(), bytes);
    let root = current
        .read_retained_capture_root_v1(
            RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            &input.context,
            &mut grant,
        )?
        .ok_or("frozen original result")?;
    let capture: RetainedHtmlCaptureV1 = serde_json::from_str(root.payload().get())?;
    check!(eq; capture.body()?.as_bytes(), setup::RAW);
    let attachments = current.attachments(&mut grant)?;
    let discovered = discover_frozen_html_results(&attachments, &mut grant)?;
    check!(eq; discovered.group_count(), 1);
    let observed = discovered.with_group(0, &mut grant, |linked, grant| {
        Ok(read::checked_frozen(&current, &linked, 0, grant)?
            .map(|checked| checked.school().clone()))
    })?;
    check!(eq; observed.as_ref(), expected);
    check!(eq; setup::state(store)?, before);
    let locators = setup::locators();
    let locator = locators.first().ok_or("no source row locator")?;
    retention::attempt_retention(
        &proof.bytes,
        &proof.reference,
        &proof.store,
        proof.sequence,
        &input.context,
        expected,
        locator,
    )?;
    Ok(())
}
