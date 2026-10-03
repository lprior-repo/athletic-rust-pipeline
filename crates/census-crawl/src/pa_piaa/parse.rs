mod entities;

use entities::unescape;

use regex::Regex;
use std::sync::LazyLock;

use super::{details_url, HOST};
use crate::CrawlError;
use crate::CrawlResult;

static BLOCK: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?s)<dl id="(\d+)" class="schoolBlock[^"]*">(.*?)</dl>"#).ok());
static NAME: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"(?s)<dt>(.*?)</dt>").ok());
static ADDRESS: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?s)<dd[^>]*>(.*?)</dd>").ok());
static TAG: LazyLock<Option<Regex>> = LazyLock::new(|| Regex::new(r"<[^>]+>").ok());
static CITY_STATE_ZIP: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"^(.*?),\s*([A-Z]{2})\s+(\d{5}(?:-\d{4})?)$").ok());
static TITLE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r"(?is)<title>(.*?)</title>").ok());
static CONTACTS: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"(?s)<h3 id="ContactTitle".*?</h3>(.*?)(?:<h3|</section>|\z)"#).ok()
});
static CARD: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<div[^>]*class="[^"]*columns[^"]*"[^>]*>"#).ok());
static NAME_PARTS: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<span class="given-name">(.*?)</span>|<span class="additional-name">(.*?)</span>|<span class="family-name">(.*?)</span>"#,
    )
    .ok()
});
static CONTACT_TITLE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?s)<div class="title">(.*?)</div>"#).ok());
static MAILTO: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"(?i)href="mailto:([^"?]+)""#).ok());

pub struct SchoolRow {
    pub school_id: String,
    pub name: String,
    pub street: Option<String>,
    pub city: Option<String>,
    pub state: Option<String>,
    pub zip: Option<String>,
    pub detail_url: String,
}

pub struct DetailsPage {
    pub school_name: String,
    pub contacts: Vec<ContactRow>,
}

pub struct ContactRow {
    pub person: String,
    pub email: Option<String>,
}

pub fn parse_directory(body: &str) -> CrawlResult<Vec<SchoolRow>> {
    let block = pattern(BLOCK.as_ref(), "school block")?;
    let name = pattern(NAME.as_ref(), "school name")?;
    let address = pattern(ADDRESS.as_ref(), "school address")?;
    let split = pattern(CITY_STATE_ZIP.as_ref(), "address split")?;
    let mut schools = Vec::new();
    for captures in block.captures_iter(body) {
        let (Some(school_id), Some(block_body)) = (captures.get(1), captures.get(2)) else {
            continue;
        };
        if let Some(row) = parse_school_block(
            school_id.as_str(),
            block_body.as_str(),
            name,
            address,
            split,
        ) {
            schools.push(row);
        }
    }
    if schools.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "letter page holds no school block".to_string(),
        });
    }
    Ok(schools)
}

fn parse_school_block(
    school_id: &str,
    block_body: &str,
    name: &Regex,
    address: &Regex,
    split: &Regex,
) -> Option<SchoolRow> {
    let found = name.captures(block_body).and_then(|row| row.get(1))?;
    let school_name = clean(found.as_str());
    if school_name.is_empty() {
        return None;
    }
    let mut row = SchoolRow {
        detail_url: details_url(school_id),
        school_id: school_id.to_string(),
        name: school_name,
        street: None,
        city: None,
        state: Some("PA".to_string()),
        zip: None,
    };
    for cell in address
        .captures_iter(block_body)
        .filter_map(|cell| cell.get(1))
    {
        let line = clean(cell.as_str());
        if line.is_empty() {
            continue;
        }
        match split.captures(line.as_str()) {
            Some(parts) => {
                let head = parts
                    .get(1)
                    .map(|part| part.as_str())
                    .map_or("", |value| value);
                let (street, city) = match head.rsplit_once(',') {
                    Some((street, city)) => (street.trim(), city.trim()),
                    None => (head.trim(), ""),
                };
                row.street = nonempty(street);
                row.city = nonempty(city);
                row.zip = parts.get(3).map(|part| part.as_str().to_string());
                row.state = parts.get(2).map(|part| part.as_str().to_string());
            }
            None if row.street.is_none() => row.street = Some(line),
            None => {}
        }
    }
    Some(row)
}

