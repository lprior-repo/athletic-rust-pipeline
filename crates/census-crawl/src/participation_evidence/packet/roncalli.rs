use super::IndependentParticipationPacketV1;
use crate::participation_evidence::roncalli::{
    read_roncalli_event_candidate_v1, CheckedRoncalliEventCandidateV1, RoncalliEventSelectorV1,
    RoncalliOriginalV1,
};
use crate::participation_evidence::{
    hash, invalid, DocumentSubjectV1, IndependentParticipationProofV1, RetainedHtmlCaptureV1,
    SubjectOwnershipV1, VerifiedIndependentParticipationV1,
};
use census_domain::model::{GradYear, Grade, SchoolYear, StoreIdentity};
use census_domain::result_calendar_correspondence::{
    RoncalliSeasonEventProofContextV1, RoncalliSeasonEventSelectionV1,
};
use census_store::{StoreError, StoreResult};
use chrono::{Datelike, NaiveDate};

pub(super) fn read(
    packet: &IndependentParticipationPacketV1,
    selection: &RoncalliSeasonEventSelectionV1,
    store_identity: StoreIdentity,
    snapshot_sequence: u64,
) -> StoreResult<VerifiedIndependentParticipationV1> {
    let candidate = candidate(&packet.status, &packet.schedule, selection)?;
    let year = school_year(&candidate)?;
    let grade = Grade::new(candidate.status().grade)
        .ok_or_else(|| invalid("Roncalli explicit student grade unsupported"))?;
    let cohort = GradYear::of(grade, year)
        .ok_or_else(|| invalid("Roncalli explicit grade and event year unsupported"))?;
    let subject = subject(&packet.status, &candidate);
    let calendar_locator = locator(&candidate.event().event_span);
    let interpretation = hash(
        selection
            .interpretation_preimage(RoncalliSeasonEventProofContextV1 {
                status_body_sha: packet.status.body_sha256(),
                schedule_body_sha: packet.schedule.body_sha256(),
                status_locator: subject.locator(),
                calendar_locator: &calendar_locator,
                day: &candidate.event().day,
            })
            .as_bytes(),
    );
    let status = proof(&packet.status, subject.locator.clone(), &interpretation);
    let season = proof(&packet.schedule, calendar_locator, &interpretation);
    Ok(VerifiedIndependentParticipationV1 {
        store_identity,
        snapshot_sequence,
        reference: packet.reference()?,
        ownership: SubjectOwnershipV1::DocumentOnly(subject.clone()),
        subject,
        day: candidate.event().day.clone(),
        school_year: year,
        cohort,
        competition: candidate.event().competition.clone(),
        venue: candidate.event().venue.clone(),
        season_scope: format!(
            "{}#{}",
            packet.schedule.requested_url(),
            candidate.event().recap_id
        ),
        status,
        season,
        roncalli_event: Some(candidate),
    })
}

fn candidate(
    status: &RetainedHtmlCaptureV1,
    schedule: &RetainedHtmlCaptureV1,
    selection: &RoncalliSeasonEventSelectionV1,
) -> StoreResult<CheckedRoncalliEventCandidateV1> {
    if !selection.is_bounded() {
        return Err(invalid("Roncalli source selection exceeds bounds"));
    }
    let status_body = status.body()?;
    let schedule_body = schedule.body()?;
    let candidate = read_roncalli_event_candidate_v1(
        RoncalliOriginalV1 {
            requested_url: status.requested_url(),
            body: &status_body,
        },
        RoncalliOriginalV1 {
            requested_url: schedule.requested_url(),
            body: &schedule_body,
        },
        RoncalliEventSelectorV1 {
            status_subject_span: selection.status_subject_start..selection.status_subject_end,
            calendar_recap_id: &selection.calendar_recap_id,
        },
    )?;
    if candidate.status_body_sha256() != status.body_sha256()
        || candidate.schedule_body_sha256() != schedule.body_sha256()
    {
        return Err(invalid("Roncalli interpreted original body hash differs"));
    }
    Ok(candidate)
}

fn school_year(candidate: &CheckedRoncalliEventCandidateV1) -> StoreResult<SchoolYear> {
    let event = candidate.event();
    let date = NaiveDate::parse_from_str(event.day.as_str(), "%Y-%m-%d")
        .map_err(|_| invalid("Roncalli event day unsupported"))?;
    let year = SchoolYear::from_date(event.day.as_str())
        .ok_or_else(|| invalid("Roncalli canonical school year unsupported"))?;
    let closing = year
        .get()
        .checked_add(1)
        .ok_or(StoreError::CounterOverflow)?;
    if candidate.status().season_year != event.season_year
        || i32::from(event.season_year) != date.year()
        || i32::from(closing) != date.year()
    {
        return Err(invalid(
            "Roncalli season edition and spring event year differ",
        ));
    }
    Ok(year)
}

fn subject(
    capture: &RetainedHtmlCaptureV1,
    candidate: &CheckedRoncalliEventCandidateV1,
) -> DocumentSubjectV1 {
    let status = candidate.status();
    DocumentSubjectV1 {
        document_namespace: "roncalli_girls_track_season_review_v1".into(),
        native_document_id: format!(
            "girls-track-field:{}:{}:{}",
            status.season_year, status.subject_span.start, status.subject_span.end
        ),
        document_url: capture.requested_url().into(),
        body_sha256: capture.body_sha256().into(),
        locator: locator(&status.subject_span),
        name: status.name.clone(),
        school: "Roncalli".into(),
    }
}

fn locator(span: &std::ops::Range<usize>) -> String {
    format!("html_bytes:{}:{}", span.start, span.end)
}

fn proof(
    capture: &RetainedHtmlCaptureV1,
    locator: String,
    interpretation: &str,
) -> IndependentParticipationProofV1 {
    IndependentParticipationProofV1 {
        capture: capture.body_sha256().into(),
        interpretation: interpretation.into(),
        locator,
        acquired_at: capture.acquired_at(),
    }
}

#[cfg(test)]
pub(in crate::participation_evidence) mod tests;
