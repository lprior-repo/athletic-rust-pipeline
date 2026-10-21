use super::{bound, hash, invalid, markup, MAX_BODY};
use census_store::{StoreError, StoreResult};
use std::ops::Range;

mod calendar;
mod status;
mod types;
pub use types::{
    CheckedRoncalliEventCandidateV1, RoncalliEventFactV1, RoncalliEventSelectorV1,
    RoncalliOriginalV1, RoncalliStatusFactV1,
};

pub fn read_roncalli_event_candidate_v1(
    review: RoncalliOriginalV1<'_>,
    schedule: RoncalliOriginalV1<'_>,
    selector: RoncalliEventSelectorV1<'_>,
) -> StoreResult<CheckedRoncalliEventCandidateV1> {
    original(review, "/girls-track-field/")?;
    original(schedule, "/girls-track-field/schedules/")?;
    let status = status::read(review.body, selector.status_subject_span)?;
    let event = calendar::read(schedule.body, selector.calendar_recap_id, &status.name)?;
    if status.season_year != event.season_year {
        return Err(invalid("Roncalli status and calendar editions differ"));
    }
    Ok(CheckedRoncalliEventCandidateV1 {
        status,
        event,
        status_body_sha256: hash(review.body.as_bytes()),
        schedule_body_sha256: hash(schedule.body.as_bytes()),
    })
}

fn original(value: RoncalliOriginalV1<'_>, path: &str) -> StoreResult<()> {
    bound(value.body.len(), MAX_BODY, "Roncalli original body bytes")?;
    let url = url::Url::parse(value.requested_url)
        .map_err(|_| invalid("Roncalli original URL invalid"))?;
    if url.scheme() != "https"
        || url.host_str() != Some("athletics.roncalli.org")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.path() != path
    {
        return Err(invalid("Roncalli original issuer or path differs"));
    }
    markup::canonical(value.body, value.requested_url)?;
    let title = markup::single(markup::elements(value.body, b"title", None)?)?;
    if !markup::text(title.html)?.ends_with(" - Roncalli High School Athletics") {
        return Err(invalid("Roncalli original school title differs"));
    }
    Ok(())
}

fn year(value: &str) -> StoreResult<u16> {
    if value.len() != 4 || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(invalid("Roncalli season year unsupported"));
    }
    value
        .parse()
        .map_err(|_| invalid("Roncalli season year invalid"))
}

fn shifted(span: Range<usize>, base: usize) -> StoreResult<Range<usize>> {
    Ok(span
        .start
        .checked_add(base)
        .ok_or(StoreError::CounterOverflow)?
        ..span
            .end
            .checked_add(base)
            .ok_or(StoreError::CounterOverflow)?)
}

#[cfg(test)]
mod tests;
