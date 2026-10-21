use super::{CheckedRetainedTfrrsListV1, CheckedRetainedTfrrsRowV1, Error, Result};
use crate::participation_evidence::physical_result::{
    admission::{bound, canonical_id, count, digest, text},
    ProposedHtmlResultTargetV1,
};
use crate::participation_evidence::{codec, MAX_PACKET};
use census_domain::model::{AthleteId, EventKind, ProfileDate, SchoolYear, Sport};
use serde::Serialize;

pub(super) fn single(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    target: &ProposedHtmlResultTargetV1<'_>,
) -> Result<()> {
    for reference in [
        target.subject_reference,
        target.proposed_attachment_reference,
        target.proposed_school_binding_reference,
    ] {
        digest(reference)?;
    }
    canonical_id(target.proposed_athlete.as_str(), "ath_subject_")?;
    canonical_id(target.proposed_school.as_str(), "sch_")?;
    if target.locator != row.locator() || target.subject_reference != row.subject()?.reference() {
        return Err(Error::Invalid(
            "target differs from exact checked TFRRS row subject",
        ));
    }
    payload(list, row, target)
}

pub(super) fn identity(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    athlete: &AthleteId,
) -> Result<()> {
    canonical_id(athlete.as_str(), "ath_subject_")?;
    payload(list, row, athlete)
}

fn payload(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
    proposal: &impl Serialize,
) -> Result<()> {
    source(list, row)?;
    let native = (
        list.list_id(),
        row.athlete_id(),
        row.athlete_path(),
        row.athlete_name(),
        row.team_path(),
        row.team_name(),
        row.meet_id(),
        row.meet_path(),
        row.meet_name(),
        row.performance_id(),
        row.performance_path(),
    );
    let context = (
        list.capture().requested_url(),
        list.capture().body_sha256(),
        list.context_reference(),
        row.context_reference(),
        row.reference(),
        row.original_row_digest(),
        row.locator(),
        row.published_date(),
        row.event_label(),
        row.event_handle(),
        row.gender(),
        row.grade(),
        list.sport(),
    );
    let marks = (
        row.published_mark(),
        row.original_mark(),
        row.conversion_note(),
    );
    let needed = codec::encode(&(proposal, native, context, marks), 4096)?
        .len()
        .checked_mul(16)
        .and_then(|bytes| bytes.checked_add(8192))
        .and_then(|bytes| bytes.checked_add(16 * 1024))
        .ok_or(Error::Overflow)?;
    bound(needed, MAX_PACKET, "estimated TFRRS encoded output bytes")
}

fn source(
    list: &CheckedRetainedTfrrsListV1<'_>,
    row: &CheckedRetainedTfrrsRowV1<'_>,
) -> Result<()> {
    count(list.selected_count())?;
    text(list.capture().requested_url(), 2048)?;
    for value in [
        row.athlete_name(),
        row.team_name(),
        row.meet_name(),
        row.event_label(),
    ] {
        text(value, 512)?;
    }
    for reference in [
        list.context_reference(),
        list.capture().body_sha256(),
        row.context_reference(),
        row.reference(),
        row.original_row_digest(),
    ] {
        digest(reference)?;
    }
    ProfileDate::parse(row.published_date())
        .map_err(|_| Error::Invalid("TFRRS result requires an exact source day"))?;
    if SchoolYear::from_date(row.published_date()).is_none()
        || !matches!(list.sport(), Sport::IndoorTrack | Sport::OutdoorTrack)
    {
        return Err(Error::Invalid(
            "unsupported TFRRS source date or sport scope",
        ));
    }
    kind(row)?;
    Ok(())
}

pub(super) fn kind(row: &CheckedRetainedTfrrsRowV1<'_>) -> Result<EventKind> {
    let kind = EventKind::from_source_label(row.event_label());
    if kind.is_relay()
        || matches!(
            kind,
            EventKind::Pentathlon
                | EventKind::Heptathlon
                | EventKind::Decathlon
                | EventKind::Unmapped { .. }
        )
    {
        return Err(Error::Invalid(
            "unsupported TFRRS relay, combined or unmapped event",
        ));
    }
    Ok(kind)
}
