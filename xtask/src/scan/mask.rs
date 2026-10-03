use regex::Regex;

#[derive(Default)]
pub(crate) struct CodeMask {
    block_depth: u32,
    raw_hashes: Option<usize>,
}

impl CodeMask {
    pub(crate) fn apply_all(&mut self, lines: &[String], char_literal: &Regex) -> Vec<String> {
        lines
            .iter()
            .map(|line| self.apply(line, char_literal))
            .collect()
    }

    fn apply(&mut self, line: &str, char_literal: &Regex) -> String {
        let chars: Vec<char> = line.chars().collect();
        let mut masked = chars.clone();
        let mut index = 0usize;
        while index < chars.len() {
            index = if self.raw_hashes.is_some() {
                self.end_raw_string(&chars, &mut masked, index)
            } else if self.block_depth > 0 {
                self.end_block_comment(&chars, &mut masked, index)
            } else {
                self.advance(&chars, &mut masked, index, char_literal)
            };
        }
        masked.into_iter().collect()
    }

    fn advance(
        &mut self,
        line: &[char],
        masked: &mut [char],
        index: usize,
        char_literal: &Regex,
    ) -> usize {
        match line.get(index).copied() {
            Some('/') if starts_with(line, index, &['/', '/']) => {
                blank(masked, index, line.len());
                line.len()
            }
            Some('/') if starts_with(line, index, &['/', '*']) => {
                self.block_depth = 1;
                blank(masked, index, index.saturating_add(2));
                self.end_block_comment(line, masked, index.saturating_add(2))
            }
            Some('r') if !continues_identifier(line, index) => {
                self.start_raw_string(line, masked, index)
            }
            Some('"') => self.end_string(line, masked, index),
            Some('\'') => self.end_char_literal(line, masked, index, char_literal),
            _ => index.saturating_add(1),
        }
    }

    fn start_raw_string(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let mut cursor = index.saturating_add(1);
        while line.get(cursor) == Some(&'#') {
            cursor = cursor.saturating_add(1);
        }
        if line.get(cursor) != Some(&'"') {
            return index.saturating_add(1);
        }
        self.raw_hashes = Some(cursor.saturating_sub(index).saturating_sub(1));
        blank(masked, index, cursor.saturating_add(1));
        self.end_raw_string(line, masked, cursor.saturating_add(1))
    }

    fn end_raw_string(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let terminator = terminator(self.raw_hashes.map_or(0, core::convert::identity));
        match find_from(line, index, &terminator) {
            Some(end) => {
                self.raw_hashes = None;
                let after = end.saturating_add(terminator.len());
                blank(masked, index, after);
                after
            }
            None => {
                blank(masked, index, line.len());
                line.len()
            }
        }
    }

    fn end_block_comment(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let start = index;
        let mut cursor = index;
        while cursor < line.len() {
            let nested = find_from(line, cursor, &['/', '*']);
            let close = find_from(line, cursor, &['*', '/']);
            if opens_first(nested, close) {
                let Some(next) = nested else { break };
                self.block_depth = self.block_depth.saturating_add(1);
                cursor = next.saturating_add(2);
                continue;
            }
            let Some(end) = close else { break };
            self.block_depth = self.block_depth.saturating_sub(1);
            if self.block_depth == 0 {
                let after = end.saturating_add(2);
                blank(masked, start, after);
                return after;
            }
            cursor = end.saturating_add(2);
        }
        blank(masked, start, line.len());
        line.len()
    }

    fn end_string(&mut self, line: &[char], masked: &mut [char], index: usize) -> usize {
        let mut cursor = index.saturating_add(1);
        while let Some(char) = line.get(cursor).copied() {
            if char == '\\' {
                cursor = cursor.saturating_add(2);
            } else if char == '"' {
                let after = cursor.saturating_add(1);
                blank(masked, index, after);
                return after;
            } else {
                cursor = cursor.saturating_add(1);
            }
        }
        blank(masked, index, line.len());
        line.len()
    }

    fn end_char_literal(
        &mut self,
        line: &[char],
        masked: &mut [char],
        index: usize,
        char_literal: &Regex,
    ) -> usize {
        let tail: String = line.get(index..).map_or(Default::default(), core::convert::identity)
        .iter()
        .collect();
        let Some(literal) = char_literal.find(&tail) else {
            return index.saturating_add(1);
        };
        let consumed = tail
            .get(..literal.end())
            .map_or(0, |matched| matched.chars().count());
        let after = index.saturating_add(consumed);
        blank(masked, index, after);
        after
    }
}

fn opens_first(nested: Option<usize>, close: Option<usize>) -> bool {
    match (nested, close) {
        (Some(next), Some(end)) => next < end,
        (Some(_), None) => true,
        (None, _) => false,
    }
}

fn terminator(hashes: usize) -> Vec<char> {
    let mut terminator: Vec<char> = Vec::with_capacity(hashes.saturating_add(1));
    terminator.push('"');
    terminator.resize(hashes.saturating_add(1), '#');
    terminator
}

fn starts_with(line: &[char], index: usize, needle: &[char]) -> bool {
    needle
        .iter()
        .enumerate()
        .all(|(offset, expected)| line.get(index.saturating_add(offset)) == Some(expected))
}

fn find_from(line: &[char], index: usize, needle: &[char]) -> Option<usize> {
    let first = *needle.first()?;
    let mut position = index;
    while let Some(char) = line.get(position) {
        if *char == first && starts_with(line, position, needle) {
            return Some(position);
        }
        position = position.saturating_add(1);
    }
    None
}

fn continues_identifier(line: &[char], index: usize) -> bool {
    index > 0
        && line
            .get(index.saturating_sub(1))
            .is_some_and(|char| char.is_alphanumeric() || *char == '_')
}

fn blank(masked: &mut [char], start: usize, end: usize) {
    let limit = end.min(masked.len());
    let mut position = start;
    while position < limit {
        if let Some(slot) = masked.get_mut(position) {
            *slot = ' ';
        }
        position = position.saturating_add(1);
    }
}
