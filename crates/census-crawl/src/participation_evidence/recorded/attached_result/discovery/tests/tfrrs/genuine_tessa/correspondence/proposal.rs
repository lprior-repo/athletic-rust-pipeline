use super::*;
use crate::participation_evidence::recorded::{
    read_recorded_independent_participation, read_recorded_provider_school,
};

pub(super) fn school(
    installed: &source::Installed,
) -> TestResult<school::ResultSchoolSourceProposalV1> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let provider =
        read_recorded_provider_school(&snapshot, &installed.school_reference, &mut grant)?
            .ok_or("actual Roncalli school root")?;
    let plan = discover_current_html_results(&attachments, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    let scope = school::ResultSchoolScopeV1 {
        day: ProfileDate::parse("2026-03-11")?,
        sport: Sport::IndoorTrack,
        category: Gender::Girls,
        year: SchoolYear::from_date("2026-03-11").ok_or("March11 school year")?,
    };
    let proposal = plan.with_group(0, &mut grant, |linked, grant| {
        school::build_claim(&attachments, &linked, 0, &provider, scope, grant)
    })?;
    let claim = proposal.claim();
    check!(eq; claim.statement.statement_text.as_str(), "Roncalli");
    check!(eq; claim.target.school, installed.school.id);
    check!(eq; claim.target.day.as_str(), "2026-03-11");
    check!(eq; claim.target.sport, Sport::IndoorTrack);
    check!(eq; claim.target.category, Gender::Girls);
    check!(eq; claim.target.year.get(), 2025);
    check!(eq; claim.person.canonical_candidate, installed.athlete.id.cast());
    Ok(proposal)
}

pub(super) fn calendar(
    installed: &source::Installed,
) -> TestResult<calendar::ResultCalendarSourceProposalV1> {
    let snapshot = installed.store.snapshot();
    let mut grant = AttachmentReplayGrantV1::new();
    let attachments =
        snapshot.read_current_evidence_subject_attachment_references_v1(&mut grant)?;
    let requested =
        read_recorded_independent_participation(&snapshot, &installed.participation, &mut grant)?
            .ok_or("actual Roncalli exact-day root")?;
    let plan = discover_current_html_results(&attachments, &mut grant)?;
    check!(eq; plan.group_count(), 1);
    check!(eq; plan.diagnostic_count(), 0);
    let proposal = plan.with_group(0, &mut grant, |linked, grant| {
        calendar::build_claim(&attachments, &linked, 0, &requested, grant)
    })?;
    let claim = proposal.claim();
    check!(eq; claim.day.as_str(), "2026-03-11");
    check!(eq; claim.sport, Sport::IndoorTrack);
    check!(eq; claim.category, Gender::Girls);
    check!(eq; claim.year.get(), 2025);
    check!(eq; claim.person.canonical_candidate, installed.athlete.id.cast());
    check!(eq; claim.calendar.scope.as_str(), "https://athletics.roncalli.org/girls-track-field/schedules/#event-3387-recap");
    check!(eq; claim.calendar.capture.as_str(), "940776905cca35e9abd104533c32734049e3b3127c248c9b2c51f191d9879e4b");
    let leaf = requested.calendar_original_body();
    check!(proposal.claim().evidence.as_slice().contains(&leaf.object));
    let context = proposal.borrowed_context(&mut grant)?;
    check!(context.iter().any(
        |held| matches!(held, AttachmentContextObjectV1::Retained(actual) if *actual == leaf)
    ));
    snapshot.verify_retained_capture_leaf_v1(leaf, &mut grant)?;
    Ok(proposal)
}
