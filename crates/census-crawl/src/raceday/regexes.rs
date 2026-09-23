//! Compiled patterns for the RaceDay HTML shapes, and the accessors that turn a failed compile
//! into a typed error.

use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::sync::LazyLock;

static TABLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<table[^>]*>.*?</table>"));
static TITLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>"));
static ROW: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr[^>]*>(.*?)</tr>"));
static CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<t[hd][^>]*>(.*?)</t[hd]>"));
static HEAD: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<thead.*?</thead>"));
static BODY: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tbody[^>]*>.*?</tbody>"));
static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));

// Accessors for the literal patterns above: a failed compile is a programming error, so it comes
// back as a typed error that the reader answers as "this file carries no meet" — never a panic.
pub(super) fn table_regex() -> CrawlResult<&'static Regex> {
    TABLE.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TABLE",
        source: source.clone(),
    })
}

pub(super) fn title_regex() -> CrawlResult<&'static Regex> {
    TITLE.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TITLE",
        source: source.clone(),
    })
}

pub(super) fn row_regex() -> CrawlResult<&'static Regex> {
    ROW.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "ROW",
        source: source.clone(),
    })
}

pub(super) fn cell_regex() -> CrawlResult<&'static Regex> {
    CELL.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "CELL",
        source: source.clone(),
    })
}

pub(super) fn head_regex() -> CrawlResult<&'static Regex> {
    HEAD.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "HEAD",
        source: source.clone(),
    })
}

pub(super) fn body_regex() -> CrawlResult<&'static Regex> {
    BODY.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "BODY",
        source: source.clone(),
    })
}

pub(super) fn tags_regex() -> CrawlResult<&'static Regex> {
    TAGS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "TAGS",
        source: source.clone(),
    })
}

/// The patterns one result body is read with, resolved together.
///
/// A body is walked with all five at once, so a parse resolves them once and hands the bundle down,
/// instead of one accessor call per use site.
pub(super) struct Patterns {
    pub(super) table: &'static Regex,
    pub(super) head: &'static Regex,
    pub(super) row: &'static Regex,
    pub(super) cell: &'static Regex,
    pub(super) body: &'static Regex,
}

impl Patterns {
    /// Resolve every pattern the reader needs, or the typed error naming the one that failed.
    pub(super) fn compile() -> CrawlResult<Self> {
        Ok(Self {
            table: table_regex()?,
            head: head_regex()?,
            row: row_regex()?,
            cell: cell_regex()?,
            body: body_regex()?,
        })
    }
}
