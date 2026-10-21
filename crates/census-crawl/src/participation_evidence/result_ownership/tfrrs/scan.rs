use super::{bounded, context, invalid, RawRow, Result, RetainedResultOwnershipError};
use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::markup;
use html5gum::{DefaultEmitter, EndTag, StartTag, Token, Tokenizer};

pub(in super::super) enum SelectionStep<'a> {
    Body(&'a str),
    Row(SourceRowLocator),
    Work,
}

pub(super) fn selected(
    html: &str,
    expected_count: usize,
    select: &mut impl FnMut(SelectionStep<'_>) -> Result<bool>,
) -> Result<Vec<RawRow>> {
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    let mut visibility = markup::Active::default();
    let mut state = State::new(expected_count, select)?;
    for (index, token) in Tokenizer::new_with_emitter(html, emitter).enumerate() {
        (state.select)(SelectionStep::Work)?;
        bounded(
            index
                .checked_add(1)
                .ok_or(RetainedResultOwnershipError::Overflow)?,
            100_000,
        )?;
        let mut token = token.map_err(|_| invalid("TFRRS HTML tokenization"))?;
        super::visibility::mark(&mut token)?;
        if visibility.accept(&token)? {
            state.accept(html, token)?;
        }
    }
    visibility.finish()?;
    if state.section.is_some() || state.row.is_some() || state.output.len() != expected_count {
        return Err(invalid(
            "requested TFRRS rows have no complete active ownership",
        ));
    }
    Ok(state.output)
}

struct State<'a, F> {
    depth: usize,
    ordinal: u32,
    section: Option<context::Section>,
    row: Option<(usize, usize, u32)>,
    expected_count: usize,
    select: &'a mut F,
    output: Vec<RawRow>,
    last_error: Option<html5gum::Span<usize>>,
}

impl<'a, F> State<'a, F>
where
    F: FnMut(SelectionStep<'_>) -> Result<bool>,
{
    fn new(expected_count: usize, select: &'a mut F) -> Result<Self> {
        let mut output = Vec::new();
        output
            .try_reserve_exact(expected_count)
            .map_err(|_| RetainedResultOwnershipError::Allocation)?;
        Ok(Self {
            depth: 0,
            ordinal: 0,
            section: None,
            row: None,
            expected_count,
            select,
            output,
            last_error: None,
        })
    }

    fn accept(&mut self, html: &str, token: Token<usize>) -> Result<()> {
        match token {
            Token::StartTag(tag) if tag.name.as_slice() == b"div" => self.start_div(tag)?,
            Token::EndTag(tag) if tag.name.as_slice() == b"div" => self.end_div(tag)?,
            Token::StartTag(tag) if tag.name.as_slice() == b"h3" => {
                self.require_tokenized(&tag)?;
                if let Some(section) = self.section.as_mut() {
                    section.begin_header(tag.span.start)?;
                }
            }
            Token::EndTag(tag) if tag.name.as_slice() == b"h3" => {
                if let Some(section) = self.section.as_mut() {
                    section.end_header(html, tag.span.end)?;
                }
            }
            Token::Error(error) => {
                if self
                    .section
                    .as_ref()
                    .is_some_and(|section| section.open_header.is_some())
                {
                    return Err(invalid("malformed TFRRS owned event header"));
                }
                self.last_error = Some(error.span);
            }
            _ => {}
        }
        Ok(())
    }

    fn start_div(&mut self, tag: StartTag<usize>) -> Result<()> {
        self.depth = self
            .depth
            .checked_add(1)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        bounded(self.depth, 128)?;
        let classes = context::classes(&tag)?;
        if context::event(classes)
            || context::has(classes, "performance-list-body")
            || context::has(classes, "performance-list-row")
        {
            self.require_tokenized(&tag)?;
        }
        if context::event(classes) {
            if self.section.is_some() {
                return Err(invalid("nested TFRRS event section"));
            }
            self.section = Some(context::Section::new(classes, self.depth, tag.span.start)?);
        } else if context::has(classes, "performance-list-body") {
            self.section
                .as_mut()
                .ok_or_else(|| invalid("TFRRS body outside event"))?
                .begin_body(self.depth)?;
        } else if context::has(classes, "performance-list-row") {
            self.start_row(tag.span.start)?;
        }
        Ok(())
    }

    fn require_tokenized(&self, tag: &StartTag<usize>) -> Result<()> {
        if self
            .last_error
            .is_some_and(|error| error.start >= tag.span.start && error.end <= tag.span.end)
        {
            return Err(invalid("malformed TFRRS owned topology tag"));
        }
        Ok(())
    }

    fn start_row(&mut self, start: usize) -> Result<()> {
        let section = self
            .section
            .as_ref()
            .ok_or_else(|| invalid("TFRRS row outside event"))?;
        if self.row.is_some()
            || section.body_depth.and_then(|depth| depth.checked_add(1)) != Some(self.depth)
        {
            return Err(invalid("ambiguous TFRRS row ancestry"));
        }
        self.ordinal = self
            .ordinal
            .checked_add(1)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        bounded(
            usize::try_from(self.ordinal).map_err(|_| RetainedResultOwnershipError::Overflow)?,
            4096,
        )?;
        self.row = Some((self.depth, start, self.ordinal));
        Ok(())
    }

    fn end_div(&mut self, tag: EndTag<usize>) -> Result<()> {
        if self.depth == 0 {
            if self.section.is_some() {
                return Err(invalid("unmatched TFRRS section div"));
            }
            return Ok(());
        }
        if self
            .row
            .as_ref()
            .is_some_and(|(depth, _, _)| *depth == self.depth)
        {
            self.end_row(tag.span.end)?;
        }
        if let Some(section) = self.section.as_mut() {
            if section.body_depth == Some(self.depth) {
                section.body_depth = None;
            }
        }
        if self
            .section
            .as_ref()
            .is_some_and(|section| section.depth == self.depth)
        {
            let section = self
                .section
                .take()
                .ok_or_else(|| invalid("TFRRS section close"))?;
            section.finish()?;
            for row in &mut self.output {
                (self.select)(SelectionStep::Work)?;
                if row.context.section_span.start == section.start {
                    row.context.section_span.end = tag.span.end;
                }
            }
        }
        self.depth = self
            .depth
            .checked_sub(1)
            .ok_or(RetainedResultOwnershipError::Overflow)?;
        Ok(())
    }

    fn end_row(&mut self, end: usize) -> Result<()> {
        let (_, start, ordinal) = self.row.take().ok_or_else(|| invalid("TFRRS row close"))?;
        let locator = SourceRowLocator {
            ordinal,
            byte_offset: start,
            byte_length: end
                .checked_sub(start)
                .ok_or(RetainedResultOwnershipError::Overflow)?,
        };
        if (self.select)(SelectionStep::Row(locator))? {
            if self.output.len() >= self.expected_count {
                return Err(invalid("duplicate successful TFRRS locator"));
            }
            let context = self
                .section
                .as_ref()
                .ok_or_else(|| invalid("TFRRS row event"))?
                .context(end)?;
            self.output.push(RawRow { locator, context });
        }
        Ok(())
    }
}
