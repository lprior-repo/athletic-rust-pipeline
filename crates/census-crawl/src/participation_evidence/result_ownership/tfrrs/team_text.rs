use super::CheckedRetainedTfrrsRowV1;
use crate::participation_evidence::result_ownership::RetainedResultOwnershipError;
use html5gum::{DefaultEmitter, Token, Tokenizer};
use std::ops::Range;

const MAX_ANCHOR_BYTES: usize = 8192;
const MAX_QUOTE_BYTES: usize = 512;

#[derive(Debug, thiserror::Error)]
pub enum TfrrsTeamTextError {
    #[error("original TFRRS team anchor is outside its checked row")]
    OutsideCheckedRow,
    #[error("original TFRRS team anchor has ambiguous or transformed markup")]
    AmbiguousMarkup,
    #[error("original TFRRS team text differs from the authenticated team name")]
    TransformedText,
    #[error("original TFRRS team quote capacity: needed {needed}, available {available}")]
    Capacity { needed: usize, available: usize },
    #[error("original TFRRS team quote span arithmetic overflow")]
    Overflow,
    #[error("original TFRRS team row: {0}")]
    Ownership(#[source] Box<RetainedResultOwnershipError>),
}

#[derive(Debug, PartialEq, Eq)]
pub struct CheckedOriginalTfrrsTeamTextV1<'a> {
    text: &'a str,
    span: Range<usize>,
}

impl<'a> CheckedOriginalTfrrsTeamTextV1<'a> {
    pub fn text(&self) -> &'a str {
        self.text
    }

    pub fn span(&self) -> Range<usize> {
        self.span.clone()
    }
}

pub fn read_original_team_text<'a>(
    row: &CheckedRetainedTfrrsRowV1<'a>,
) -> Result<CheckedOriginalTfrrsTeamTextV1<'a>, TfrrsTeamTextError> {
    let original = row
        .original_row()
        .map_err(|error| TfrrsTeamTextError::Ownership(Box::new(error)))?;
    let anchor = row.team_span();
    let offset = row.locator().byte_offset;
    let start = anchor
        .start
        .checked_sub(offset)
        .ok_or(TfrrsTeamTextError::OutsideCheckedRow)?;
    let end = anchor
        .end
        .checked_sub(offset)
        .ok_or(TfrrsTeamTextError::OutsideCheckedRow)?;
    let html = original
        .get(start..end)
        .ok_or(TfrrsTeamTextError::OutsideCheckedRow)?;
    bound(html.len(), MAX_ANCHOR_BYTES)?;
    let relative = literal_span(html, row.team_name())?;
    let text = html
        .get(relative.clone())
        .ok_or(TfrrsTeamTextError::AmbiguousMarkup)?;
    let start = anchor
        .start
        .checked_add(relative.start)
        .ok_or(TfrrsTeamTextError::Overflow)?;
    let end = anchor
        .start
        .checked_add(relative.end)
        .ok_or(TfrrsTeamTextError::Overflow)?;
    Ok(CheckedOriginalTfrrsTeamTextV1 {
        text,
        span: start..end,
    })
}

fn literal_span(html: &str, authenticated: &str) -> Result<Range<usize>, TfrrsTeamTextError> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut tokens = Tokenizer::new_with_emitter(html, emitter);
    let Token::StartTag(start) = tokens
        .next()
        .ok_or(TfrrsTeamTextError::AmbiguousMarkup)?
        .map_err(|_| TfrrsTeamTextError::AmbiguousMarkup)?
    else {
        return Err(TfrrsTeamTextError::AmbiguousMarkup);
    };
    let Token::String(value) = tokens
        .next()
        .ok_or(TfrrsTeamTextError::AmbiguousMarkup)?
        .map_err(|_| TfrrsTeamTextError::AmbiguousMarkup)?
    else {
        return Err(TfrrsTeamTextError::AmbiguousMarkup);
    };
    let Token::EndTag(end) = tokens
        .next()
        .ok_or(TfrrsTeamTextError::AmbiguousMarkup)?
        .map_err(|_| TfrrsTeamTextError::AmbiguousMarkup)?
    else {
        return Err(TfrrsTeamTextError::AmbiguousMarkup);
    };
    if start.name.as_slice() != b"a"
        || start.self_closing
        || start.span.start != 0
        || end.name.as_slice() != b"a"
        || end.span.end != html.len()
        || tokens.next().is_some()
    {
        return Err(TfrrsTeamTextError::AmbiguousMarkup);
    }
    let span = start.span.end..end.span.start;
    let raw = html
        .get(span.clone())
        .ok_or(TfrrsTeamTextError::AmbiguousMarkup)?;
    bound(raw.len(), MAX_QUOTE_BYTES)?;
    if raw.is_empty() || raw != authenticated || raw.as_bytes() != value.as_slice() {
        return Err(TfrrsTeamTextError::TransformedText);
    }
    Ok(span)
}

fn bound(needed: usize, available: usize) -> Result<(), TfrrsTeamTextError> {
    if needed > available {
        return Err(TfrrsTeamTextError::Capacity { needed, available });
    }
    Ok(())
}

#[cfg(test)]
mod tests;
