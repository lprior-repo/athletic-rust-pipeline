use super::*;
use crate::{CrawlError, CrawlResult};
use types::Selected;

pub(super) fn admit_count(needed: usize) -> Result<()> {
    if needed > MAX_PARTICIPATION_FACTS {
        return Err(RetainedResultOwnershipError::Capacity {
            needed,
            available: MAX_PARTICIPATION_FACTS,
        });
    }
    if needed == 0 {
        return Err(RetainedResultOwnershipError::Invalid(
            "empty requested subject set",
        ));
    }
    needed
        .checked_mul(std::mem::size_of::<Selected>())
        .ok_or(RetainedResultOwnershipError::Overflow)?;
    Ok(())
}

pub(super) fn validate_spans(
    body: &str,
    requested: &[SourceRowLocator],
    owned_pre: &std::ops::Range<usize>,
) -> Result<()> {
    let mut previous_end = 0usize;
    let mut previous_ordinal = 0u32;
    for row in requested {
        let end = row
            .byte_offset
            .checked_add(row.byte_length)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        if row.ordinal <= previous_ordinal
            || row.byte_length == 0
            || row.byte_offset < previous_end
            || row.byte_offset < owned_pre.start
            || end > owned_pre.end
            || body.get(row.byte_offset..end).is_none()
        {
            return Err(RetainedResultOwnershipError::Invalid(
                "requested locators are invalid, duplicate or unordered",
            ));
        }
        previous_end = end;
        previous_ordinal = row.ordinal;
    }
    Ok(())
}

pub(super) fn native_document(value: &str) -> Result<(String, String)> {
    let url =
        url::Url::parse(value).map_err(|_| RetainedResultOwnershipError::Invalid("result URL"))?;
    let host = url
        .host_str()
        .ok_or(RetainedResultOwnershipError::Invalid("result host"))?;
    if url.scheme() != "https"
        || !(host == "milesplit.com" || host.ends_with(".milesplit.com"))
        || url.query().is_some()
        || url.fragment().is_some()
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(RetainedResultOwnershipError::Invalid(
            "result URL ownership",
        ));
    }
    let mut parts = url
        .path_segments()
        .ok_or(RetainedResultOwnershipError::Invalid("result path"))?;
    if parts.next() != Some("meets") {
        return Err(RetainedResultOwnershipError::Invalid("result meet path"));
    }
    let meet = parts
        .next()
        .ok_or(RetainedResultOwnershipError::Invalid("result meet ID"))?;
    let meet = meet
        .split('-')
        .next()
        .ok_or(RetainedResultOwnershipError::Invalid("result meet ID"))?;
    if parts.next() != Some("results") {
        return Err(RetainedResultOwnershipError::Invalid(
            "result document path",
        ));
    }
    let document = parts
        .next()
        .ok_or(RetainedResultOwnershipError::Invalid("result document ID"))?;
    if parts.next() != Some("raw")
        || parts.next().is_some()
        || !native_id(meet)
        || !native_id(document)
    {
        return Err(RetainedResultOwnershipError::Invalid(
            "result native document identity",
        ));
    }
    Ok((meet.into(), document.into()))
}

fn native_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 32
        && !value.starts_with('0')
        && value.bytes().all(|byte| byte.is_ascii_digit())
}

pub(super) fn require_owner(expected: &str, found: &str, role: &str) -> CrawlResult<()> {
    if expected != found {
        return Err(CrawlError::Schema {
            url: expected.into(),
            detail: format!("{role} differs from retained exact document URL"),
        });
    }
    Ok(())
}

pub(super) fn finish(
    body: &str,
    capture: &RetainedHtmlCaptureV1,
    provisional: Vec<ProvisionalRawRowLocatorV1>,
) -> Result<Vec<Selected>> {
    let mut selected = Vec::new();
    selected
        .try_reserve_exact(provisional.len())
        .map_err(|_| RetainedResultOwnershipError::Allocation)?;
    for locator in provisional {
        let end = locator
            .row
            .byte_offset
            .checked_add(locator.row.byte_length)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        let original =
            body.get(locator.row.byte_offset..end)
                .ok_or(RetainedResultOwnershipError::Invalid(
                    "original result span",
                ))?;
        selected.push(Selected {
            locator,
            original_digest: super::super::hash(original.as_bytes()),
            reference: serialized_digest(&(
                INTERPRETATION,
                capture.requested_url(),
                capture.body_sha256(),
                locator.row,
            ))?,
        });
    }
    Ok(selected)
}
