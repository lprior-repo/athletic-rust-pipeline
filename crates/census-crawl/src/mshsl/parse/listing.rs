use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::SchoolListRow;
use crate::directory::acquisition::resource;
use crate::mshsl::text::{clean_bounded, copy_bounded};
use crate::mshsl::{MAX_LISTING_BYTES, SCHOOL_LIST_URL};
use crate::{CrawlError, CrawlResult, SCHOOL_BINDINGS};

const ROW_MARKER: &str = "<div class=\"views-row\">";
const TITLE_MARKER: &str = "school-teaser__title";
const LOCALITY_MARKER: &str = "class=\"locality\"";

static LIST_ROW: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(r##"<a href="/schools/([^"#?]+)" class="school-teaser__title">([^<]*)</a>"##)
});
static LOCALITY: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"<span class="locality">([^<]*)</span>"#));

fn pattern(
    cached: &'static Result<Regex, regex::Error>,
    name: &'static str,
) -> CrawlResult<&'static Regex> {
    cached.as_ref().map_err(|source| CrawlError::RegexInit {
        pattern: name,
        source: source.clone(),
    })
}

fn schema(detail: &'static str) -> CrawlError {
    CrawlError::Schema {
        url: SCHOOL_LIST_URL.to_string(),
        detail: detail.to_string(),
    }
}

fn segments(html: &str) -> CrawlResult<impl Iterator<Item = &str>> {
    if html.len() > MAX_LISTING_BYTES {
        return Err(resource(
            "MSHSL listing HTML bytes",
            html.len(),
            MAX_LISTING_BYTES,
        ));
    }
    let count = html
        .matches(ROW_MARKER)
        .take(SCHOOL_BINDINGS.saturating_add(1))
        .count();
    if count > SCHOOL_BINDINGS {
        return Err(resource("MSHSL listing rows", count, SCHOOL_BINDINGS));
    }
    if let Some((prefix, _)) = html.split_once(ROW_MARKER) {
        if prefix.contains(TITLE_MARKER) {
            return Err(schema("school title precedes the first listing row"));
        }
    }
    Ok(html.split(ROW_MARKER).skip(usize::from(count > 0)))
}

fn city(segment: &str, locality: &Regex) -> CrawlResult<Option<String>> {
    match segment.matches(LOCALITY_MARKER).count() {
        0 => Ok(None),
        1 => {
            let capture = locality
                .captures(segment)
                .ok_or_else(|| schema("listing row has malformed locality"))?;
            let value = capture
                .get(1)
                .ok_or_else(|| schema("listing locality capture is missing"))?;
            let city = clean_bounded(value.as_str())?;
            Ok((!city.is_empty()).then_some(city))
        }
        _ => Err(schema("listing row has multiple localities")),
    }
}

fn read_row<'a>(
    segment: &'a str,
    link: &Regex,
    locality: &Regex,
) -> CrawlResult<(&'a str, String, Option<String>)> {
    if segment.matches(TITLE_MARKER).count() != 1 {
        return Err(schema("listing row must contain one school title"));
    }
    let capture = link
        .captures(segment)
        .ok_or_else(|| schema("listing row has no school link"))?;
    let slug = capture
        .get(1)
        .ok_or_else(|| schema("listing slug capture is missing"))?
        .as_str()
        .trim();
    let name = capture
        .get(2)
        .ok_or_else(|| schema("listing name capture is missing"))?;
    let name = clean_bounded(name.as_str())?;
    if slug.is_empty() || slug.contains('/') || name.is_empty() {
        return Err(schema("listing row has an invalid school identity"));
    }
    let city = city(segment, locality)?;
    Ok((slug, name, city))
}

fn append<'a>(
    rows: &mut Vec<SchoolListRow>,
    seen: &mut HashMap<&'a str, usize>,
    slug: &'a str,
    name: String,
    city: Option<String>,
) -> CrawlResult<()> {
    if let Some(index) = seen.get(slug) {
        let previous = rows
            .get(*index)
            .ok_or_else(|| schema("listing identity index is missing"))?;
        if previous.name != name || previous.city != city {
            return Err(schema("conflicting identity for listing slug"));
        }
        return Ok(());
    }
    let requested = rows
        .len()
        .checked_add(1)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "MSHSL listing row count".to_string(),
        })?;
    if requested > SCHOOL_BINDINGS {
        return Err(resource("MSHSL listing rows", requested, SCHOOL_BINDINGS));
    }
    rows.try_reserve(1)
        .map_err(|_| resource("MSHSL listing row allocation", requested, SCHOOL_BINDINGS))?;
    seen.try_reserve(1).map_err(|_| {
        resource(
            "MSHSL listing identity allocation",
            requested,
            SCHOOL_BINDINGS,
        )
    })?;
    let row = SchoolListRow {
        slug: copy_bounded(slug)?,
        name,
        city,
    };
    if seen.insert(slug, rows.len()).is_some() {
        return Err(schema("listing identity index was replaced"));
    }
    rows.push(row);
    Ok(())
}

pub fn parse_school_list(html: &str) -> CrawlResult<Vec<SchoolListRow>> {
    let segments = segments(html)?;
    let link = pattern(&LIST_ROW, "mshsl listing row")?;
    let locality = pattern(&LOCALITY, "mshsl listing locality")?;
    let mut rows = Vec::new();
    let mut seen = HashMap::new();
    for segment in segments {
        let (slug, name, city) = read_row(segment, link, locality)?;
        append(&mut rows, &mut seen, slug, name, city)?;
    }
    Ok(rows)
}