pub fn parse_details(body: &str) -> CrawlResult<DetailsPage> {
    let title = pattern(TITLE.as_ref(), "page title")?;
    let contacts = pattern(CONTACTS.as_ref(), "contact block")?;
    let card_start = pattern(CARD.as_ref(), "contact card")?;
    let parts = pattern(NAME_PARTS.as_ref(), "contact name")?;
    let contact_title = pattern(CONTACT_TITLE.as_ref(), "contact title")?;
    let mailto = pattern(MAILTO.as_ref(), "contact address")?;
    let school_name = extract_school_name(title, body)?;
    let contacts = parse_contact_block(body, contacts, card_start, parts, contact_title, mailto)?;
    Ok(DetailsPage {
        school_name,
        contacts,
    })
}

fn extract_school_name(title: &Regex, body: &str) -> CrawlResult<String> {
    let name = title
        .captures(body)
        .and_then(|found| found.get(1))
        .map(|found| clean(found.as_str()))
        .map(|name| {
            name.strip_suffix("- PIAA")
                .map_or(name.as_str(), |value| value)
                .trim()
                .to_string()
        })
        .map_or(Default::default(), core::convert::identity);
    if name.is_empty() {
        return Err(CrawlError::Invariant {
            detail: "details page carries no title naming the school".to_string(),
        });
    }
    Ok(name)
}

fn parse_contact_block(
    body: &str,
    contacts: &Regex,
    card_start: &Regex,
    parts: &Regex,
    contact_title: &Regex,
    mailto: &Regex,
) -> CrawlResult<Vec<ContactRow>> {
    let mut rows: Vec<ContactRow> = Vec::new();
    for block in contacts
        .captures_iter(body)
        .filter_map(|found| found.get(1))
    {
        let block_body = block.as_str();
        let starts: Vec<usize> = card_start
            .find_iter(block_body)
            .map(|found| found.start())
            .collect();
        for (index, start) in starts.iter().enumerate() {
            let end = starts
                .get(index.saturating_add(1))
                .copied()
                .map_or(block_body.len(), |value| value);
            let Some(card) = block_body.get(*start..end) else {
                continue;
            };
            let Some(found) = contact_title.captures(card) else {
                continue;
            };
            let Some(title) = found.get(1) else {
                continue;
            };
            if !is_administrator_title(clean(title.as_str()).as_str()) {
                continue;
            }
            let person = parts
                .captures_iter(card)
                .flat_map(|found| (1..=3).filter_map(move |index| found.get(index)))
                .map(|part| clean(part.as_str()))
                .filter(|part| !part.is_empty())
                .collect::<Vec<String>>()
                .join(" ");
            if person.is_empty() {
                continue;
            }
            if rows.iter().any(|row| row.person == person) {
                continue;
            }
            rows.push(ContactRow {
                email: mailto
                    .captures(card)
                    .and_then(|found| found.get(1))
                    .map(|found| clean(found.as_str()))
                    .and_then(|address| nonempty(address.as_str())),
                person,
            });
        }
    }
    Ok(rows)
}

fn is_administrator_title(title: &str) -> bool {
    matches!(
        title.to_lowercase().as_str(),
        "athletic director" | "assistant athletic director" | "activities director"
    )
}

fn pattern<'a>(pattern: Option<&'a Regex>, what: &str) -> CrawlResult<&'a Regex> {
    pattern.ok_or_else(|| CrawlError::Invariant {
        detail: format!("{what} pattern did not compile: {HOST}"),
    })
}

fn nonempty(value: &str) -> Option<String> {
    super::nonempty(value)
}

fn clean(value: &str) -> String {
    let stripped = match TAG.as_ref() {
        Some(tag) => tag.replace_all(value, " ").into_owned(),
        None => value.to_string(),
    };
    unescape(stripped.as_str())
        .split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
}
