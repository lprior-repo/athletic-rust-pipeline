use crate::directory::{compile_pattern, group};
use crate::{CrawlError, CrawlResult};
use regex::Regex;
use std::collections::BTreeMap;

const MAX_SOURCE_ROWS: usize = 20_000;
const ATTRIBUTE: &str = r#"([A-Za-z_:][A-Za-z0-9_:.-]*)\s*=\s*(?:"([^"]*)"|'([^']*)')"#;
const LITERAL: &str =
    r#"\b(firstHalf|secondHalf|window\.client_title)\s*=\s*(?:"([^"\\]*)"|'([^'\\]*)')"#;
const TAG: &str = r"(?s)<[^>]*>";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffRow {
    pub id: String,
    pub category_id: String,
    pub name: String,
    pub sport: String,
    pub level: String,
    pub role: String,
    pub email: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StaffDirectory {
    pub name: String,
    pub members: Vec<StaffRow>,
}

struct Markup {
    attribute: Regex,
    literal: Regex,
    tag: Regex,
}

impl Markup {
    fn new() -> CrawlResult<Self> {
        Ok(Self {
            attribute: compile_pattern(ATTRIBUTE, "SIDEARM attributes")?,
            literal: compile_pattern(LITERAL, "SIDEARM published literals")?,
            tag: compile_pattern(TAG, "SIDEARM text tags")?,
        })
    }

    fn attribute<'a>(&self, tag: &'a str, name: &str) -> &'a str {
        self.attribute
            .captures_iter(tag)
            .find(|capture| group(capture, 1) == name)
            .map_or("", |capture| {
                capture
                    .get(2)
                    .or_else(|| capture.get(3))
                    .map_or("", |value| value.as_str())
            })
    }

    fn has_class(&self, tag: &str, name: &str) -> bool {
        self.attribute(tag, "class")
            .split_whitespace()
            .any(|class| class == name)
    }

    fn literal<'a>(&self, body: &'a str, name: &str) -> Option<&'a str> {
        self.literal
            .captures_iter(body)
            .find(|capture| group(capture, 1) == name)
            .and_then(|capture| capture.get(2).or_else(|| capture.get(3)))
            .map(|value| value.as_str().trim())
    }

    fn text(&self, body: &str) -> String {
        let stripped = self.tag.replace_all(body, " ");
        decode_entities(&stripped)
            .split_whitespace()
            .fold(String::new(), |mut text, word| {
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(word);
                text
            })
    }

    fn cell<'a>(&self, row: &'a str, header: &str) -> &'a str {
        elements(row, "<td", "</td>")
            .find(|(tag, _)| {
                self.attribute(tag, "headers")
                    .split_whitespace()
                    .any(|value| value == header)
            })
            .map_or("", |(_, body)| body)
    }

    fn email(&self, row: &str) -> Option<String> {
        let cell = self.cell(row, "col-staff_email");
        let first = self.literal(cell, "firstHalf")?;
        let second = self.literal(cell, "secondHalf")?;
        (!first.is_empty() && !second.is_empty()).then(|| format!("{first}@{second}"))
    }
}

pub fn parse_staff_directory(html: &str) -> CrawlResult<StaffDirectory> {
    if html.len() > crate::net::MAX_BODY_BYTES {
        return Err(schema("oversized SIDEARM staff directory"));
    }
    let markup = Markup::new()?;
    let article = elements(html, "<article", "</article>")
        .find(|(tag, _)| markup.has_class(tag, "sidearm-staff"))
        .map(|(_, body)| body)
        .ok_or_else(|| schema("missing article.sidearm-staff"))?;
    if elements(article, "<tr", "</tr>").count() > MAX_SOURCE_ROWS {
        return Err(schema("SIDEARM staff directory exceeds 20000 table rows"));
    }
    let name = school_name(html, &markup)
        .ok_or_else(|| schema("missing school name in window.client_title or h1"))?;
    let categories: BTreeMap<&str, String> = elements(article, "<tr", "</tr>")
        .filter(|(tag, _)| markup.has_class(tag, "sidearm-staff-category"))
        .map(|(tag, body)| {
            (
                markup.attribute(tag, "data-category-id"),
                elements(body, "<th", "</th>")
                    .next()
                    .map_or_else(String::new, |(_, heading)| markup.text(heading)),
            )
        })
        .collect();
    let members = elements(article, "<tr", "</tr>")
        .filter(|(tag, _)| markup.has_class(tag, "sidearm-staff-member"))
        .map(|(tag, row)| staff_row(&markup, &categories, tag, row))
        .collect();
    Ok(StaffDirectory { name, members })
}

