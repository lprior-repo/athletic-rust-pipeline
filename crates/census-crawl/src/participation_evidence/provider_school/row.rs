use super::super::{bound, hash, invalid, markup};
use crate::milesplit::{SourceRowLocator, TeamRef};
use census_store::{StoreError, StoreResult};
use html5gum::{DefaultEmitter, Token, Tokenizer};
use std::ops::Range;

pub(super) struct Spans {
    pub(super) row: SourceRowLocator,
    pub(super) row_sha256: String,
    pub(super) name: SourceRowLocator,
    pub(super) city: SourceRowLocator,
    pub(super) anchor: SourceRowLocator,
}

struct Scan<'a> {
    html: &'a str,
    team: &'a TeamRef,
    active: Option<(usize, u32)>,
    rows: u32,
    selected: Option<Spans>,
}

pub(super) fn read(html: &str, team: &TeamRef) -> StoreResult<Spans> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut visibility = markup::Active::default();
    let mut scan = Scan {
        html,
        team,
        active: None,
        rows: 0,
        selected: None,
    };
    Tokenizer::new_with_emitter(html, emitter)
        .take(100_001)
        .enumerate()
        .try_for_each(|(index, token)| -> StoreResult<()> {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                100_000,
                "provider school directory tokens",
            )?;
            let token = token.map_err(|_| invalid("provider school directory token failure"))?;
            if visibility.accept(&token)? {
                scan.visit(token)?;
            }
            Ok(())
        })?;
    visibility.finish()?;
    if scan.active.is_some() {
        return Err(invalid("provider school directory row is unclosed"));
    }
    scan.selected
        .ok_or_else(|| invalid("provider school selected row lacks active provenance"))
}

impl Scan<'_> {
    fn visit(&mut self, token: Token<usize>) -> StoreResult<()> {
        match token {
            Token::StartTag(tag) if tag.name.as_slice() == b"tr" => {
                if self.active.is_some() {
                    return Err(invalid("provider school directory rows are nested"));
                }
                self.active = Some((tag.span.start, self.rows));
                self.rows = self
                    .rows
                    .checked_add(1)
                    .ok_or(StoreError::CounterOverflow)?;
            }
            Token::EndTag(tag) if tag.name.as_slice() == b"tr" => {
                let (start, ordinal) = self
                    .active
                    .take()
                    .ok_or_else(|| invalid("provider school unmatched row close"))?;
                let row = self
                    .html
                    .get(start..tag.span.end)
                    .ok_or_else(|| invalid("provider school directory span invalid"))?;
                if row.contains(&self.team.url) {
                    let spans = selected(row, start, ordinal, self.team)?;
                    if self.selected.replace(spans).is_some() {
                        return Err(invalid("provider school selected active row is ambiguous"));
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

fn selected(html: &str, start: usize, ordinal: u32, team: &TeamRef) -> StoreResult<Spans> {
    shape(html)?;
    let cells = markup::elements(html, b"td", None)?;
    let [name, city] = cells.as_slice() else {
        return Err(invalid("provider school directory selected columns differ"));
    };
    let anchor = markup::single(markup::elements(name.html, b"a", None)?)?;
    if anchor.attribute("href") != Some(team.url.as_str())
        || markup::text(name.html)? != team.name
        || markup::text(anchor.html)? != team.name
        || markup::text(city.html)? != team.city_state
    {
        return Err(invalid("provider school directory active fields differ"));
    }
    let name_start = start
        .checked_add(name.span.start)
        .ok_or(StoreError::CounterOverflow)?;
    Ok(Spans {
        row: locator(ordinal, start, 0..html.len())?,
        row_sha256: hash(html.as_bytes()),
        name: locator(ordinal, start, name.span.clone())?,
        city: locator(ordinal, start, city.span.clone())?,
        anchor: locator(ordinal, name_start, anchor.span.clone())?,
    })
}

fn locator(ordinal: u32, base: usize, span: Range<usize>) -> StoreResult<SourceRowLocator> {
    Ok(SourceRowLocator {
        ordinal,
        byte_offset: base
            .checked_add(span.start)
            .ok_or(StoreError::CounterOverflow)?,
        byte_length: span
            .end
            .checked_sub(span.start)
            .ok_or(StoreError::CounterOverflow)?,
    })
}

fn shape(html: &str) -> StoreResult<()> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut visibility = markup::Active::default();
    let (mut cells, mut anchors) = (0usize, 0usize);
    Tokenizer::new_with_emitter(html, emitter)
        .take(4097)
        .enumerate()
        .try_for_each(|(index, token)| -> StoreResult<()> {
            bound(
                index.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                4096,
                "provider school selected row tokens",
            )?;
            let token = token.map_err(|_| invalid("provider school selected row token failure"))?;
            if !visibility.accept(&token)? {
                return Ok(());
            }
            if let Token::StartTag(tag) = token {
                match tag.name.as_slice() {
                    b"td" => cells = cells.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                    b"a" => anchors = anchors.checked_add(1).ok_or(StoreError::CounterOverflow)?,
                    b"th" => {
                        return Err(invalid("provider school selected row has an extra column"))
                    }
                    _ => {}
                }
            }
            Ok(())
        })?;
    visibility.finish()?;
    if cells != 2 || anchors != 1 {
        return Err(invalid(
            "provider school selected row column or anchor shape differs",
        ));
    }
    Ok(())
}
