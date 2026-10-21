use super::{bounded, invalid, Context, Gender, Result};
use crate::participation_evidence::markup;
use html5gum::StartTag;
use std::ops::Range;

pub(super) struct Section {
    pub(super) depth: usize,
    pub(super) start: usize,
    pub(super) header: Option<(String, Range<usize>)>,
    pub(super) open_header: Option<usize>,
    pub(super) body_depth: Option<usize>,
    pub(super) body_seen: bool,
    handle: u32,
    gender: Gender,
}

pub(super) fn classes(tag: &StartTag<usize>) -> Result<&str> {
    let Some(raw) = tag.attributes.get(b"class".as_slice()) else {
        return Ok("");
    };
    bounded(raw.len(), 2048)?;
    std::str::from_utf8(raw).map_err(|_| invalid("TFRRS class encoding"))
}

pub(super) fn has(classes: &str, required: &str) -> bool {
    classes
        .split_ascii_whitespace()
        .any(|value| value == required)
}

pub(super) fn event(classes: &str) -> bool {
    classes
        .split_ascii_whitespace()
        .any(|value| value.starts_with("standard_event_hnd_"))
}

impl Section {
    pub(super) fn new(classes: &str, depth: usize, start: usize) -> Result<Self> {
        let handle = event_handle(classes)?;
        if !has(classes, "row") {
            return Err(invalid("ambiguous TFRRS event section"));
        }
        let gender = match (has(classes, "gender_m"), has(classes, "gender_f")) {
            (true, false) => Gender::Boys,
            (false, true) => Gender::Girls,
            _ => return Err(invalid("ambiguous TFRRS section gender")),
        };
        Ok(Self {
            depth,
            start,
            header: None,
            open_header: None,
            body_depth: None,
            body_seen: false,
            handle,
            gender,
        })
    }

    pub(super) fn begin_header(&mut self, start: usize) -> Result<()> {
        if self.header.is_some() || self.open_header.is_some() || self.body_seen {
            return Err(invalid("ambiguous TFRRS event header"));
        }
        self.open_header = Some(start);
        Ok(())
    }

    pub(super) fn end_header(&mut self, html: &str, end: usize) -> Result<()> {
        let start = self
            .open_header
            .take()
            .ok_or_else(|| invalid("unmatched TFRRS event header"))?;
        let text = markup::text(
            html.get(start..end)
                .ok_or_else(|| invalid("TFRRS header span"))?,
        )?;
        bounded(text.len(), 256)?;
        let suffix = match self.gender {
            Gender::Boys => " (Men)",
            Gender::Girls => " (Women)",
            _ => return Err(invalid("TFRRS header gender")),
        };
        let label = text
            .strip_suffix(suffix)
            .ok_or_else(|| invalid("TFRRS header gender mismatch"))?
            .trim();
        if label.is_empty() {
            return Err(invalid("empty TFRRS event label"));
        }
        self.header = Some((label.into(), start..end));
        Ok(())
    }

    pub(super) fn begin_body(&mut self, depth: usize) -> Result<()> {
        if self.header.is_none() || self.open_header.is_some() || self.body_seen {
            return Err(invalid("ambiguous TFRRS performance body"));
        }
        self.body_seen = true;
        self.body_depth = Some(depth);
        Ok(())
    }

    pub(super) fn context(&self, end: usize) -> Result<Context> {
        let (label, span) = self
            .header
            .as_ref()
            .ok_or_else(|| invalid("missing TFRRS event header"))?;
        Ok(Context {
            handle: self.handle,
            gender: self.gender,
            label: label.clone(),
            section_span: self.start..end,
            header_span: span.clone(),
        })
    }

    pub(super) fn finish(&self) -> Result<()> {
        if !self.body_seen
            || self.body_depth.is_some()
            || self.open_header.is_some()
            || self.header.is_none()
        {
            return Err(invalid("unclosed or incomplete TFRRS event section"));
        }
        Ok(())
    }
}

fn event_handle(classes: &str) -> Result<u32> {
    let mut handles = classes
        .split_ascii_whitespace()
        .filter_map(|value| value.strip_prefix("standard_event_hnd_"));
    let raw = handles
        .next()
        .ok_or_else(|| invalid("TFRRS event handle"))?;
    let handle = raw
        .parse::<u32>()
        .map_err(|_| invalid("TFRRS event handle"))?;
    if handle == 0 || handle.to_string() != raw || handles.next().is_some() {
        return Err(invalid("ambiguous TFRRS event handle"));
    }
    Ok(handle)
}
