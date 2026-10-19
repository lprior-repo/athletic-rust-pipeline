use super::super::calendar_source;
use super::super::source::{self, InstalledSource, TestResult};
use super::multi_subject_fixture::{accept_pair, sibling_may15};
use census_crawl::participation_evidence::recorded::{
    discover_frozen_html_results, with_frozen_attached_participation,
};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::read::checked_frozen;
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_store::{
    Application, FrozenCurrentEvidenceSubjectAttachmentReferencesV1,
    ReplayedResultCalendarCorrespondencesV1, StoreError,
};
fn accept_calendar(installed: &InstalledSource, base: &ParticipationEvidenceRefV1) -> TestResult {
    let proposal = calendar_source::propose(installed, base)?;
    let inspection = installed
        .store
        .inspect_result_calendar_correspondence_v1(AttachmentReplayGrantV1::new())?;
    let command = census_domain::result_calendar_correspondence::ResultCalendarReviewCommandV1 { version: 1, policy: super::calendar_text(census_domain::result_calendar_correspondence::RESULT_CALENDAR_POLICY_V1)?, store: inspection.store_identity().clone(), operation: super::calendar_text("controlled-calendar-multi-accept")?, input_token: inspection.input_token().clone(), evidence_generation: inspection.evidence_generation(), derived_generation: inspection.derived_generation(), accepted_head: inspection.public_head().clone(), expected_inventory: inspection.inventory().clone(), source: proposal.claim().clone(), action: census_domain::result_calendar_correspondence::ResultCalendarActionV1::Accept { prior: census_domain::result_calendar_correspondence::ResultCalendarPredecessorV1::First }, actor: census_domain::result_calendar_correspondence::ResultCalendarReviewActorV1::LocalOperator, reviewed_at: super::review_time()?, rationale: super::calendar_text("Explicit controlled calendar review")? };
    let context = proposal.borrowed_context(&mut AttachmentReplayGrantV1::new())?;
    let prepared = installed.store.prepare_result_calendar_correspondence_v1(
        command,
        census_store::ResultCalendarCorrespondencePreparationV1::New {
            inspection,
            context: &context,
        },
    )?;
    if prepared.is_replay() {
        return Err("frozen sibling preparation replay".into());
    }
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().clone();
    match installed
        .store
        .apply_result_calendar_correspondence_v1(prepared, &approved, &digest)?
    {
        Application::Written(_) => Ok(()),
        Application::Repeated(_) => Err("frozen sibling replayed".into()),
    }
}
fn frozen_reference(
    installed: &InstalledSource,
    replayed: &ReplayedResultCalendarCorrespondencesV1<'_>,
    frozen: &FrozenCurrentEvidenceSubjectAttachmentReferencesV1<'_>,
    grant: &mut AttachmentReplayGrantV1,
) -> census_crawl::CrawlResult<String> {
    let plan = discover_frozen_html_results(frozen, grant)?;
    let may15 = installed.row_locators.get(1).ok_or(StoreError::Refused {
        detail: "frozen May15 locator absent".into(),
    })?;
    let mut found: Option<String> = None;
    for index in 0..plan.group_count() {
        let observed: Option<String> = plan.with_group(index, grant, |linked, grant| {
            if linked.recorded_root().key.as_str() != may15.document_key.as_str() {
                return Ok(None);
            }
            let row = linked.row(0).ok_or(StoreError::Refused {
                detail: "frozen May15 row absent".into(),
            })?;
            if row.checked_row().locator() != may15.locator {
                return Err(StoreError::Refused {
                    detail: "frozen May15 row locator differs".into(),
                }
                .into());
            }
            with_frozen_attached_participation(frozen, grant, |attached, grant| {
                attached.with_row_facts(&linked, 0, grant, |facts, grant| {
                    let checked = checked_frozen(replayed, &linked, 0, &facts, grant)?.ok_or(
                        StoreError::Refused {
                            detail: "frozen calendar absent".into(),
                        },
                    )?;
                    Ok(Some(checked.documentary_reference().as_str().to_owned()))
                })
            })
        })?;
        if let Some(reference) = observed {
            if found.is_some() {
                return Err(StoreError::Refused {
                    detail: "frozen May15 ambiguous".into(),
                }
                .into());
            }
            found = Some(reference);
        }
    }
    found.ok_or(
        StoreError::Refused {
            detail: "frozen May15 group absent".into(),
        }
        .into(),
    )
}
#[test]
fn calendar_multi_subject_frozen_binds_exact_packet() -> TestResult {
    let installed = source::install()?;
    let points = super::super::point_source::install(&installed)?;
    let base = points.get(1).ok_or("May15 base absent")?.clone();
    super::super::school_authority::establish(&installed, &base)?;
    let sibling = sibling_may15(&installed, &base)?;
    accept_pair(&installed, &base, &sibling)?;
    let (prepared, digest, approved) = super::prepare_enrollment(&installed)?;
    match installed
        .store
        .apply_result_calendar_enrollment_v1(prepared, &approved, &digest)?
    {
        Application::Written(_) => {}
        Application::Repeated(_) => return Err("frozen enrollment replayed".into()),
    };
    accept_calendar(&installed, &base)?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let current =
        snapshot.read_current_result_calendar_correspondences_v1(&attachments, &mut grant)?;
    let frozen_identity = snapshot.freeze_public_identity_v2()?;
    let proof = current.freeze_closure(&frozen_identity, &mut grant)?;
    let sequence = current.snapshot_sequence();
    let identity = current.store_identity().clone();
    let (reference, bytes) = proof.into_parts();
    let mut replay_grant = AttachmentReplayGrantV1::new();
    let replayed = census_store::replay_result_calendar_source_proof_v1(
        &bytes,
        &reference,
        &identity,
        sequence,
        &mut replay_grant,
    )?;
    let frozen = replayed.attachments(&mut replay_grant)?;
    let documentary = frozen_reference(&installed, &replayed, frozen, &mut replay_grant)?;
    check!(eq; documentary.as_str(), base.as_str());
    check!(documentary.as_str() != sibling.as_str());
    source::require_originals(&installed.store, &installed.source_records)?;
    Ok(())
}
