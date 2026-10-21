use super::*;
use crate::evidence_subject_attachment::external::CheckedCapture;
use census_domain::result_calendar_correspondence::{
    RoncalliSeasonEventProofContextV1, RoncalliSeasonEventSelectionV1,
};

pub(super) fn require(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    selection: &RoncalliSeasonEventSelectionV1,
    captures: (&CheckedCapture<'_>, &CheckedCapture<'_>),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    if !selection.is_bounded()
        || subject.namespace.as_str() != "roncalli_girls_track_season_review_v1"
        || captures.0.body_sha256 != subject.body.as_str()
        || captures.1.body_sha256 != source.calendar.capture.as_str()
    {
        return Err(Error::SourceMismatch("Roncalli documentary source binding").into());
    }
    let status = retained(materials, &subject.body)?;
    let calendar = retained(materials, &source.calendar.capture)?;
    let status_body = captures.0.body_bytes(&status.object, grant)?;
    let calendar_body = captures.1.body_bytes(&calendar.object, grant)?;
    let status_locator = inline(materials, &subject.locator, 96, grant)?;
    let calendar_locator = inline(materials, &source.calendar.locator, 96, grant)?;
    let scanned = status_body
        .len()
        .checked_add(calendar_body.len())
        .ok_or(Error::BudgetExceeded)?;
    grant
        .charge_steps(scanned.div_ceil(512))
        .map_err(grant_error)?;
    locators(
        selection,
        (&status.object, &status_body),
        &status_locator,
        (&calendar.object, &calendar_body),
        &calendar_locator,
    )?;
    let schedule_url = captures.1.requested_url(grant)?;
    grant
        .charge_bytes(
            schedule_url
                .len()
                .checked_add(64)
                .ok_or(Error::BudgetExceeded)?,
        )
        .map_err(grant_error)?;
    let scope = format!("{}#{}", schedule_url, selection.calendar_recap_id);
    if source.calendar.scope.as_str() != scope {
        return Err(Error::SourceMismatch("Roncalli calendar scope").into());
    }
    proof(
        source,
        materials,
        subject,
        selection,
        (&status_locator, &calendar_locator),
        grant,
    )
}

fn locators(
    selection: &RoncalliSeasonEventSelectionV1,
    status: (&AttachmentObjectReferenceV1, &[u8]),
    status_locator: &[u8],
    calendar: (&AttachmentObjectReferenceV1, &[u8]),
    calendar_locator: &[u8],
) -> StoreResult<()> {
    let (start, end) = locator_range(status.0, status_locator)?;
    if usize::try_from(start).ok() != Some(selection.status_subject_start)
        || usize::try_from(end).ok() != Some(selection.status_subject_end)
    {
        return Err(Error::SourceMismatch("Roncalli selected status locator").into());
    }
    exact_span(status.1, status_locator, (start, end))?;
    exact_span(
        calendar.1,
        calendar_locator,
        locator_range(calendar.0, calendar_locator)?,
    )
}

fn exact_span(body: &[u8], locator: &[u8], range: (u64, u64)) -> StoreResult<()> {
    let (start, end) = range;
    let raw = std::str::from_utf8(locator)
        .map_err(|_| Error::SourceMismatch("Roncalli original locator UTF-8"))?;
    let (left, right) = raw
        .strip_prefix("html_bytes:")
        .and_then(|raw| raw.split_once(':'))
        .ok_or(Error::SourceMismatch("Roncalli original locator shape"))?;
    let canonical = [left, right].into_iter().all(|digits| {
        !digits.is_empty()
            && (digits.len() == 1 || !digits.starts_with('0'))
            && digits.bytes().all(|byte| byte.is_ascii_digit())
    });
    let text = std::str::from_utf8(body)
        .map_err(|_| Error::SourceMismatch("Roncalli original body UTF-8"))?;
    let start = usize::try_from(start).map_err(|_| Error::BudgetExceeded)?;
    let end = usize::try_from(end).map_err(|_| Error::BudgetExceeded)?;
    if !canonical
        || left.parse::<usize>().ok() != Some(start)
        || right.parse::<usize>().ok() != Some(end)
        || text.get(start..end).is_none()
    {
        return Err(Error::SourceMismatch("Roncalli original locator span").into());
    }
    Ok(())
}

fn proof(
    source: &ResultCalendarSourceClaimV1,
    materials: &[Material],
    subject: &RetainedDocumentSubjectReferenceV1,
    selection: &RoncalliSeasonEventSelectionV1,
    locators: (&[u8], &[u8]),
    grant: &mut AttachmentReplayGrantV1,
) -> StoreResult<()> {
    let status_locator = std::str::from_utf8(locators.0)
        .map_err(|_| Error::SourceMismatch("Roncalli status locator UTF-8"))?;
    let calendar_locator = std::str::from_utf8(locators.1)
        .map_err(|_| Error::SourceMismatch("Roncalli calendar locator UTF-8"))?;
    grant.charge_bytes(512).map_err(grant_error)?;
    grant.charge_steps(1).map_err(grant_error)?;
    let expected = selection.interpretation_preimage(RoncalliSeasonEventProofContextV1 {
        status_body_sha: subject.body.as_str(),
        schedule_body_sha: source.calendar.capture.as_str(),
        status_locator,
        calendar_locator,
        day: &source.day,
    });
    if expected.len() > 512 || subject.interpretation != source.calendar.interpretation {
        return Err(Error::SourceMismatch("Roncalli shared interpretation binding").into());
    }
    let status = inline(materials, &subject.interpretation, 512, grant)?;
    let season = inline(materials, &source.calendar.interpretation, 512, grant)?;
    if status != expected.as_bytes() || season != expected.as_bytes() {
        return Err(Error::SourceMismatch("Roncalli exact interpretation preimage").into());
    }
    Ok(())
}

#[cfg(test)]
mod tests;