fn staff_row(
    markup: &Markup,
    categories: &BTreeMap<&str, String>,
    tag: &str,
    row: &str,
) -> StaffRow {
    let email = markup
        .email(row)
        .map_or_else(String::new, core::convert::identity);
    let category_id = markup.attribute(tag, "data-category-id");
    let published_sport = markup.text(markup.cell(row, "col-staff_custom_1"));
    let sport = if published_sport.is_empty() {
        categories
            .get(category_id)
            .map_or_else(String::new, |label| label.clone())
    } else {
        published_sport
    };
    let name_cell = markup.cell(row, "col-fullname");
    let name = elements(name_cell, "<a", "</a>")
        .next()
        .map_or_else(String::new, |(_, body)| markup.text(body));
    StaffRow {
        id: markup.attribute(tag, "data-member-id").to_string(),
        category_id: category_id.to_string(),
        name,
        sport,
        level: markup.text(markup.cell(row, "col-staff_custom_2")),
        role: markup.text(markup.cell(row, "col-staff_title")),
        email,
    }
}

fn school_name(html: &str, markup: &Markup) -> Option<String> {
    let title = markup
        .literal(html, "window.client_title")
        .map(|value| markup.text(value))
        .filter(|value| !value.is_empty());
    title.or_else(|| {
        elements(html, "<h1", "</h1>")
            .map(|(_, body)| markup.text(body))
            .find(|value| !value.is_empty())
    })
}

fn elements<'a>(
    html: &'a str,
    open: &'static str,
    close: &'static str,
) -> impl Iterator<Item = (&'a str, &'a str)> {
    html.split(open).skip(1).filter_map(move |chunk| {
        if !chunk.starts_with(|ch: char| ch.is_ascii_whitespace() || ch == '>') {
            return None;
        }
        let (tag, rest) = chunk.split_once('>')?;
        let (body, _) = rest.split_once(close)?;
        Some((tag, body))
    })
}

fn decode_entities(value: &str) -> String {
    let mut parts = value.split('&');
    let prefix = parts.next().map_or("", |part| part);
    parts.fold(prefix.to_string(), |mut text, part| {
        match part
            .split_once(';')
            .and_then(|(entity, rest)| decode_entity(entity).map(|decoded| (decoded, rest)))
        {
            Some((decoded, rest)) => {
                text.push(decoded);
                text.push_str(rest);
            }
            None => {
                text.push('&');
                text.push_str(part);
            }
        }
        text
    })
}

fn decode_entity(entity: &str) -> Option<char> {
    match entity {
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        "lt" => Some('<'),
        "gt" => Some('>'),
        "nbsp" => Some(' '),
        _ => {
            let digits = entity.strip_prefix('#')?;
            let code = match digits
                .strip_prefix('x')
                .or_else(|| digits.strip_prefix('X'))
            {
                Some(hex) => u32::from_str_radix(hex, 16).ok()?,
                None => digits.parse::<u32>().ok()?,
            };
            char::from_u32(code)
        }
    }
}

fn schema(detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: super::DIRECTORY_URL.to_string(),
        detail: detail.to_string(),
    }
}
