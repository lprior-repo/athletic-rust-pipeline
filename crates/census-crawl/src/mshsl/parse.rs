use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

use super::text::{clean, decode_cfemail_fragment};
use super::{SCHOOL_LIST_URL, SCHOOL_URL_PREFIX};

/// Compiled once per process, with no panic path: a malformed pattern yields `None`.
static LIST_ROW: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r##"<a href="/schools/([^"#?]+)" class="school-teaser__title">([^<]*)</a>"##).ok()
});
static LOCALITY: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<span class="locality">([^<]*)</span>"#).ok());
static PAGE_LINK: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"href=["']\?page=([0-9]+)["']"#).ok());
static PAGER_NEXT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<a[^>]*rel="next"[^>]*>"#).ok());
static PAGE_TITLE: LazyLock<Option<Regex>> = LazyLock::new(|| {
    Regex::new(r#"<h1[^>]*class="[^"]*heading--page-title[^"]*"[^>]*>\s*<div>([^<]*)</div>"#).ok()
});
static GROUP_ID: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"/group/([0-9]{1,8})/"#).ok());
static ENROLLMENT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"MSHSL Classification Enrollment:\s*([0-9,]{1,12})"#).ok());
static WEBSITE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<a\s+href="([^"]+)"\s+class="school-intro__link""#).ok());
static ADMIN_ROLE: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"<strong>([^<]*)</strong>"#).ok());
static ADMIN_NAME: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"</strong>\s*<div>([^<]*)</div>"#).ok());
static ADMIN_NAME_TEXT: LazyLock<Option<Regex>> =
    LazyLock::new(|| Regex::new(r#"</strong>([^<]{2,80})"#).ok());

/// One row of the `/schools` listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchoolListRow {
    /// Page slug (`/schools/<slug>`).
    pub slug: String,
    pub name: String,
    pub city: Option<String>,
}

/// One entry of a school page's Administration block.
///
/// Every address published for the entry is kept (an entry can carry more than one `mailto`); the first
/// decoded address is the one emitted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdminEntry {
    pub role: String,
    pub name: String,
    pub emails: Vec<String>,
}

impl AdminEntry {
    pub fn email(&self) -> Option<&str> {
        self.emails.first().map(String::as_str)
    }
}

/// A parsed school detail page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchoolDetail {
    /// `<h1>` title, used only when the listing row has no name.
    pub name: Option<String>,
    /// MSHSL numeric school id, read from the page's `/group/<id>/` links.
    pub school_id: Option<String>,
    pub enrollment: Option<u32>,
    pub website: Option<String>,
    pub admin: Vec<AdminEntry>,
}

