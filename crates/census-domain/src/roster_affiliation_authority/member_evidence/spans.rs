use super::{Result, SourceRowLocator};
use crate::roster_affiliation_authority::{LocallyCheckedRosterIssuanceV1, RosterAuthorityError};
use serde_json::value::RawValue;
use std::ops::Range;

pub(super) fn verify(
    section: &RawValue,
    container: &RawValue,
    heading: &RawValue,
    locator: SourceRowLocator,
    original: &LocallyCheckedRosterIssuanceV1,
) -> Result<()> {
    let body = usize::try_from(original.capture().body().bytes().get())
        .map_err(|_| RosterAuthorityError::Overflow)?;
    let section = range(section, body)?;
    let container = range(container, body)?;
    let heading = range(heading, body)?;
    let end = locator
        .byte_offset()
        .checked_add(locator.byte_length())
        .ok_or(RosterAuthorityError::Overflow)?;
    if locator.byte_length() == 0
        || end > body
        || section.start > container.start
        || container.end > section.end
        || section.start > heading.start
        || heading.end > container.start
        || locator.byte_offset() < container.start
        || end > container.end
    {
        return Err(RosterAuthorityError::Invalid("preimage span bounds"));
    }
    Ok(())
}

fn range(raw: &RawValue, body: usize) -> Result<Range<usize>> {
    let range: Range<usize> = serde_json::from_str(raw.get())?;
    if range.start >= range.end || range.end > body {
        return Err(RosterAuthorityError::Invalid("preimage span bounds"));
    }
    Ok(range)
}
