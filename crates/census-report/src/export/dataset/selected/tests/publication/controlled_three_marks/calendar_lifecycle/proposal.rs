use super::super::calendar_source;
use super::super::source::TestResult;
use super::install_reviewed;
use base64::Engine as _;
use census_crawl::participation_evidence::recorded::read_recorded_independent_participation;
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::ResultCalendarSourceProposalV1;
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{Gender, Sport};
use census_domain::result_calendar_correspondence::RESULT_CALENDAR_PARSER_CONTRACT_V1;
use census_store::{AttachmentContextObjectV1, JournalReadBudget, RetainedCaptureLeafKindV1};
use sha2::Digest as _;

pub(super) fn may15_proposal(
    installed: &super::super::source::InstalledSource,
    points: &[ParticipationEvidenceRefV1],
) -> TestResult<ResultCalendarSourceProposalV1> {
    let fifty = points.get(1).ok_or("May15 source missing")?;
    calendar_source::propose(installed, fifty)
}

fn require_leaf_binding(
    installed: &super::super::source::InstalledSource,
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
    check!(leaf.kind == RetainedCaptureLeafKindV1::OriginalBody);
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
    Ok(())
}

fn require_body_bytes(
    installed: &super::super::source::InstalledSource,
    reference: &ParticipationEvidenceRefV1,
    proposal: &ResultCalendarSourceProposalV1,
) -> TestResult {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let requested = read_recorded_independent_participation(&snapshot, reference, &mut grant)?
        .ok_or("calendar documentary capture missing")?;
    let leaf = requested.calendar_original_body();
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
    check!(eq; proposal.claim().calendar.capture.as_str(), digest.as_str());
    Ok(())
}

fn require_exact_scope(proposal: &ResultCalendarSourceProposalV1) -> TestResult {
    check!(eq; proposal.claim().day.as_str(), "2026-05-15");
    check!(eq; proposal.claim().sport, Sport::OutdoorTrack);
    check!(eq; proposal.claim().category, Gender::Boys);
    check!(proposal.claim().category != Gender::Unknown);
    let contract = proposal.claim().parser_contract.digest.as_str().to_owned();
    let expected_digest = format!(
        "{:x}",
        sha2::Sha256::digest(RESULT_CALENDAR_PARSER_CONTRACT_V1)
    );
    check!(eq; contract.as_str(), expected_digest.as_str());
    check!(!proposal.claim().evidence.is_empty());
    check!(proposal.claim().evidence.len() <= 32);
    Ok(())
}

#[test]
fn calendar_proposal_binds_exact_day_status_sport_category() -> TestResult {
    let (installed, points, _accepted) = install_reviewed()?;
    let fifty = points.get(1).ok_or("May15 source missing")?.clone();
    let other = points.get(2).ok_or("May16 source missing")?.clone();
    let proposal = may15_proposal(&installed, &points)?;
    require_exact_scope(&proposal)?;
    require_leaf_binding(&installed, &fifty, &proposal)?;
    require_body_bytes(&installed, &fifty, &proposal)?;
    check!(calendar_source::propose(&installed, &other).is_err());
    super::super::source::require_originals(&installed.store, &installed.source_records)?;
    Ok(())
}
