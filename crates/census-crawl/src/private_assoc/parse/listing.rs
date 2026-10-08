use super::{check_size, has_class, read_row, Patterns, LISTING_CLASS};
use crate::directory::ReadOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::{DirectoryError, SourceLabel};

type ItemSpan<'a> = (regex::Captures<'a>, usize);

pub(super) fn read_listing(
    text: &str,
    patterns: &Patterns,
    label: &SourceLabel,
) -> CrawlResult<ReadOutcome> {
    let mut items = item_spans(text, patterns).peekable();
    if items.peek().is_none() {
        return Err(CrawlError::Invariant {
            detail: "the body carries no school-list-item element".to_string(),
        });
    }
    Ok(read_items(text, patterns, label, items))
}

fn item_spans<'a>(text: &'a str, patterns: &'a Patterns) -> impl Iterator<Item = ItemSpan<'a>> {
    let mut items = patterns.div.captures_iter(text).filter(is_item).peekable();
    std::iter::from_fn(move || {
        let captures = items.next()?;
        let end = items.peek().and_then(|next| next.get(0));
        Some((captures, end.map_or(text.len(), |tag| tag.start())))
    })
}

fn is_item(captures: &regex::Captures<'_>) -> bool {
    let classes = captures.get(1).map_or("", |value| value.as_str());
    has_class(classes, LISTING_CLASS)
}

fn read_items<'a>(
    text: &str,
    patterns: &Patterns,
    label: &SourceLabel,
    items: impl Iterator<Item = ItemSpan<'a>>,
) -> ReadOutcome {
    let mut outcome = ReadOutcome::new();
    let mut failed_line = 0;
    let mut cursor = Cursor { line: 1, offset: 0 };
    let result = items.enumerate().try_for_each(|(index, (captures, end))| {
        failed_line = index.saturating_add(1);
        check_size("association rows", failed_line, 20_000)?;
        let (line, chunk) = cursor.locate(text, &captures, end)?;
        failed_line = line;
        read_row(line, chunk, patterns, label, &mut outcome)
    });
    if let Err(error) = result {
        outcome.stop(failed_line, error);
    }
    outcome
}

struct Cursor {
    line: usize,
    offset: usize,
}

impl Cursor {
    fn locate<'a>(
        &mut self,
        text: &'a str,
        captures: &regex::Captures<'_>,
        end: usize,
    ) -> Result<(usize, &'a str), DirectoryError> {
        let missing = || DirectoryError::Representation {
            detail: "listing item has an invalid locator".to_string(),
        };
        let tag = captures.get(0).ok_or_else(missing)?;
        let prefix = text.get(self.offset..tag.start()).ok_or_else(missing)?;
        self.line = self.line.saturating_add(prefix.matches('\n').count());
        self.offset = tag.start();
        let chunk = text.get(tag.end()..end).ok_or_else(missing)?;
        Ok((self.line, chunk))
    }
}
