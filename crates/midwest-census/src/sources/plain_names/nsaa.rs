//! Nebraska (NSAA): the directory screen's option list and its per-school `<h1>` blocks.
//!
//! `GET https://secure.nsaahome.org/nsaaforms/direxportscreen.php` returns the request form plus
//! the `<option>` list of all **312** member schools; `?session=&school=<name>` returns one
//! school's full record. NSAA publishes no numeric school id, so the published school name *is*
//! the provider key space.

use super::parse::{clean_text, nonempty, without_comments};
use super::NSAA_FORM_URL;
use crate::model::{Gender, Sport};
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

/// What one NSAA directory row describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NsaaRow {
    /// A sport row: the school's head coach for that sport and gender side.
    SportCoach { sport: Sport, gender: Gender },
    /// A school-wide athletic/activities-director row.
    AthleticDirector,
}

/// One published row of a school's staff/coach table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsaaRole {
    /// Published row label, e.g. `Track & Field (Girls)` or `AD Secretary`.
    pub label: String,
    /// Published name cell, verbatim (still carrying any co-op annotation).
    pub name: String,
    /// The row carries the directory's co-op highlight (`class="table-info"`).
    pub co_op: bool,
}

/// One NSAA member school: metadata plus every published staff/coach row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NsaaSchool {
    /// Published school name — NSAA's only identity key.
    pub name: String,
    /// City from the `City, NE <zip>` line.
    pub city: Option<String>,
    /// `Enrollment:` figure.
    pub enrollment: Option<u32>,
    /// `Homepage:` link.
    pub homepage: Option<String>,
    /// Staff/coach rows in published order.
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

fn nsaa_row_regex() -> Result<&'static Regex> {
    NSAA_ROW_REGEX
        .as_ref()
        .map_err(|e| anyhow::anyhow!("nsaa directory row regex: {e}"))
}

fn nsaa_city_regex() -> Result<&'static Regex> {
    NSAA_CITY_REGEX
        .as_ref()
        .map_err(|e| anyhow::anyhow!("nsaa city regex: {e}"))
}

fn nsaa_enrollment_regex() -> Result<&'static Regex> {
    NSAA_ENROLLMENT_REGEX
        .as_ref()
        .map_err(|e| anyhow::anyhow!("nsaa enrollment regex: {e}"))
}

fn nsaa_homepage_regex() -> Result<&'static Regex> {
    NSAA_HOMEPAGE_REGEX
        .as_ref()
        .map_err(|e| anyhow::anyhow!("nsaa url regex: {e}"))
}

/// The directory screen's bulk sentinel: renders every school, so it is never a school name.
pub(super) const NSAA_ALL_SCHOOLS: &str = "View all schools";

static NSAA_OPTION_REGEX: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?s)<option([^>]*)>(.*?)</option>"));

fn nsaa_option_regex() -> Result<&'static Regex> {
    NSAA_OPTION_REGEX
        .as_ref()
        .map_err(|e| anyhow::anyhow!("nsaa option regex: {e}"))
}

/// One-school request URL, e.g. `…/direxportscreen.php?session=&school=Adams+Central` (form-urlencoded
/// by the `url` crate — `+` for space, hyphens literal; the server decodes both forms identically).
///
/// This is the route the adapter walks: the screen's bulk form renders all 312 schools in one body
/// and needs ~49 s to do it, while this single-school view answers in ~0.3 s.
pub fn nsaa_school_url(name: &str) -> String {
    let encoded: String = url::form_urlencoded::byte_serialize(name.as_bytes()).collect();
    format!("{NSAA_FORM_URL}?session=&school={encoded}")
}

/// The 312 member-school names from the directory form's `<option>` list.
///
/// The list carries two non-school entries — a `disabled` placeholder and the `View all schools`
/// bulk sentinel — and no `value` attributes, so the option text is the key space.
pub fn parse_nsaa_school_names(html: &str) -> Result<Vec<String>> {
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

/// Every `<tr>` staff/coach row of one school block, in published order.
fn nsaa_roles(block: &str) -> Result<Vec<NsaaRole>> {
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

/// Parse a directory response into one [`NsaaSchool`] per `<h1 class="mt-3">` block.
///
/// One school view carries a single block; the screen's bulk form carries all 312 in the same
/// markup, so the same parser serves both.
pub fn parse_nsaa_directory(html: &str) -> Result<Vec<NsaaSchool>> {
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
