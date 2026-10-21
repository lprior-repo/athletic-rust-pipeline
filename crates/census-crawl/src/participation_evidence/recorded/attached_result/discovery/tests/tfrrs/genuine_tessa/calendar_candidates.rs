use super::*;
use crate::participation_evidence::recorded::{
    read_recorded_independent_participation, record_independent_participation_packet,
    with_current_attached_participation, CheckedDocumentaryParticipationFactsV1,
};
use crate::participation_evidence::result_ownership::calendar_correspondence::{
    build_claim, ResultCalendarSourceProposalV1,
};
use crate::participation_evidence::{
    IndependentParticipationPacketV1, ParticipationEvidenceRefV1, RetainedHtmlCaptureV1,
};
use census_domain::result_calendar_correspondence::{
    ResultCalendarSourceClaimV1, RoncalliSeasonEventSelectionV1,
};

#[test]
fn unrelated_same_body_status_does_not_inflate_selected_calendar_context() -> TestResult {
    let mut installed = source::install()?;
    review::attach(&installed, false)?;
    review::attach(&installed, true)?;
    let expected = claim(&installed)?;
    let original = installed.participation.clone();
    let second = record_second(&installed)?;
    check!(ne; second, original);
    absent_is_sticky(&installed, &second)?;
    installed.participation = second;
    review::attach_named(
        &installed,
        true,
        Some("genuine-tessa-second-status-test-attachment"),
    )?;
    installed.participation = original;
    let before = super::super::setup::state(&installed.store, &installed.context)?;
    same_originals_are_separate_sources(&installed)?;
    let mut actual = claim(&installed)?;
    check!(ne; actual.attachment_inventory, expected.attachment_inventory);
    actual.attachment_inventory = expected.attachment_inventory.clone();
    check!(eq; actual, expected);
    check!(eq; super::super::setup::state(&installed.store, &installed.context)?, before);
    source::originals(&installed.store, &installed.originals)?;
    Ok(())
}

fn record_second(installed: &source::Installed) -> TestResult<ParticipationEvidenceRefV1> {
    let original = roncalli_fixture::capture(
        "https://athletics.roncalli.org/girls-track-field/",
        roncalli_fixture::REVIEW.as_bytes(),
    )?;
    let review = controlled_second_capture(&original)?;
    check!(eq; review.body_sha256(), original.body_sha256());
    check!(ne; objects::reference(&serialized_bytes(&review)?)?, objects::reference(&serialized_bytes(&original)?)?);
    let schedule = roncalli_fixture::capture(
        "https://athletics.roncalli.org/girls-track-field/schedules/",
        roncalli_fixture::SCHEDULE.as_bytes(),
    )?;
    let packet = IndependentParticipationPacketV1::from_roncalli_season_event(
        review,
        schedule,
        RoncalliSeasonEventSelectionV1 {
            status_subject_start: 78347,
            status_subject_end: 78365,
            calendar_recap_id: "event-3387-recap".into(),
        },
    )?;
    let checked = packet.read(
        installed.store.store_identity()?,
        installed.store.snapshot().sequence(),
    )?;
    check!(eq; checked.subject().name(), "Tessa Mize");
    check!(eq; checked.subject().locator(), "html_bytes:78347:78365");
    check!(eq; checked.cohort(), GradYear::CO2027);
    let recording = Recording::new();
    record_independent_participation_packet(&installed.store, &recording, &packet)?;
    crate::recording::prepare_recorded(
        recording.drain(),
        "genuine-tessa-second-status-original-controlled-review",
    )?
    .apply(&installed.store)?;
    Ok(packet.reference()?)
}

fn claim(installed: &source::Installed) -> TestResult<ResultCalendarSourceClaimV1> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let requested =
        read_recorded_independent_participation(&snapshot, &installed.participation, &mut grant)?
            .ok_or("requested genuine Roncalli source")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    let proposal = plan.with_group(0, &mut grant, |linked, grant| {
        build_claim(&current, &linked, 0, &requested, grant)
    })?;
    require_context(&proposal, &mut grant)?;
    grant.ensure_available()?;
    Ok(proposal.claim().clone())
}

fn require_context(
    proposal: &ResultCalendarSourceProposalV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult {
    let context = proposal.borrowed_context(grant)?;
    check!(eq; context.len(), proposal.claim().evidence.as_slice().len());
    Ok(())
}

fn same_originals_are_separate_sources(installed: &source::Installed) -> TestResult {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    with_current_attached_participation(&current, &mut grant, |sources, grant| {
        plan.with_group(0, grant, |linked, grant| {
            sources.with_row_facts(&linked, 0, grant, |facts, _| Ok(two_sources(facts)))
        })
    })??;
    grant.ensure_available()?;
    Ok(())
}

fn two_sources(facts: CheckedDocumentaryParticipationFactsV1<'_>) -> TestResult {
    let [first, second] = facts.sources() else {
        return Err("two independently attached genuine status selections required".into());
    };
    check!(ne; first.root(), second.root());
    check!(ne; first.source().reference(), second.source().reference());
    check!(eq; first.source().status_proof().capture(), second.source().status_proof().capture());
    check!(eq; first.source().season_proof().capture(), second.source().season_proof().capture());
    check!(eq; first.source().day(), second.source().day());
    check!(eq; first.source().subject().name(), second.source().subject().name());
    Ok(())
}

fn absent_is_sticky(
    installed: &source::Installed,
    reference: &ParticipationEvidenceRefV1,
) -> TestResult {
    let snapshot = installed.store.snapshot();
    let before = super::super::setup::state(&installed.store, &installed.context)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let requested = read_recorded_independent_participation(&snapshot, reference, &mut grant)?
        .ok_or("recorded but unattached genuine source")?;
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let plan = discover_current_html_results(&current, &mut grant)?;
    let result = plan.with_group(0, &mut grant, |linked, grant| {
        build_claim(&current, &linked, 0, &requested, grant)
    });
    check!(
        matches!(result, Err(crate::CrawlError::Store(census_store::StoreError::Refused { detail })) if detail == "calendar requested status absent")
    );
    check!(grant.ensure_available().is_err());
    check!(eq; super::super::setup::state(&installed.store, &installed.context)?, before);
    Ok(())
}

fn controlled_second_capture(
    original: &RetainedHtmlCaptureV1,
) -> TestResult<RetainedHtmlCaptureV1> {
    let mut provenance = original.provenance().clone();
    let time = original
        .acquired_at()
        .checked_add_signed(chrono::Duration::seconds(1))
        .ok_or("controlled second capture timestamp overflow")?;
    provenance.started_at = time;
    Ok(RetainedHtmlCaptureV1::new(
        original.requested_url(),
        original.requested_url(),
        200,
        time,
        roncalli_fixture::REVIEW.as_bytes(),
        provenance,
    )?)
}
