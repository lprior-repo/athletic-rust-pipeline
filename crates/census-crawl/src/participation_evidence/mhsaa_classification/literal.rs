use super::invalid;
use census_store::{StoreError, StoreResult};
use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MhsaaOriginalSpanV1<'a> {
    raw: &'a str,
    span: Range<usize>,
}

impl<'a> MhsaaOriginalSpanV1<'a> {
    pub(super) fn new(html: &'a str, span: Range<usize>) -> StoreResult<Self> {
        let raw = html
            .get(span.clone())
            .ok_or_else(|| invalid("MHSAA original span is invalid"))?;
        Ok(Self { raw, span })
    }

    pub(super) fn one(&self, literal: &str) -> StoreResult<Self> {
        let mut positions = self.raw.match_indices(literal);
        let (offset, _) = positions
            .next()
            .ok_or_else(|| invalid("MHSAA literal field is absent"))?;
        if literal.is_empty() || positions.next().is_some() {
            return Err(invalid("MHSAA literal field is ambiguous"));
        }
        let start = self
            .span
            .start
            .checked_add(offset)
            .ok_or(StoreError::CounterOverflow)?;
        let end = start
            .checked_add(literal.len())
            .ok_or(StoreError::CounterOverflow)?;
        let raw = self
            .raw
            .get(
                offset
                    ..offset
                        .checked_add(literal.len())
                        .ok_or(StoreError::CounterOverflow)?,
            )
            .ok_or_else(|| invalid("MHSAA field span is invalid"))?;
        Ok(Self {
            raw,
            span: start..end,
        })
    }

    pub fn raw(&self) -> &'a str {
        self.raw
    }
    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
    pub fn byte_offset(&self) -> usize {
        self.span.start
    }
    pub fn byte_length(&self) -> usize {
        self.raw.len()
    }
}
