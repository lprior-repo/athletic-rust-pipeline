//! The provider's two published schedules: their page URL, the shape of one schedule row, and
//! the parser that reads a rendered schedule table into those rows.

use census_domain::model::Sport;
use anyhow::Result;
use regex::Regex;
use std::sync::LazyLock;

use super::BASE;

/// The two schedules this adapter walks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleSport {
    Track,
    CrossCountry,
}

impl ScheduleSport {
    fn path(self) -> &'static str {
        match self {
            ScheduleSport::Track => "track",
            ScheduleSport::CrossCountry => "xc",
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            ScheduleSport::Track => "track",
            ScheduleSport::CrossCountry => "xc",
        }
    }

    /// Which sport a row of this schedule belongs to, from its month alone.
    pub(super) fn sport_for(self, month: u8) -> Sport {
        match self {
            ScheduleSport::CrossCountry => Sport::CrossCountry,
            // The published "track" schedule opens in the indoor season and runs into the outdoor
            // one; the month is the only field a row carries that separates them.
            ScheduleSport::Track if month <= 3 || month >= 11 => Sport::IndoorTrack,
            ScheduleSport::Track => Sport::OutdoorTrack,
        }
    }
}

/// Schedule page for one sport and season year.
pub fn schedule_url(sport: ScheduleSport, year: i16) -> String {
    format!("{BASE}/sports/{}/{year}/schedule", sport.path())
}

/// One row of a schedule table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MeetRow {
    /// `YYYY-MM-DD`, from the row's own date cell and the month heading above it.
    pub date: String,
    pub name: String,
    pub location: String,
    /// Provider key: the last segment of the row's `/links/<slug>` target, when it has one.
    pub slug: Option<String>,
    /// The link's own `aria-label`, which repeats month, day, name and venue.
    pub aria_label: Option<String>,
}

static ROW: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r"(?is)<tr\b[^>]*>.*?</tr>"));
static DATE_CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<td[^>]*class="[^"]*date[^"]*"[^>]*>(.*?)</td>"#));
static NAME_CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<td[^>]*class="[^"]*awayteam[^"]*"[^>]*>(.*?)</td>"#));
static VENUE_CELL: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<td[^>]*class="[^"]*hometeam[^"]*"[^>]*>(.*?)</td>"#));
static TITLE: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<span[^>]*title="([^"]*)""#));
static LINK: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]*href="/links/([^"/?#]+)""#));
static ARIA: LazyLock<Result<Regex, regex::Error>> =
    LazyLock::new(|| Regex::new(r#"(?is)<a[^>]*aria-label="([^"]*)""#));
static TAGS: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(?is)<[^>]*>"));
static DAY: LazyLock<Result<Regex, regex::Error>> = LazyLock::new(|| Regex::new(r"(\d{1,2})"));

/// The compiled pattern in one slot above, or the compile error it carries.
///
/// Every pattern here is a literal, so a failure is a programming mistake rather than something a
/// page can cause; it is reported as an error instead of panicking at first use.
fn compiled(slot: &'static LazyLock<Result<Regex, regex::Error>>) -> Result<&'static Regex> {
    slot.as_ref().map_err(|e| anyhow::anyhow!("regex: {e}"))
}

fn row() -> Result<&'static Regex> {
    compiled(&ROW)
}

fn date_cell() -> Result<&'static Regex> {
    compiled(&DATE_CELL)
}

fn name_cell() -> Result<&'static Regex> {
    compiled(&NAME_CELL)
}

fn venue_cell() -> Result<&'static Regex> {
    compiled(&VENUE_CELL)
}

fn title() -> Result<&'static Regex> {
    compiled(&TITLE)
}

fn link() -> Result<&'static Regex> {
    compiled(&LINK)
}

fn aria() -> Result<&'static Regex> {
    compiled(&ARIA)
}

fn tags() -> Result<&'static Regex> {
    compiled(&TAGS)
}

fn day() -> Result<&'static Regex> {
    compiled(&DAY)
}

const MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

/// Read every competition row from one schedule page.
///
/// Rows are published under month headings; the heading is the only month a row carries, so it is
/// tracked as the table is walked. A row without a date, a name or a venue is skipped: the platform
/// cannot mint an identity for it.
pub fn schedule_rows(body: &str, year: i16) -> Result<Vec<MeetRow>> {
    let mut rows = Vec::new();
    let mut month: Option<u8> = None;
    let day_pattern = day()?;
    for found in row()?.find_iter(body) {
        let row = found.as_str();
        // The month heading is a row of its own (`<tr class="month-title …">`) whose text is the
        // month name; every competition row beneath it belongs to that month until the next heading.
        if row.contains("month-title") {
            month = month_from_text(&tags()?.replace_all(row, " "));
            continue;
        }
        if !row.contains("event-row") {
            continue;
        }
        let Some(month) = month else {
            continue;
        };
        let Some(day) = date_cell()?
            .captures(row)
            .and_then(|cell| cell.get(1))
            .and_then(|cell| day_pattern.captures(cell.as_str()))
            .and_then(|day| day.get(1))
            .and_then(|day| day.as_str().parse::<u8>().ok())
        else {
            continue;
        };
        let name = cell_text(name_cell()?, row)?;
        let location = cell_text(venue_cell()?, row)?;
        if name.is_empty() || location.is_empty() {
            continue;
        }
        rows.push(MeetRow {
            date: format!("{year:04}-{month:02}-{day:02}"),
            name,
            location,
            slug: link()?
                .captures(row)
                .and_then(|capture| capture.get(1))
                .map(|slug| slug.as_str().to_string())
                .filter(|slug| !slug.is_empty()),
            aria_label: aria()?
                .captures(row)
                .and_then(|capture| capture.get(1))
                .map(|label| normalize_whitespace(label.as_str()))
                .filter(|label| !label.is_empty()),
        });
    }
    Ok(rows)
}

/// A table cell's label: its `<span title="…">` when it has one, otherwise its stripped text.
fn cell_text(cell: &Regex, row: &str) -> Result<String> {
    let Some(captures) = cell.captures(row) else {
        return Ok(String::new());
    };
    let inner = captures
        .get(1)
        .map(|cell| cell.as_str())
        .unwrap_or_default();
    if let Some(title) = title()?.captures(inner).and_then(|title| title.get(1)) {
        return Ok(normalize_whitespace(title.as_str()));
    }
    Ok(normalize_whitespace(&tags()?.replace_all(inner, " ")))
}

fn month_from_text(text: &str) -> Option<u8> {
    let text = text.trim().to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|month| text.starts_with(&month.to_ascii_lowercase()))
        .and_then(|index| index.checked_add(1))
        // `MONTHS` holds twelve names, so the 1-based month always fits a `u8`.
        .and_then(|month| u8::try_from(month).ok())
}

fn normalize_whitespace(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}
