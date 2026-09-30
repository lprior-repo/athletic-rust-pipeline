use crate::directory::{census_state, optional, skip_absent, skip_optional, skip_row, ReadOutcome};
use crate::{CrawlError, CrawlResult};
use census_domain::school_directory::{
    AssociationLabel, CityName, PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel,
    StreetLine, ZipCode,
};
use regex::Regex;
use std::sync::LazyLock;

mod address;
use address::{address_text, split_address, tag_text};

const LISTING_CLASS: &str = "school-list-item";
const UNKNOWN_ASSOCIATION: &str =
    "the listing names no association this reader recognizes (NAIS, CAPE, NASSP)";
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
        .collect::<Result<Vec<(&'static str, Regex)>, regex::Error>>()?;
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
    let patterns = PATTERNS.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: "private_assoc patterns",
        source: source.clone(),
    })?;
    let label = SourceLabel::PrivateAssociation {
        label: association_label(text, patterns)?,
    };
    let items: Vec<(usize, usize)> = patterns
        .div
        .captures_iter(text)
        .filter(|captures| has_class(captures.get(1).map_or("", |m| m.as_str()), LISTING_CLASS))
        .filter_map(|captures| captures.get(0).map(|tag| (tag.start(), tag.end())))
        .collect();
    if items.is_empty() {
        return Err(CrawlError::Invariant {
            detail: format!("the body carries no {LISTING_CLASS} element"),
        });
    }
    let mut outcome = ReadOutcome::new();
    for (index, (start, open_end)) in items.iter().copied().enumerate() {
        let end = items
            .get(index.saturating_add(1))
            .map_or(text.len(), |(next, _)| *next);
        let chunk = text.get(open_end..end).unwrap_or_default();
        let prefix = text.get(..start).unwrap_or_default();
        let line = prefix.matches('\n').count().saturating_add(1);
        read_row(line, chunk, patterns, &label, &mut outcome);
    }
    Ok(outcome)
}

fn association_label(text: &str, patterns: &Patterns) -> CrawlResult<AssociationLabel> {
    let label = patterns
        .associations
        .iter()
        .find(|(_, pattern)| pattern.is_match(text))
        .map(|(label, _)| *label)
        .ok_or_else(|| CrawlError::Invariant {
            detail: UNKNOWN_ASSOCIATION.to_string(),
        })?;
    AssociationLabel::parse(label).map_err(|error| CrawlError::Invariant {
        detail: format!("{label} is not a usable association label: {error}"),
    })
}

fn has_class(classes: &str, wanted: &str) -> bool {
    classes
        .split_whitespace()
        .any(|token| token.eq_ignore_ascii_case(wanted))
}

fn read_row(
    line: usize,
    chunk: &str,
    patterns: &Patterns,
    label: &SourceLabel,
    outcome: &mut ReadOutcome,
) {
    let name_text = patterns
        .h3
        .captures(chunk)
        .and_then(|captures| captures.get(1))
        .map_or(String::new(), |inner| {
            tag_text(inner.as_str(), &patterns.br, &patterns.tag)
        });
    let Some(name) = skip_row(outcome, line, "name", SchoolName::parse(&name_text)) else {
        return;
    };
    let addr_text = address_text(
        chunk,
        &patterns.div,
        &patterns.close_div,
        &patterns.br,
        &patterns.tag,
    );
    let parts = split_address(&addr_text);
    let street = skip_optional(
        outcome,
        line,
        "street",
        present(&parts.street, StreetLine::parse),
    );
    let line2 = skip_optional(
        outcome,
        line,
        "street line 2",
        present(&parts.line2, StreetLine::parse),
    );
    let city = skip_optional(outcome, line, "city", present(&parts.city, CityName::parse));
    let parsed_zip = present(&parts.zip, |code| ZipCode::of(code, Some(&parts.plus4)));
    let zip = skip_absent(outcome, line, optional("zip", parsed_zip));
    let state = census_state(&parts.state);
    let address = PostalAddress::of(street, line2, city.clone(), state, zip);
    let weak = SchoolDirectoryEntry::weak(name, city, state, label.clone());
    if let Some(entry) = skip_row(outcome, line, "name", weak) {
        outcome.push(entry.with_address(address));
    }
}

fn present<T, F>(
    value: &str,
    parse: F,
) -> Result<Option<T>, census_domain::school_directory::DirectoryError>
where
    F: FnOnce(&str) -> Result<T, census_domain::school_directory::DirectoryError>,
{
    if value.is_empty() {
        Ok(None)
    } else {
        parse(value).map(Some)
    }
}
