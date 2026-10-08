use crate::directory::ReadOutcome;
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::{AssociationLabel, DirectoryError, SourceLabel};
use regex::Regex;
use std::sync::LazyLock;

mod address;
mod listing;
mod rows;
use rows::read_row;

const LISTING_CLASS: &str = "school-list-item";
const ASSOCIATIONS: [(&str, &str); 6] = [
    ("NAIS", "national association of independent schools"),
    ("CAPE", "council for american private education"),
    (
        "NASSP",
        "national association of secondary school principals",
    ),
    ("NAIS", "nais"),
    ("CAPE", "cape"),
    ("NASSP", "nassp"),
];

struct Patterns {
    div: Regex,
    h3: Regex,
    close_div: Regex,
    br: Regex,
    tag: Regex,
    associations: Vec<(&'static str, Regex)>,
}

static PATTERNS: LazyLock<Result<Patterns, regex::Error>> = LazyLock::new(|| {
    let associations = ASSOCIATIONS
        .iter()
        .map(|(label, phrase)| {
            Regex::new(&format!(r"(?i)\b{}\b", regex::escape(phrase))).map(|re| (*label, re))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Patterns {
        div: Regex::new(r#"(?is)<div[^>]*class\s*=\s*["']([^"']*)["'][^>]*>"#)?,
        h3: Regex::new(r"(?is)<h3[^>]*>(.*?)</h3>")?,
        close_div: Regex::new(r"(?is)</div>")?,
        br: Regex::new(r"(?is)<br\s*/?>")?,
        tag: Regex::new(r"(?is)<[^>]*>")?,
        associations,
    })
});

pub fn parse_listing(text: &str) -> CrawlResult<ReadOutcome> {
    if text.len() > 8 * 1024 * 1024 {
        return Err(CrawlError::Resource {
            resource: "association body bytes",
            requested: text.len(),
            limit: 8 * 1024 * 1024,
        });
    }
    let patterns = PATTERNS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "private_assoc patterns",
        source: source.clone(),
    })?;
    let label = SourceLabel::PrivateAssociation {
        label: association_label(text, patterns)?,
    };
    listing::read_listing(text, patterns, &label)
}

fn association_label(text: &str, patterns: &Patterns) -> CrawlResult<AssociationLabel> {
    let label = patterns
        .associations
        .iter()
        .find(|(_, pattern)| pattern.is_match(text))
        .map(|(label, _)| *label)
        .ok_or_else(|| CrawlError::Invariant {
            detail: "the listing names no association this reader recognizes (NAIS, CAPE, NASSP)"
                .to_string(),
        })?;
    AssociationLabel::parse(label).map_err(|error| CrawlError::Invariant {
        detail: error.to_string(),
    })
}

fn has_class(classes: &str, wanted: &str) -> bool {
    classes
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case(wanted))
}

fn check_size(
    resource: &'static str,
    requested: usize,
    limit: usize,
) -> Result<(), DirectoryError> {
    if requested > limit {
        return Err(DirectoryError::Capacity {
            resource,
            requested,
            limit,
        });
    }
    Ok(())
}
