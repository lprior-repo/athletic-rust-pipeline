use regex::{Captures, Regex};

use crate::CrawlError;

pub fn compile_pattern(pattern: &'static str, name: &'static str) -> Result<Regex, CrawlError> {
    Regex::new(pattern).map_err(|source| CrawlError::RegexInit {
        pattern: name,
        source,
    })
}

pub fn group<'a>(captures: &Captures<'a>, index: usize) -> &'a str {
    captures
        .get(index)
        .map(|matched| matched.as_str().trim())
        .unwrap_or("")
}

pub fn line_of(text: &str, offset: usize) -> usize {
    text.get(..offset)
        .map(|prefix| prefix.matches('\n').count().saturating_add(1))
        .unwrap_or(1)
}
