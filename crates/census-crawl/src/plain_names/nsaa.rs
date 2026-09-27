
use super::parse::{clean_text, nonempty, without_comments};
use super::NSAA_FORM_URL;
use crate::{CrawlError, CrawlResult};
use census_domain::model::{Gender, Sport};
use regex::Regex;
use std::sync::LazyLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsaaRow {
    SportCoach { sport: Sport, gender: Gender },
    AthleticDirector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsaaRole {
    pub label: String,
    pub name: String,
    pub co_op: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsaaSchool {
    pub name: String,
    pub city: Option<String>,
    pub enrollment: Option<u32>,
    pub homepage: Option<String>,
    pub roles: Vec<NsaaRole>,
}

static NSAA_BLOCK_HEADING: &str = r#"<h1 class="mt-3">"#;

static NSAA_ROW_REGEX: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| {
    Regex::new(
        r#"(?s)<tr([^>]*)>\s*<td scope='row'>(.*?)</td>\s*<td scope='row'>(.*?)</td>\s*</tr>"#,
    )
});
static NSAA_CITY_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)([A-Za-z][A-Za-z .'\-]*?)\s*,\s*NE\s+\d{5}"));
static NSAA_ENROLLMENT_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?i)Enrollment:\s*([\d,]+)"));
static NSAA_HOMEPAGE_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?i)Homepage:\s*<a[^>]*href="([^"]+)""#));

fn nsaa_row_regex() -> CrawlResult<&'static Regex> {
    NSAA_ROW_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "nsaa directory row",
            source: source.clone(),
        })
}

fn nsaa_city_regex() -> CrawlResult<&'static Regex> {
    NSAA_CITY_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "nsaa city",
            source: source.clone(),
        })
}

fn nsaa_enrollment_regex() -> CrawlResult<&'static Regex> {
    NSAA_ENROLLMENT_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "nsaa enrollment",
            source: source.clone(),
        })
}

fn nsaa_homepage_regex() -> CrawlResult<&'static Regex> {
    NSAA_HOMEPAGE_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "nsaa url",
            source: source.clone(),
        })
}

pub(super) const NSAA_ALL_SCHOOLS: &str = "View all schools";

static NSAA_OPTION_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<option([^>]*)>(.*?)</option>"));

fn nsaa_option_regex() -> CrawlResult<&'static Regex> {
    NSAA_OPTION_REGEX
        .as_ref()
        .map_err(|source| CrawlError::RegexInit {
            pattern: "nsaa option",
            source: source.clone(),
        })
}

pub fn nsaa_school_url(name: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(name.as_bytes()).collect();
    format!("{NSAA_FORM_URL}?session=&school={encoded}")
}

pub fn parse_nsaa_school_names(html: &str) -> CrawlResult<Vec<String>> {
    let html = without_comments(html)?;
    let html: &str = &html;
    let mut names: Vec<String> = Vec::new();
    for capture in nsaa_option_regex()?.captures_iter(html) {
        let (Some(attributes), Some(value)) = (capture.get(1), capture.get(2)) else {
            continue;
        };
        if attributes
            .as_str()
            .to_ascii_lowercase()
            .contains("disabled")
        {
            continue;
        }
        let name = clean_text(value.as_str())?;
        if name.is_empty() || name == NSAA_ALL_SCHOOLS || names.contains(&name) {
            continue;
        }
        names.push(name);
    }
    Ok(names)
}

fn nsaa_roles(block: &str) -> CrawlResult<Vec<NsaaRole>> {
    let mut roles: Vec<NsaaRole> = Vec::new();
    for capture in nsaa_row_regex()?.captures_iter(block) {
        let (Some(attributes), Some(label), Some(value)) =
            (capture.get(1), capture.get(2), capture.get(3))
        else {
            continue;
        };
        let label = clean_text(label.as_str())?;
        let value = clean_text(value.as_str())?;
        if label.is_empty() || value.is_empty() {
            continue;
        }
        roles.push(NsaaRole {
            label,
            name: value,
            co_op: attributes.as_str().contains("table-info"),
        });
    }
    Ok(roles)
}

pub fn parse_nsaa_directory(html: &str) -> CrawlResult<Vec<NsaaSchool>> {
    let html = without_comments(html)?;
    let html: &str = &html;
    let mut schools: Vec<NsaaSchool> = Vec::new();
    for block in html.split(NSAA_BLOCK_HEADING).skip(1) {
        let Some((raw_name, rest)) = block.split_once("</h1>") else {
            continue;
        };
        let name = clean_text(raw_name)?;
        if name.is_empty() {
            continue;
        }
        let roles = nsaa_roles(rest)?;
        let city = match nsaa_city_regex()?
            .captures(rest)
            .and_then(|capture| capture.get(1))
        {
            Some(city) => nonempty(&clean_text(city.as_str())?),
            None => None,
        };
        let enrollment = nsaa_enrollment_regex()?
            .captures(rest)
            .and_then(|capture| capture.get(1))
            .map(|digits| {
                digits
                    .as_str()
                    .chars()
                    .filter(char::is_ascii_digit)
                    .collect::<String>()
            })
            .and_then(|digits| digits.parse().ok());
        let homepage = nsaa_homepage_regex()?
            .captures(rest)
            .and_then(|capture| capture.get(1))
            .and_then(|href| nonempty(href.as_str()));
        schools.push(NsaaSchool {
            name,
            city,
            enrollment,
            homepage,
            roles,
        });
    }
    Ok(schools)
}
