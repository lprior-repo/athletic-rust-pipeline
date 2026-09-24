//! Regex analysis of one response's rows: which athlete links a row carries and what is wrong with
//! them.

use regex::Regex;

use crate::links::link_class;

/// One row's athlete links, in document order, with the class of each.
pub(crate) struct RowAnalysis {
    /// Every athlete href the row carried (all classes).
    pub(crate) athlete_hrefs: Vec<String>,
    /// The hrefs whose class is `empty_id` or `other_noncanonical`.
    pub(crate) noncanonical: Vec<String>,
    /// The sport tokens (`tf` / `xc`) of the canonical sported hrefs.
    pub(crate) sported: Vec<String>,
    /// The row's issue messages, in the order the checks run.
    pub(crate) issues: Vec<String>,
    /// Whether the row is a same-sport candidate with no issue.
    pub(crate) candidate: bool,
}

/// The compiled patterns every page and row is tested against.
pub(crate) struct Regexes {
    /// Splits one response's `results` string into rows.
    pub(crate) tr: Regex,
    href: Regex,
    athlete: Regex,
    sported: Regex,
    sportless: Regex,
    empty_id: Regex,
    name: Regex,
}

/// The compiled patterns every row is tested against, bundled so `analyse_row` takes one argument
/// for them instead of six.
pub(crate) struct RowRegexes<'a> {
    href: &'a Regex,
    athlete: &'a Regex,
    sported: &'a Regex,
    sportless: &'a Regex,
    empty_id: &'a Regex,
    name: &'a Regex,
}

impl Regexes {
    /// Compile the seven patterns the page and row analysis runs on.
    pub(crate) fn compile() -> Result<Self, regex::Error> {
        Ok(Self {
            tr: Regex::new(r"<tr[ >]")?,
            href: Regex::new(r#"href=["']([^"']*)["']"#)?,
            athlete: Regex::new(r"/athlete/")?,
            sported: Regex::new(r"^/athlete/(\d+)/(track-and-field|cross-country)(/all)?/?$")?,
            sportless: Regex::new(r"^/athlete/(\d+)/?$")?,
            empty_id: Regex::new(r"^/athlete//")?,
            name: Regex::new(
                r#"href=["']/athlete/\d+/(?:track-and-field|cross-country)(?:/all)?/?["'][^>]*>([^<]*)"#,
            )?,
        })
    }

    /// The borrow bundle the row analysis takes.
    pub(crate) fn for_rows(&self) -> RowRegexes<'_> {
        RowRegexes {
            href: &self.href,
            athlete: &self.athlete,
            sported: &self.sported,
            sportless: &self.sportless,
            empty_id: &self.empty_id,
            name: &self.name,
        }
    }
}

/// One href with its class token, and the sport token for the canonical sported forms.
type AthleteLink = (String, &'static str, Option<&'static str>);

/// Every href in the row, in document order.
fn hrefs_of(row_html: &str, href_re: &Regex) -> Vec<String> {
    href_re
        .captures_iter(row_html)
        .filter_map(|m| m.get(1))
        .map(|g| g.as_str().to_string())
        .collect()
}

/// The hrefs that point at an athlete, each with its class.
fn athlete_links(hrefs: &[String], re: &RowRegexes<'_>) -> Vec<AthleteLink> {
    hrefs
        .iter()
        .filter(|h| re.athlete.is_match(h))
        .map(|h| {
            let (cls, st) = link_class(h, re.athlete, re.sported, re.sportless, re.empty_id);
            (h.clone(), cls, st)
        })
        .collect()
}

/// The hrefs whose class is `empty_id` or `other_noncanonical`.
fn noncanonical_hrefs(athlete: &[AthleteLink]) -> Vec<String> {
    athlete
        .iter()
        .filter(|(_, cls, _)| *cls == "empty_id" || *cls == "other_noncanonical")
        .map(|(h, _, _)| h.clone())
        .collect()
}

/// The sport tokens of the canonical sported hrefs.
fn sported_tokens(athlete: &[AthleteLink]) -> Vec<String> {
    athlete
        .iter()
        .filter(|(_, cls, _)| *cls == "sported")
        .filter_map(|(_, _, st)| st.map(|s| s.to_string()))
        .collect()
}

/// The row's issue messages: a noncanonical link, another sport, or no display name.
fn row_issues(
    noncanonical: &[String],
    identity: bool,
    selected: bool,
    row_html: &str,
    name_re: &Regex,
) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();
    if !noncanonical.is_empty() {
        issues.push("athlete URL is not canonical".to_string());
    }
    if identity && !selected {
        issues.push("result row belongs to another or unspecified sport".to_string());
    }

    let name_matches: Vec<String> = name_re
        .captures_iter(row_html)
        .filter_map(|m| m.get(1))
        .map(|g| g.as_str().trim().to_string())
        .collect();

    if identity && selected && name_matches.first().is_some_and(|name| name.is_empty()) {
        issues.push("athlete display name is absent or exceeds bound".to_string());
    }
    issues
}

/// Analyse one row: its athlete links, their classes, and the issues that disqualify it.
pub(crate) fn analyse_row(row_html: &str, sport: Option<&str>, re: &RowRegexes<'_>) -> RowAnalysis {
    let hrefs = hrefs_of(row_html, re.href);
    let athlete = athlete_links(&hrefs, re);
    let noncanonical = noncanonical_hrefs(&athlete);
    let sported = sported_tokens(&athlete);
    let identity = athlete
        .iter()
        .any(|(_, cls, _)| *cls == "sported" || *cls == "sportless");
    let selected = match sport {
        Some(s) => sported.iter().any(|t| t == s),
        None => false,
    };
    let issues = row_issues(&noncanonical, identity, selected, row_html, re.name);
    let candidate = identity && selected && issues.is_empty();

    RowAnalysis {
        athlete_hrefs: athlete.into_iter().map(|(h, _, _)| h).collect(),
        noncanonical,
        sported,
        issues,
        candidate,
    }
}
