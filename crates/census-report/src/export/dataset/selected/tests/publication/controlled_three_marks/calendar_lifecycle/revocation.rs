mod publication;

use super::super::source::{InstalledSource, TestResult};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::model::StoreIdentity;
use census_domain::result_calendar_correspondence::{
    ResultCalendarActionV1, ResultCalendarDigestV1, ResultCalendarInventoryV1,
    ResultCalendarPredecessorV1, ResultCalendarRecordReferenceV1, ResultCalendarReviewActorV1,
    ResultCalendarReviewCommandV1, ResultCalendarReviewedRecordV1, RESULT_CALENDAR_POLICY_V1,
};
use census_store::{Application, Receipt, ResultCalendarCorrespondencePreparationV1, Store};

pub(in crate::export::dataset::selected::tests::publication) fn enroll_calendar(
    store: &Store,
    operation: &str,
    rationale: &str,
) -> TestResult {
    let (prepared, digest, approved) =
        super::prepare_enrollment(store, operation, rationale)?;
    match store.apply_result_calendar_enrollment_v1(prepared, &approved, &digest)? {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Err("calendar enrollment unexpectedly replayed".into()),
    }
}

pub(in crate::export::dataset::selected::tests::publication) fn apply_calendar_action(
    store: &Store,
    proposal: &ResultCalendarSourceProposalV1,
    operation: &str,
    action: ResultCalendarActionV1,
    rationale: &str,
) -> TestResult<(ResultCalendarReviewCommandV1, Receipt)> {
    let inspection =
        store.inspect_result_calendar_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = ResultCalendarReviewCommandV1 {
        version: 1,
        policy: super::calendar_text(RESULT_CALENDAR_POLICY_V1)?,
        store: inspection.store_identity().clone(),
        operation: super::calendar_text(operation)?,
        input_token: inspection.input_token().clone(),
        evidence_generation: inspection.evidence_generation(),
        derived_generation: inspection.derived_generation(),
        accepted_head: inspection.public_head().clone(),
        expected_inventory: inspection.inventory().clone(),
        source: proposal.claim().clone(),
        action,
        actor: ResultCalendarReviewActorV1::LocalOperator,
        reviewed_at: super::review_time()?,
        rationale: super::calendar_text(rationale)?,
    };
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = store.prepare_result_calendar_correspondence_v1(
        command.clone(),
        ResultCalendarCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    if prepared.is_replay() {
        return Err("calendar preparation returned replay".into());
    }
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_calendar_correspondence_v1(prepared, &approved, &digest)? {
        Application::Written(receipt) => Ok((command, receipt)),
        Application::Repeated(_) => Err("calendar action unexpectedly replayed".into()),
    }
}

fn require_current(
    store: &Store,
    records: usize,
    receipts: usize,
    active: usize,
) -> TestResult<(
    ResultCalendarInventoryV1,
    Vec<ResultCalendarReviewedRecordV1>,
)> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    check!(eq; current.records().len(), records);
    check!(eq; current.receipts().len(), receipts);
    check!(eq; current.active_records(&mut grant)?.count(), active);
    Ok((current.inventory().clone(), current.records().to_vec()))
}

fn freeze_calendar(
    store: &Store,
) -> TestResult<(Vec<u8>, AttachmentObjectReferenceV1, u64, StoreIdentity)> {
    let snapshot = store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    let frozen = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&frozen, &mut grant)?;
    let sequence = current.snapshot_sequence();
    let identity = current.store_identity().clone();
    let (reference, bytes) = proof.into_parts();
    Ok((bytes, reference, sequence, identity))
}

fn require_frozen(
    bytes: &[u8],
    reference: &AttachmentObjectReferenceV1,
    identity: &StoreIdentity,
    sequence: u64,
    expected: &ResultCalendarInventoryV1,
    records: usize,
    active: usize,
) -> TestResult<Vec<ResultCalendarReviewedRecordV1>> {
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed = census_store::replay_result_calendar_source_proof_v1(
        bytes, reference, identity, sequence, &mut grant,
    )?;
    check!(eq; replayed.inventory(), expected);
    check!(eq; replayed.records().len(), records);
    check!(eq; replayed.active_records(&mut grant)?.count(), active);
    Ok(replayed.records().to_vec())
}

pub(in crate::export::dataset::selected::tests::publication) fn replay_original_without_reactivation(
    store: &Store,
    command: ResultCalendarReviewCommandV1,
    expected: &Receipt,
) -> TestResult {
    let before = store.receipt_count()?;
    let prepared = store.prepare_result_calendar_correspondence_v1(
        command,
        ResultCalendarCorrespondencePreparationV1::ReplayOnly,
    )?;
    if !prepared.is_replay() {
        return Err("acceptance replay returned new application".into());
    }
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match store.apply_result_calendar_correspondence_v1(prepared, &approved, &digest)? {
        Application::Repeated(receipt) => check!(eq; receipt, *expected),
        Application::Written(_) => return Err("acceptance replay unexpectedly wrote".into()),
    }
    check!(eq; store.receipt_count()?, before);
    Ok(())
}

