use super::{
    documentary_lifecycle, documentary_material, documentary_review, point_source,
    school_authority, source,
};
use base64::Engine as _;
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, read_recorded_independent_participation,
};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::{
    build_claim, ResultCalendarSourceProposalV1,
};
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::{
    AttachmentPredecessorV1, AttachmentReplayGrantV1,
};
use census_domain::model::{Gender, Sport};
use census_store::{AttachmentContextObjectV1, JournalReadBudget};
use sha2::Digest as _;
use source::{InstalledSource, TestResult};

#[test]
fn calendar_proposal_binds_original_day_and_refuses_withdrawn_documentary_authority() -> TestResult
{
    let installed = source::install()?;
    let points = point_source::install(&installed)?;
    let fifty = points.get(1).ok_or("May15 source missing")?;
    school_authority::establish(&installed, fifty)?;
    let accepted = documentary_review::accept(
        &installed,
        fifty,
        documentary_material::Kind::Point,
        AttachmentPredecessorV1::First,
        "controlled-calendar-source-status",
    )?;
    let other_day = points.get(2).ok_or("May16 source missing")?;
    documentary_review::accept(
        &installed,
        other_day,
        documentary_material::Kind::Point,
        AttachmentPredecessorV1::First,
        "controlled-calendar-other-day-status",
    )?;
    let proposal = propose(&installed, fifty)?;
    check!(eq; proposal.claim().day.as_str(), "2026-05-15");
    check!(eq; proposal.claim().sport, Sport::OutdoorTrack);
    check!(eq; proposal.claim().category, Gender::Boys);
    require_schedule_original_body(&installed, fifty, &proposal)?;
    check!(propose(&installed, other_day).is_err());
    documentary_lifecycle::revoke(&installed, &accepted)?;
    check!(propose(&installed, fifty).is_err());
    documentary_lifecycle::replay_without_reactivation(&installed, &accepted)?;
    check!(propose(&installed, fifty).is_err());
    source::require_originals(&installed.store, &installed.source_records)?;
    check!(eq; proposal.claim().day.as_str(), "2026-05-15");
    Ok(())
}

fn require_schedule_original_body(
    installed: &InstalledSource,
    reference: &ParticipationEvidenceRefV1,
    proposal: &ResultCalendarSourceProposalV1,
) -> TestResult {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let requested = read_recorded_independent_participation(&snapshot, reference, &mut grant)?
        .ok_or("calendar documentary capture missing")?;
    let leaf = requested.calendar_original_body();
    check!(eq; leaf.version, 1);
    check!(leaf.slot == census_store::RetainedCaptureSlotV1::ParticipationSchedule);
    check!(leaf.kind == census_store::RetainedCaptureLeafKindV1::OriginalBody);
    check!(eq; leaf.root.key.as_str(), requested.root().key.as_str());
    check!(eq; leaf.object.digest.as_str(), requested.checked().season_proof().capture());
    check!(eq; proposal.claim().calendar.capture.as_str(), leaf.object.digest.as_str());
    check!(proposal
        .claim()
        .evidence
        .as_slice()
        .iter()
        .any(|held| held == &leaf.object));
    let context = proposal.borrowed_context(&mut grant)?;
    check!(context.iter().any(|held| matches!(
        held,
        AttachmentContextObjectV1::Retained(actual) if *actual == leaf
    )));
    snapshot.verify_retained_capture_leaf_v1(leaf, &mut grant)?;
    let raw = snapshot
        .journal_payload_raw_bounded(
            requested.root().schema.phase(),
            &requested.root().key,
            JournalReadBudget::new(2 * 1024 * 1024)?,
        )?
        .ok_or("calendar packet payload missing")?;
    #[derive(serde::Deserialize)]
    struct PacketSlots<'a> {
        #[serde(borrow)]
        schedule: &'a serde_json::value::RawValue,
    }
    #[derive(serde::Deserialize)]
    struct ScheduleView<'a> {
        body_bytes: usize,
        #[serde(borrow)]
        body_sha256: &'a str,
        #[serde(borrow)]
        body_base64: &'a str,
    }
    let slots: PacketSlots<'_> = serde_json::from_str(raw.get())?;
    let view: ScheduleView<'_> = serde_json::from_str(slots.schedule.get())?;
    if view.body_bytes == 0 || view.body_bytes > 512 * 1024 {
        return Err("schedule body bounds".into());
    }
    let body = base64::engine::general_purpose::STANDARD.decode(view.body_base64)?;
    check!(eq; body.len(), view.body_bytes);
    check!(eq; view.body_sha256, leaf.object.digest.as_str());
    let digest = format!("{:x}", sha2::Sha256::digest(&body));
    check!(eq; digest.as_str(), leaf.object.digest.as_str());
    Ok(())
}

pub(super) fn propose(
    installed: &InstalledSource,
    reference: &ParticipationEvidenceRefV1,
) -> TestResult<ResultCalendarSourceProposalV1> {
    let locator = installed.row_locators.get(1).ok_or("May15 row missing")?;
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let requested = read_recorded_independent_participation(&snapshot, reference, &mut grant)?
        .ok_or("calendar documentary capture missing")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let discovered = discover_current_html_results(&current, &mut grant)?;
    for index in 0..discovered.group_count() {
        let proposal = discovered.with_group(index, &mut grant, |linked, grant| {
            if linked.recorded_root().key.as_str() != locator.document_key.as_str() {
                return Ok(None);
            }
            let row = linked.row(0).ok_or(census_store::StoreError::Refused {
                detail: "controlled calendar row missing".into(),
            })?;
            if row.checked_row().locator() != locator.locator {
                return Err(census_store::StoreError::Refused {
                    detail: "controlled calendar row locator differs".into(),
                }
                .into());
            }
            build_claim(&current, &linked, 0, &requested, grant).map(Some)
        })?;
        if let Some(proposal) = proposal {
            return Ok(proposal);
        }
    }
    Err("controlled calendar result group missing".into())
}
