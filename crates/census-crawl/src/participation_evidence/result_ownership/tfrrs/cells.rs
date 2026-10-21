use super::{bounded, invalid, Result, RetainedResultOwnershipError};
use html5gum::{DefaultEmitter, EndTag, StartTag, Token, Tokenizer};
use std::ops::Range;

pub(super) struct Cell {
    pub(super) label: String,
    pub(super) span: Range<usize>,
}

pub(super) fn read(html: &str, offset: usize) -> Result<Vec<Cell>> {
    bounded(html.len(), 65_536)?;
    let mut state = State::new(offset)?;
    let mut emitter = DefaultEmitter::<usize>::new_with_span();
    emitter.naively_switch_states(true);
    for (index, token) in Tokenizer::new_with_emitter(html, emitter).enumerate() {
        bounded(
            index
                .checked_add(1)
                .ok_or(RetainedResultOwnershipError::Overflow)?,
            8192,
        )?;
        state.accept(token.map_err(|_| invalid("TFRRS row tokenization"))?)?;
    }
    if !state.stack.is_empty() || state.cell.is_some() || !state.closed {
        return Err(invalid("unclosed TFRRS selected row or cell"));
    }
    validate(&state.cells)?;
    Ok(state.cells)
}

struct State {
    offset: usize,
    stack: Vec<Vec<u8>>,
    cell: Option<(usize, String)>,
    cells: Vec<Cell>,
    closed: bool,
}

impl State {
    fn new(offset: usize) -> Result<Self> {
        let mut stack = Vec::new();
        stack
            .try_reserve_exact(64)
            .map_err(|_| RetainedResultOwnershipError::Allocation)?;
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(10)
            .map_err(|_| RetainedResultOwnershipError::Allocation)?;
        Ok(Self {
            offset,
            stack,
            cell: None,
            cells,
            closed: false,
        })
    }

    fn accept(&mut self, token: Token<usize>) -> Result<()> {
        match token {
            Token::StartTag(tag) => self.start(tag)?,
            Token::EndTag(tag) => self.end(tag)?,
            Token::String(text)
                if self.cell.is_none() && !text.as_slice().iter().all(u8::is_ascii_whitespace) =>
            {
                return Err(invalid("TFRRS row text outside owned cell"));
            }
            Token::Error(_) | Token::Doctype(_) => {
                return Err(invalid("malformed selected TFRRS row"))
            }
            _ => {}
        }
        Ok(())
    }

    fn start(&mut self, tag: StartTag<usize>) -> Result<()> {
        self.validate_start(&tag)?;
        let name = tag.name.as_slice();
        if self.stack.len() == 1 {
            self.begin_cell(&tag)?;
        } else if tag.attributes.contains_key(b"data-label".as_slice()) {
            return Err(invalid("nested TFRRS cell"));
        }
        if name != b"br" {
            bounded(
                self.stack
                    .len()
                    .checked_add(1)
                    .ok_or(RetainedResultOwnershipError::Overflow)?,
                64,
            )?;
            self.stack.push(tag.name.0);
        }
        Ok(())
    }

    fn validate_start(&self, tag: &StartTag<usize>) -> Result<()> {
        attributes(tag)?;
        let name = tag.name.as_slice();
        if self.closed
            || !matches!(
                name,
                b"div"
                    | b"a"
                    | b"span"
                    | b"br"
                    | b"sup"
                    | b"b"
                    | b"strong"
                    | b"small"
                    | b"i"
                    | b"em"
            )
            || super::visibility::hidden(tag)?
            || (tag.self_closing && name != b"br")
        {
            return Err(invalid("unsupported or inactive selected TFRRS markup"));
        }
        if self.stack.is_empty() && (name != b"div" || tag.span.start != 0) {
            return Err(invalid("TFRRS selected row root"));
        }
        if name == b"a" && self.stack.iter().any(|open| open.as_slice() == b"a") {
            return Err(invalid("nested TFRRS native link"));
        }
        Ok(())
    }

    fn begin_cell(&mut self, tag: &StartTag<usize>) -> Result<()> {
        if tag.name.as_slice() != b"div" {
            return Err(invalid("TFRRS row child is not a cell"));
        }
        let raw = tag
            .attributes
            .get(b"data-label".as_slice())
            .ok_or_else(|| invalid("unlabeled TFRRS cell"))?;
        let label = std::str::from_utf8(raw).map_err(|_| invalid("TFRRS cell label encoding"))?;
        if self.cell.is_some() || self.cells.iter().any(|cell| cell.label == label) {
            return Err(invalid("duplicate TFRRS cell"));
        }
        self.cell = Some((tag.span.start, label.into()));
        Ok(())
    }

    fn end(&mut self, tag: EndTag<usize>) -> Result<()> {
        let open = self
            .stack
            .last()
            .ok_or_else(|| invalid("unmatched TFRRS row tag"))?;
        if open.as_slice() != tag.name.as_slice() {
            return Err(invalid("ambiguous TFRRS row nesting"));
        }
        if self.stack.len() == 2 {
            let (start, label) = self
                .cell
                .take()
                .ok_or_else(|| invalid("TFRRS cell closure"))?;
            bounded(
                tag.span
                    .end
                    .checked_sub(start)
                    .ok_or(RetainedResultOwnershipError::Overflow)?,
                8192,
            )?;
            bounded(
                self.cells
                    .len()
                    .checked_add(1)
                    .ok_or(RetainedResultOwnershipError::Overflow)?,
                10,
            )?;
            let start = self
                .offset
                .checked_add(start)
                .ok_or(RetainedResultOwnershipError::Overflow)?;
            let end = self
                .offset
                .checked_add(tag.span.end)
                .ok_or(RetainedResultOwnershipError::Overflow)?;
            self.cells.push(Cell {
                label,
                span: start..end,
            });
        }
        self.stack.pop();
        if self.stack.is_empty() {
            self.closed = true;
        }
        Ok(())
    }
}

fn attributes(tag: &StartTag<usize>) -> Result<()> {
    bounded(tag.attributes.len(), 32)?;
    for (key, value) in &tag.attributes {
        bounded(key.len(), 128)?;
        bounded(value.len(), 2048)?;
    }
    Ok(())
}

fn validate(cells: &[Cell]) -> Result<()> {
    for label in ["Athlete", "Year", "Team", "Meet", "Meet Date"] {
        if !cells.iter().any(|cell| cell.label == label) {
            return Err(invalid("missing TFRRS native row cell"));
        }
    }
    let time = cells.iter().any(|cell| cell.label == "Time");
    let mark = cells.iter().any(|cell| cell.label == "Mark");
    if time == mark {
        return Err(invalid("ambiguous TFRRS mark cell"));
    }
    if cells.iter().any(|cell| {
        !matches!(
            cell.label.as_str(),
            "Place"
                | "Athlete"
                | "Year"
                | "Team"
                | "Time"
                | "Mark"
                | "Meet"
                | "Meet Date"
                | "Conv"
                | "Wind"
        )
    }) {
        return Err(invalid("unsupported TFRRS row cell"));
    }
    Ok(())
}

pub(super) fn get<'a>(cells: &'a [Cell], label: &str) -> Result<&'a Cell> {
    cells
        .iter()
        .find(|cell| cell.label == label)
        .ok_or_else(|| invalid("missing checked TFRRS cell"))
}