fn require_cold_revoked(
    installed: InstalledSource,
    expected: &ResultCalendarInventoryV1,
    first: &ResultCalendarDigestV1,
    second: &ResultCalendarDigestV1,
) -> TestResult {
    let InstalledSource {
        directory,
        store,
        identity_writer,
        candidate,
        public,
        source_records,
        ..
    } = installed;
    drop(identity_writer);
    drop(store);
    let reopened = super::super::reopen(&directory)?;
    super::super::source::require_identity(&reopened, &candidate, &public)?;
    super::super::source::require_originals(&reopened, &source_records)?;
    let (inventory, records) = require_current(&reopened, 2, 2, 0)?;
    check!(eq; inventory, *expected);
    let first_record = records.first().ok_or("reopened accepted record absent")?;
    let second_record = records.get(1).ok_or("reopened revoking record absent")?;
    check!(eq; first_record.record_digest, *first);
    check!(eq; second_record.record_digest, *second);
    Ok(())
}

#[test]
fn calendar_revoke_keeps_revoked_across_replay_and_reopen() -> TestResult {
    let (installed, points, _documentary) = super::install_reviewed()?;
    enroll_calendar(
        &installed.store,
        "controlled-calendar-enroll",
        "Explicit controlled calendar enrollment after school9 and reviewed status",
    )?;
    let proposal = super::proposal::may15_proposal(&installed, &points)?;
    let claim = proposal.claim().clone();
    let (accept_command, accept_receipt) = apply_calendar_action(
        &installed.store,
        &proposal,
        "controlled-calendar-accept",
        ResultCalendarActionV1::Accept {
            prior: ResultCalendarPredecessorV1::First,
        },
        "Explicit controlled calendar review",
    )?;
    let (accept_inventory, accept_records) = require_current(&installed.store, 1, 1, 1)?;
    let accept_record = accept_records.first().ok_or("accepted record absent")?;
    check!(eq; accept_record.command.source, claim);
    check!(eq; accept_record.record_digest.as_str(), accept_receipt.digest.as_str());
    let (old_bytes, old_reference, old_sequence, old_identity) = freeze_calendar(&installed.store)?;
    let old_records = require_frozen(
        &old_bytes,
        &old_reference,
        &old_identity,
        old_sequence,
        &accept_inventory,
        1,
        1,
    )?;
    check!(eq;
        old_records.first().ok_or("old frozen record absent")?.record_digest,
        accept_record.record_digest);
    let (_, revoke_receipt) = apply_calendar_action(
        &installed.store,
        &proposal,
        "controlled-calendar-revoke",
        ResultCalendarActionV1::Revoke {
            prior: ResultCalendarRecordReferenceV1 {
                operation: accept_record.command.operation.clone(),
                digest: accept_record.record_digest.clone(),
            },
        },
        "Explicit controlled calendar review",
    )?;
    drop(revoke_receipt);
    let (revoked_inventory, revoked_records) = require_current(&installed.store, 2, 2, 0)?;
    let revoke_record = revoked_records.get(1).ok_or("revoking record absent")?;
    check!(eq; revoke_record.command.source, claim);
    match &revoke_record.command.action {
        ResultCalendarActionV1::Revoke { prior } => {
            check!(eq; prior.operation, accept_record.command.operation);
            check!(eq; prior.digest, accept_record.record_digest);
        }
        _ => return Err("revoking record action differs".into()),
    }
    replay_original_without_reactivation(&installed.store, accept_command, &accept_receipt)?;
    let (still_revoked, _) = require_current(&installed.store, 2, 2, 0)?;
    check!(eq; still_revoked, revoked_inventory);
    let (new_bytes, new_reference, new_sequence, new_identity) = freeze_calendar(&installed.store)?;
    require_frozen(
        &new_bytes,
        &new_reference,
        &new_identity,
        new_sequence,
        &revoked_inventory,
        2,
        0,
    )?;
    require_frozen(
        &old_bytes,
        &old_reference,
        &old_identity,
        old_sequence,
        &accept_inventory,
        1,
        1,
    )?;
    super::super::source::require_originals(&installed.store, &installed.source_records)?;
    require_cold_revoked(
        installed,
        &revoked_inventory,
        &accept_record.record_digest,
        &revoke_record.record_digest,
    )
}