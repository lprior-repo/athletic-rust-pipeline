use super::aliases::{self, tokens};
use super::{CatalogError, EventKind};
use crate::model::{EventSpecification, HurdleSpecification, IndoorTrackSpecification};

pub(super) struct ContextualLabel<'a> {
    label: &'a str,
    dimensions: [Option<DimensionSpan>; 2],
}

#[derive(Clone, Copy)]
pub(super) enum LabelToken<'a> {
    Declaration(&'a str),
    ExplicitDimension,
}

impl<'a> LabelToken<'a> {
    pub(super) fn declaration(self) -> Option<&'a str> {
        match self {
            Self::Declaration(word) => Some(word),
            Self::ExplicitDimension => None,
        }
    }
}

impl<'a> ContextualLabel<'a> {
    pub(super) fn parse(label: &'a str, kind: &EventKind) -> Result<Self, CatalogError> {
        let mut parsed = Self {
            label,
            dimensions: [None, None],
        };
        if !super::policy::INCLUDED.contains(kind) {
            return Ok(parsed);
        }
        let specification = match EventSpecification::from_published_label(label, kind) {
            Ok(value) => value,
            Err(_) => return Ok(parsed),
        };
        let mut bytes = 0usize;
        let mut words = 0usize;
        for chunk in label.split_inclusive(specification_separator) {
            let raw = chunk.trim_end_matches(specification_separator);
            let after = bytes
                .checked_add(chunk.len())
                .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
            let end = words
                .checked_add(tokens(raw).count())
                .ok_or(CatalogError::AggregateLabelBytesOverflow)?;
            let prefix = label.get(..bytes).ok_or(CatalogError::InvalidLabel)?;
            let tail = label.get(after..).ok_or(CatalogError::InvalidLabel)?;
            let next = tail
                .split(specification_separator)
                .find(|word| !word.is_empty());
            if let Some(dimension) = recognized_dimension(raw, next, prefix, kind, &specification)?
            {
                let slot = match dimension {
                    Dimension::Hurdles(_) => parsed.dimensions.first_mut(),
                    Dimension::IndoorTrack(_) => parsed.dimensions.last_mut(),
                }
                .ok_or(CatalogError::InvalidLabel)?;
                *slot = Some(DimensionSpan::new(words..end, dimension));
            }
            bytes = after;
            words = end;
        }
        Ok(parsed)
    }

    pub(super) fn tokens(&self) -> impl Iterator<Item = LabelToken<'a>> + Clone + '_ {
        tokens(self.label).enumerate().map(|(index, word)| {
            if self
                .dimensions
                .iter()
                .flatten()
                .any(|span| span.contains(index))
            {
                LabelToken::ExplicitDimension
            } else {
                LabelToken::Declaration(word)
            }
        })
    }

    pub(super) fn pair(&self, left: &str, right: &str) -> bool {
        self.tokens().zip(self.tokens().skip(1)).any(|(one, two)| {
            one.declaration()
                .is_some_and(|word| word.eq_ignore_ascii_case(left))
                && two
                    .declaration()
                    .is_some_and(|word| word.eq_ignore_ascii_case(right))
        })
    }
}

fn specification_separator(ch: char) -> bool {
    ch.is_whitespace() || matches!(ch, '(' | ')' | '[' | ']' | ',')
}

#[derive(Clone, Copy)]
pub(super) enum Dimension {
    Hurdles(HurdleSpecification),
    IndoorTrack(IndoorTrackSpecification),
}

pub(super) struct DimensionSpan {
    tokens: std::ops::Range<usize>,
    dimension: Dimension,
}

impl DimensionSpan {
    pub(super) fn new(tokens: std::ops::Range<usize>, dimension: Dimension) -> Self {
        Self { tokens, dimension }
    }

    pub(super) fn contains(&self, index: usize) -> bool {
        (match self.dimension {
            Dimension::Hurdles(value) => value.height_micrometres != 0,
            Dimension::IndoorTrack(value) => value.length_micrometres != 0,
        }) && self.tokens.contains(&index)
    }
}

pub(super) fn recognized_dimension(
    raw: &str,
    next: Option<&str>,
    prefix: &str,
    kind: &EventKind,
    specification: &EventSpecification,
) -> Result<Option<Dimension>, CatalogError> {
    if !raw.bytes().next().is_some_and(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }
    if let Some(expected) = specification.indoor_track {
        if next.is_some_and(|marker| aliases::member(marker, &["banked", "flat", "track"])) {
            let candidate = context_probe("(", raw, next, ")", kind)?;
            if candidate.is_some_and(|value| value.indoor_track == Some(expected)) {
                return Ok(Some(Dimension::IndoorTrack(expected)));
            }
        }
    }
    if let Some(expected) = specification.hurdles {
        if EventKind::from_source_label(prefix) == *kind {
            for unit in [None, next] {
                let candidate = context_probe("hurdles ", raw, unit, "", kind)?;
                if candidate.is_some_and(|value| value.hurdles == Some(expected)) {
                    return Ok(Some(Dimension::Hurdles(expected)));
                }
            }
        }
    }
    Ok(None)
}

fn context_probe(
    head: &str,
    raw: &str,
    next: Option<&str>,
    tail: &str,
    kind: &EventKind,
) -> Result<Option<EventSpecification>, CatalogError> {
    let bytes = [
        head.len(),
        raw.len(),
        next.map_or(0, str::len),
        1,
        tail.len(),
    ]
    .into_iter()
    .try_fold(0usize, |sum, bytes| {
        sum.checked_add(bytes)
            .ok_or(CatalogError::AggregateLabelBytesOverflow)
    })?;
    let mut probe = String::new();
    probe
        .try_reserve(bytes)
        .map_err(|_| CatalogError::ResourceExhausted)?;
    probe.push_str(head);
    probe.push_str(raw);
    if let Some(next) = next {
        probe.push(' ');
        probe.push_str(next);
    }
    probe.push_str(tail);
    Ok(EventSpecification::from_published_label(&probe, kind).ok())
}
