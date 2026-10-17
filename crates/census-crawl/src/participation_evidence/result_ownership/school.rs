use super::*;
use std::ops::Range;

const INTERPRETATION: &str = "retained-milesplit-original-school-statement/v1";
const MAX_SCHOOL_BYTES: usize = 4096;

#[derive(Debug)]
pub enum ResultSchoolStatementV1<'a> {
    Available(CheckedResultSchoolStatementV1<'a>),
    Unavailable,
}

#[derive(Debug)]
pub struct CheckedResultSchoolStatementV1<'a> {
    start: usize,
    end: usize,
    text: &'a str,
    digest: String,
    body_digest: &'a str,
    row: SourceRowLocator,
    row_digest: &'a str,
    row_reference: &'a str,
    context_reference: &'a str,
    reference: String,
}

impl CheckedResultSchoolStatementV1<'_> {
    pub fn interpretation(&self) -> &'static str {
        INTERPRETATION
    }
    pub fn span(&self) -> Range<usize> {
        self.start..self.end
    }
    pub fn text(&self) -> &str {
        self.text
    }
    pub fn digest(&self) -> &str {
        &self.digest
    }
    pub fn body_digest(&self) -> &str {
        self.body_digest
    }
    pub fn row(&self) -> SourceRowLocator {
        self.row
    }
    pub fn row_digest(&self) -> &str {
        self.row_digest
    }
    pub fn row_reference(&self) -> &str {
        self.row_reference
    }
    pub fn context_reference(&self) -> &str {
        self.context_reference
    }
    pub fn reference(&self) -> &str {
        &self.reference
    }
}

pub(super) fn read<'a>(
    row: &CheckedRetainedResultRowV1<'a>,
    span: Option<(usize, usize)>,
    capture: &'a RetainedHtmlCaptureV1,
) -> Result<ResultSchoolStatementV1<'a>> {
    let Some((start, end)) = span else {
        return Ok(ResultSchoolStatementV1::Unavailable);
    };
    let Some(length) = end
        .checked_sub(start)
        .filter(|size| (1..=MAX_SCHOOL_BYTES).contains(size))
    else {
        return Ok(ResultSchoolStatementV1::Unavailable);
    };
    let locator = row.locator();
    let relative =
        start
            .checked_sub(locator.byte_offset)
            .ok_or(RetainedResultOwnershipError::Invalid(
                "school statement precedes original row",
            ))?;
    let relative_end = relative
        .checked_add(length)
        .ok_or(RetainedResultOwnershipError::Overflow)?;
    let text = row.original_row()?.get(relative..relative_end).ok_or(
        RetainedResultOwnershipError::Invalid("school statement exceeds original row"),
    )?;
    if text != row.parsed_row().school {
        return Err(RetainedResultOwnershipError::Invalid(
            "school statement differs from parsed cell",
        ));
    }
    let digest = super::super::hash(text.as_bytes());
    let reference = serialized_digest(&(
        INTERPRETATION,
        capture.body_sha256(),
        row.context_reference(),
        row.reference(),
        row.original_row_digest(),
        start,
        end,
        &digest,
    ))?;
    Ok(ResultSchoolStatementV1::Available(
        CheckedResultSchoolStatementV1 {
            start,
            end,
            text,
            digest,
            body_digest: capture.body_sha256(),
            row: locator,
            row_digest: row.original_row_digest(),
            row_reference: row.reference(),
            context_reference: row.context_reference(),
            reference,
        },
    ))
}

#[cfg(test)]
mod tests;
