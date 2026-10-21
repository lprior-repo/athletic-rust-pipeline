use super::{bounded, invalid, native_links, Result, RetainedResultOwnershipError};
use crate::participation_evidence::markup;
use crate::tfrrs::{
    clock_seconds, converted_from, feet_inches_metres, is_track_size_conversion, metric_metres,
    ParsedMark,
};
use html5gum::{DefaultEmitter, Token, Tokenizer};
use std::ops::Range;

pub(super) struct Mark {
    pub(super) published: String,
    pub(super) span: Range<usize>,
    pub(super) note: Option<String>,
    pub(super) note_span: Option<Range<usize>>,
    pub(super) original: Option<String>,
}

pub(super) fn read(
    body: &str,
    cell: &super::cells::Cell,
    link: &native_links::Link,
) -> Result<Mark> {
    let html = body
        .get(cell.span.clone())
        .ok_or_else(|| invalid("TFRRS mark cell span"))?;
    let is_time = cell.label == "Time";
    let valid = if is_time {
        clock_seconds(&link.text).is_some()
    } else {
        feet_inches_metres(&link.text).is_some() || metric_metres(&link.text).is_some()
    };
    if !valid {
        return Err(invalid("unsupported TFRRS published mark"));
    }
    let note = note(html, cell.span.start, &link.span)?;
    let whole = markup::text(html)?;
    match note {
        None => {
            if whole != link.text {
                return Err(invalid("ambiguous unconverted TFRRS mark"));
            }
            Ok(Mark {
                published: link.text.clone(),
                span: link.span.clone(),
                note: None,
                note_span: None,
                original: None,
            })
        }
        Some((note, span)) => converted(link, is_time, &whole, note, span),
    }
}

fn converted(
    link: &native_links::Link,
    is_time: bool,
    whole: &str,
    note: String,
    span: Range<usize>,
) -> Result<Mark> {
    if !is_time || whole != format!("{} #", link.text) || !is_track_size_conversion(&note) {
        return Err(invalid("unsupported TFRRS conversion"));
    }
    let Some(ParsedMark::Time(original)) = converted_from(&note) else {
        return Err(invalid("TFRRS original conversion mark"));
    };
    if note.split_whitespace().collect::<Vec<_>>().join(" ")
        != format!("Converted from {original} for Track Size.")
    {
        return Err(invalid("unsupported TFRRS conversion note grammar"));
    }
    Ok(Mark {
        published: link.text.clone(),
        span: link.span.clone(),
        note: Some(note),
        note_span: Some(span),
        original: Some(original),
    })
}

fn note(html: &str, offset: usize, mark: &Range<usize>) -> Result<Option<(String, Range<usize>)>> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut note = None;
    for (index, token) in Tokenizer::new_with_emitter(html, emitter).enumerate() {
        bounded(
            index
                .checked_add(1)
                .ok_or(RetainedResultOwnershipError::Overflow)?,
            4096,
        )?;
        if let Token::StartTag(tag) = token.map_err(|_| invalid("TFRRS conversion tokenization"))? {
            record_title(&mut note, &tag, offset)?;
        }
    }
    if note.is_some() {
        let wrappers = markup::elements(html, b"span", None)?;
        let owned = wrappers.iter().any(|wrapper| {
            wrapper.attribute("title").is_some()
                && offset
                    .checked_add(wrapper.span.start)
                    .is_some_and(|start| start < mark.start)
                && offset
                    .checked_add(wrapper.span.end)
                    .is_some_and(|end| end > mark.end)
        });
        if !owned {
            return Err(invalid(
                "TFRRS conversion note does not own the published mark",
            ));
        }
    }
    Ok(note)
}

fn record_title(
    note: &mut Option<(String, Range<usize>)>,
    tag: &html5gum::StartTag<usize>,
    offset: usize,
) -> Result<()> {
    if let Some(title) = tag.attributes.get(b"title".as_slice()) {
        if note.is_some() || tag.name.as_slice() != b"span" {
            return Err(invalid("ambiguous TFRRS conversion note"));
        }
        bounded(title.len(), 256)?;
        let value = std::str::from_utf8(title)
            .map_err(|_| invalid("TFRRS conversion encoding"))?
            .trim()
            .to_owned();
        let start = offset
            .checked_add(title.span.start)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        let end = offset
            .checked_add(title.span.end)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        *note = Some((value, start..end));
    }
    Ok(())
}