/// The school universe as published by the `/schools` listing.
pub fn parse_school_list(html: &str) -> Vec<SchoolListRow> {
    let (Some(rows), Some(locality)) = (LIST_ROW.as_ref(), LOCALITY.as_ref()) else {
        return Vec::new();
    };
    const ROW_MARKER: &str = "<div class=\"views-row\">";
    let mut chunks: Vec<&str> = html.split(ROW_MARKER).skip(1).collect();
    if chunks.is_empty() {
        chunks.push(html);
    }
    let mut school_rows: Vec<SchoolListRow> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for chunk in chunks {
        let Some(capture) = rows.captures(chunk) else {
            continue;
        };
        let (Some(slug), Some(name)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        let slug = slug.as_str().trim().to_string();
        let name = clean(name.as_str());
        if slug.is_empty() || slug.contains('/') || name.is_empty() {
            continue;
        }
        if !seen.insert(slug.clone()) {
            continue;
        }
        let city = locality
            .captures(chunk)
            .and_then(|capture| capture.get(1))
            .map(|value| clean(value.as_str()))
            .filter(|value| !value.is_empty());
        school_rows.push(SchoolListRow { slug, name, city });
    }
    school_rows
}

/// Next listing page number, from the pager markup.
///
/// A page is followed when the pager links to `current + 1`: either as a numbered page link or through
/// the "next" anchor, which is what a Drupal pager renders on every page but the last.
pub fn parse_next_listing_page(html: &str, current: usize) -> Option<usize> {
    let next = current.checked_add(1)?;
    let link = PAGE_LINK.as_ref()?;
    let numbered = link
        .captures_iter(html)
        .filter_map(|capture| capture.get(1))
        .filter_map(|value| value.as_str().parse::<usize>().ok())
        .any(|page| page == next);
    if numbered {
        return Some(next);
    }
    let anchor = PAGER_NEXT.as_ref()?;
    let followed = anchor.find_iter(html).any(|tag| {
        link.captures(tag.as_str())
            .and_then(|capture| capture.get(1))
            .and_then(|value| value.as_str().parse::<usize>().ok())
            .is_some_and(|page| page == next)
    });
    followed.then_some(next)
}

/// URL of a `/schools` listing page.
pub fn listing_page_url(page: usize) -> String {
    if page == 0 {
        SCHOOL_LIST_URL.to_string()
    } else {
        format!("{SCHOOL_LIST_URL}?page={page}")
    }
}

/// URL of a school page.
pub fn school_page_url(slug: &str) -> String {
    format!("{SCHOOL_URL_PREFIX}{slug}")
}

/// The `Administration` grid of a school page, up to the next section heading.
fn administration_block(html: &str) -> Option<&str> {
    let start = html.find("grid--administration")?;
    let tail = html.get(start..)?;
    let end = ["<strong>Conference", "<h2"]
        .iter()
        .filter_map(|marker| tail.find(marker))
        .min()
        .unwrap_or(tail.len());
    tail.get(..end)
}

/// The Administration block as role/name/address entries, in document order.
pub fn parse_admin_entries(html: &str) -> Vec<AdminEntry> {
    let Some(block) = administration_block(html) else {
        return Vec::new();
    };
    let (Some(role_re), Some(name_re), Some(text_re)) = (
        ADMIN_ROLE.as_ref(),
        ADMIN_NAME.as_ref(),
        ADMIN_NAME_TEXT.as_ref(),
    ) else {
        return Vec::new();
    };
    const ITEM_MARKER: &str = "<div class=\"grid__item\">";
    let mut entries: Vec<AdminEntry> = Vec::new();
    for item in block.split(ITEM_MARKER).skip(1) {
        let Some(role) = role_re
            .captures(item)
            .and_then(|capture| capture.get(1))
            .map(|value| clean(value.as_str()).trim_end_matches(':').to_string())
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        let name = name_re
            .captures(item)
            .and_then(|capture| capture.get(1))
            .map(|value| clean(value.as_str()))
            .or_else(|| {
                text_re
                    .captures(item)
                    .and_then(|capture| capture.get(1))
                    .map(|value| clean(value.as_str()))
            })
            .unwrap_or_default();
        entries.push(AdminEntry {
            role,
            name,
            emails: decode_cfemail_fragment(item),
        });
    }
    entries
}

/// Parse a school page: identity, facts and Administration block.
pub fn parse_school_detail(html: &str) -> SchoolDetail {
    let name = PAGE_TITLE
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .map(|value| clean(value.as_str()))
        .filter(|value| !value.is_empty());
    let school_id = GROUP_ID
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().to_string());
    let enrollment = ENROLLMENT
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .and_then(|value| value.as_str().replace(',', "").parse::<u32>().ok())
        .filter(|value| *value > 0 && *value < 100_000);
    let website = WEBSITE
        .as_ref()
        .and_then(|pattern| pattern.captures(html))
        .and_then(|capture| capture.get(1))
        .map(|value| value.as_str().trim().to_string())
        .filter(|value| value.starts_with("http://") || value.starts_with("https://"));
    SchoolDetail {
        name,
        school_id,
        enrollment,
        website,
        admin: parse_admin_entries(html),
    }
}
