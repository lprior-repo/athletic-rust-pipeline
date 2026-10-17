use super::Declaration;
use std::ops::Range;

#[derive(Clone, Copy)]
pub(in crate::milesplit) enum DeclarationKind {
    Canonical,
    OpenGraph,
    Twitter,
    JsonLd,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::milesplit) enum Termination {
    Tag,
    ScriptEnd,
    ScriptEof,
}

pub(in crate::milesplit) struct LocatedDeclaration {
    pub(in crate::milesplit) declaration: Declaration,
    pub(in crate::milesplit) kind: DeclarationKind,
    pub(in crate::milesplit) span: Option<Range<usize>>,
    pub(in crate::milesplit) termination: Termination,
}

pub(super) struct Position {
    offset: Option<usize>,
    tag_start: Option<usize>,
    script_start: Option<usize>,
}

impl Default for Position {
    fn default() -> Self {
        Self {
            offset: Some(0),
            tag_start: None,
            script_start: None,
        }
    }
}

impl Position {
    pub(super) fn advance(&mut self, offset: isize) {
        self.offset = self
            .offset
            .and_then(|value| value.checked_add_signed(offset));
    }

    pub(super) fn open(&mut self) {
        self.tag_start = self.offset.and_then(|value| value.checked_sub(1));
    }

    pub(super) fn script(&mut self) {
        self.script_start = self.tag_start;
    }

    pub(super) fn locate(
        &self,
        declaration: Declaration,
        kind: DeclarationKind,
        termination: Termination,
    ) -> LocatedDeclaration {
        let start = match termination {
            Termination::Tag => self.tag_start,
            Termination::ScriptEnd | Termination::ScriptEof => self.script_start,
        };
        let span = start
            .zip(self.offset)
            .filter(|(start, end)| start < end)
            .map(|(start, end)| start..end);
        LocatedDeclaration {
            declaration,
            kind,
            span,
            termination,
        }
    }
}

#[cfg(test)]
mod tests;
