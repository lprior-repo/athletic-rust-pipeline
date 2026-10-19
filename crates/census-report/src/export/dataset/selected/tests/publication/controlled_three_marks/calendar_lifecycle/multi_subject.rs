use super::super::calendar_source;
use super::super::source::{self, InstalledSource, TestResult};
use super::multi_subject_fixture::{accept_pair, sibling_may15};
use census_crawl::participation_evidence::recorded::read_recorded_independent_participation;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{Gender, Sport};
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarPredecessorV1, ResultCalendarReviewCommandV1,
    ResultCalendarReviewedRecordV1, RESULT_CALENDAR_POLICY_V1,
};
use census_store::{
    Application, AttachmentContextObjectV1, Receipt, ResultCalendarCorrespondencePreparationV1,
    RetainedCaptureRootV1, Store,
};
fn accept_calendar(
    installed: &InstalledSource,
    proposal: &census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1,
    operation: &str,
) -> TestResult<(ResultCalendarReviewCommandV1, Receipt)> {
    let inspection = installed
        .store
        .inspect_result_calendar_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = ResultCalendarReviewCommandV1 { version: 1, policy: super::calendar_text(RESULT_CALENDAR_POLICY_V1)?, store: inspection.store_identity().clone(), operation: super::calendar_text(operation)?, input_token: inspection.input_token().clone(), evidence_generation: inspection.evidence_generation(), derived_generation: inspection.derived_generation(), accepted_head: inspection.public_head().clone(), expected_inventory: inspection.inventory().clone(), source: proposal.claim().clone(), action: ResultCalendarActionV1::Accept { prior: ResultCalendarPredecessorV1::First }, actor: census_domain::result_calendar_correspondence::ResultCalendarReviewActorV1::LocalOperator, reviewed_at: super::review_time()?, rationale: super::calendar_text("Explicit controlled calendar review")? };
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = installed.store.prepare_result_calendar_correspondence_v1(
        command.clone(),
        ResultCalendarCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    if prepared.is_replay() {
        return Err("sibling calendar preparation replay".into());
    }
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match installed
        .store
        .apply_result_calendar_correspondence_v1(prepared, &approved, &digest)?
    {
        Application::Written(receipt) => Ok((command, receipt)),
        Application::Repeated(_) => Err("sibling calendar replayed".into()),
    }
}
fn proposal_root_key(
    proposal: &census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1,
) -> TestResult<String> {
    let mut grant = AttachmentReplayGrantV1::new();
    let context = proposal.borrowed_context(&mut grant)?;
    let wanted = &proposal.claim().calendar.documentary_root;
    for item in context {
        if let AttachmentContextObjectV1::Inline { reference, bytes } = item {
            if reference == wanted {
                let root: RetainedCaptureRootV1 = serde_json::from_slice(bytes)?;
                return Ok(root.key);
            }
        }
    }
    Err("sibling documentary root absent".into())
}
fn require_current_and_frozen(
    store: &Store,
    expected: &ResultCalendarReviewedRecordV1,
) -> TestResult {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    check!(eq; current.records().len(), 1);
    check!(eq; current.receipts().len(), 1);
    check!(eq; current.active_records(&mut grant)?.count(), 1);
    check!(eq; current.records().first().ok_or("sibling current absent")?.record_digest, expected.record_digest);
    let frozen = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&frozen, &mut grant)?;
    let sequence = current.snapshot_sequence();
    let identity = current.store_identity().clone();
    let inventory = current.inventory().clone();
    let (reference, bytes) = proof.into_parts();
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed = census_store::replay_result_calendar_source_proof_v1(
        &bytes, &reference, &identity, sequence, &mut grant,
    )?;
    check!(eq; replayed.inventory(), &inventory);
    check!(eq; replayed.records().len(), 1);
    check!(eq; replayed.active_records(&mut grant)?.count(), 1);
    let frozen_record = replayed.records().first().ok_or("sibling frozen absent")?;
    check!(eq; frozen_record.record_digest, expected.record_digest);
    check!(eq; frozen_record.command.source, expected.command.source);
    Ok(())
}
#[test]
fn calendar_multi_subject_binds_exact_packet_without_bridging() -> TestResult {
    let installed = source::install()?;
    let points = super::super::point_source::install(&installed)?;
    let base = points.get(1).ok_or("May15 base absent")?.clone();
    let other = points.get(2).ok_or("May16 absent")?.clone();
    super::super::school_authority::establish(&installed, &base)?;
    let sibling = sibling_may15(&installed, &base)?;
    let accepted = accept_pair(&installed, &base, &sibling)?;
    check!(eq; accepted.command.subjects.len(), 2);
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let recorded_a = read_recorded_independent_participation(&snapshot, &base, &mut grant)?
        .ok_or("recorded base absent")?;
    let recorded_b = read_recorded_independent_participation(&snapshot, &sibling, &mut grant)?
        .ok_or("recorded sibling absent")?;
    check!(recorded_a.root().key.as_str() != recorded_b.root().key.as_str());
    check!(recorded_a.checked().reference().as_str() != recorded_b.checked().reference().as_str());
    check!(eq; recorded_a.checked().day().as_str(), recorded_b.checked().day().as_str());
    check!(eq; recorded_a.checked().day().as_str(), "2026-05-15");
    let proposal_a = calendar_source::propose(&installed, &base)?;
    let proposal_b = calendar_source::propose(&installed, &sibling)?;
    check!(eq; proposal_a.claim().day.as_str(), "2026-05-15");
    check!(eq; proposal_a.claim().sport, Sport::OutdoorTrack);
    check!(eq; proposal_a.claim().category, Gender::Boys);
    let root_a_key = proposal_root_key(&proposal_a)?;
    let root_b_key = proposal_root_key(&proposal_b)?;
    check!(eq; root_a_key, recorded_a.root().key);
    check!(eq; root_b_key, recorded_b.root().key);
    check!(root_a_key != root_b_key);
    check!(eq; proposal_a.claim().documentary_attachment.digest, proposal_b.claim().documentary_attachment.digest);
    check!(eq; proposal_a.claim().documentary_attachment.digest, accepted.reference.digest);
    check!(eq; proposal_a.claim().calendar.capture.as_str(), recorded_a.checked().season_proof().capture());
    check!(eq; proposal_b.claim().calendar.capture.as_str(), recorded_b.checked().season_proof().capture());
    check!(calendar_source::propose(&installed, &other).is_err());
    let (prepared, digest, approved) = super::prepare_enrollment(&installed)?;
    match installed
        .store
        .apply_result_calendar_enrollment_v1(prepared, &approved, &digest)?
    {
        Application::Written(_) => {}
        Application::Repeated(_) => return Err("sibling enrollment replayed".into()),
    };
    let (_, receipt_a) =
        accept_calendar(&installed, &proposal_a, "controlled-calendar-multi-accept")?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    let record_a = current
        .records()
        .first()
        .ok_or("sibling calendar absent")?
        .clone();
    check!(eq; record_a.command.source, proposal_a.claim().clone());
    check!(eq; record_a.record_digest.as_str(), receipt_a.digest.as_str());
    require_current_and_frozen(&installed.store, &record_a)?;
    source::require_originals(&installed.store, &installed.source_records)?;
    Ok(())
}
