use super::super::{MAX_ROWS, MAX_ROW_BYTES};
use super::{Gap, TemporalResult};
use html5gum::{DefaultEmitter, Span, Token, Tokenizer};
use sha2::{Digest, Sha256};
use std::ops::Range;

mod active;
mod close;
mod open;

pub(super) const MAX_DEPTH: usize = 256;
const MAX_SCOPES: usize = 4096;
pub(super) const MAX_HEADING_BYTES: usize = 4096;

struct Frame {
    name: Vec<u8>,
    span: Span<usize>,
    inert: bool,
    section: Option<usize>,
    roster: Option<usize>,
    heading: Vec<u8>,
    row: Option<usize>,
}

pub(super) struct Heading {
    pub(super) span: Range<usize>,
    pub(super) sha256: [u8; 32],
}

#[derive(Default)]
pub(super) struct Scope {
    pub(super) section: Range<usize>,
    pub(super) heading_count: usize,
    pub(super) heading: Option<Heading>,
    pub(super) container_count: usize,
    pub(super) container: Range<usize>,
}

pub(super) struct Row {
    pub(super) scope: usize,
    pub(super) span: Range<usize>,
    pub(super) active: String,
}

#[derive(Default)]
pub(super) struct Scan {
    stack: Vec<Frame>,
    pub(super) scopes: Vec<Scope>,
    pub(super) rows: Vec<Row>,
    active_row: Option<usize>,
    #[cfg(test)]
    pub(super) heading_hashes: usize,
    #[cfg(test)]
    pub(super) section_updates: usize,
}

pub(super) fn scan(html: &str) -> TemporalResult<Scan> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut scan = Scan::default();
    for token in Tokenizer::new_with_emitter(html, emitter) {
        match token.map_err(|_| Gap::MalformedMarkup)? {
            Token::StartTag(tag) => scan.open(html, tag)?,
            Token::EndTag(tag) => scan.close(html, &tag.name, tag.span)?,
            Token::String(value) => scan.text(html, &value, value.span)?,
            Token::Error(_) => return Err(Gap::MalformedMarkup),
            Token::Comment(_) | Token::Doctype(_) => {}
        }
    }
    Ok(scan)
}

fn reserve<T>(rows: &mut Vec<T>, limit: usize) -> TemporalResult<()> {
    if rows.len() >= limit {
        return Err(Gap::ResourceLimit);
    }
    rows.try_reserve(1).map_err(|_| Gap::ResourceLimit)
}
