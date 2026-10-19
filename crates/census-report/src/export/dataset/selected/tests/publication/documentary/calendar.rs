use super::fixture::Fixture;
use super::TestResult;
use census_crawl::participation_evidence::recorded::{
    discover_current_html_results, read_recorded_independent_participation,
};
use census_crawl::participation_evidence::result_ownership::calendar_correspondence::{
    build_claim, ResultCalendarSourceProposalV1,
};
use census_crawl::participation_evidence::ParticipationEvidenceRefV1;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::{Gender, GradYear, Sport};
use census_store::AttachmentContextObjectV1;

mod lifecycle;
mod publication;
mod replay;
mod school_oracle;

fn propose(fixture: &Fixture) -> TestResult<ResultCalendarSourceProposalV1> {
    let reference = fixture
        .expected
        .first()
        .and_then(|row| row.semantic_references.first())
        .ok_or("genuine documentary source reference")?;
    let reference = ParticipationEvidenceRefV1::parse(reference)?;
    let snapshot = fixture.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let requested = read_recorded_independent_participation(&snapshot, &reference, &mut grant)?
        .ok_or("genuine documentary capture")?;
    let leaf = requested.calendar_original_body();
    let current = snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let discovered = discover_current_html_results(&current, &mut grant)?;
    check!(eq; discovered.group_count(), 1);
    check!(eq; discovered.diagnostic_count(), 0);
    let proposal = discovered.with_group(0, &mut grant, |linked, grant| {
        build_claim(&current, &linked, 0, &requested, grant)
    })?;
    let schedule = "26833e4c45506da5d776eb844b106dcfb041f67f7916bec0fb59c950b99693eb";
    check!(eq; leaf.object.digest.as_str(), schedule);
    check!(eq; proposal.claim().calendar.capture.as_str(), schedule);
    let context = proposal.borrowed_context(&mut grant)?;
    check!(context.iter().any(|held| matches!(
        held,
        AttachmentContextObjectV1::Retained(actual) if *actual == leaf
    )));
    snapshot.verify_retained_capture_leaf_v1(leaf, &mut grant)?;
    Ok(proposal)
}

fn require_scope(proposal: &ResultCalendarSourceProposalV1) -> TestResult {
    let claim = proposal.claim();
    check!(eq; claim.day.as_str(), "2026-05-15");
    check!(eq; claim.sport, Sport::OutdoorTrack);
    check!(eq; claim.category, Gender::Boys);
    check!(eq; claim.year.get(), 2025);
    check!(eq; claim.result.row_digest.as_str(), "bc7c7f83bcfc715c9c3343e09cf45f82bee7651ac86838f3427e1097de010b3d");
    check!(eq; claim.result_attachment.operation.as_str(), "genuine-reviewed-may15-b-v1");
    Ok(())
}

#[test]
fn native_genuine_documentary_v4_co2026_calendar_claim_binds_may15_original_schedule() -> TestResult
{
    let fixture = Fixture::open_for(GradYear::new(2026).ok_or("genuine Class-of-2026 cohort")?)?;
    let proposal = propose(&fixture)?;
    require_scope(&proposal)?;
    fixture.require_original_unchanged()?;
    fixture.close()
}
