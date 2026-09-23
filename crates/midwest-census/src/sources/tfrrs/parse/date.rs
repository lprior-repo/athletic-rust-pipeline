//! The date a row's `Meet Date` cell publishes.
//!
//! A list states no season of its own, so its rows' dates are the only column a mark can be
//! dated by — and the school year a grade belongs to is the one that date's month falls in.

use super::html::text_of;
use census_domain::model::SchoolYear;

/// A date as the list publishes it (`Mar 28, 2026`), kept both verbatim and normalized.
///
/// The school year a grade belongs to is decided by the month (`Aug 1` boundary, see
/// [`SchoolYear::containing`]), so the parse keeps the two parts rather than only a formatted string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedDate {
    /// The published text, verbatim.
    pub raw: String,
    /// `YYYY-MM-DD`.
    pub iso: String,
    pub year: i16,
    pub month: u8,
    pub day: u8,
}

impl PublishedDate {
    /// The school year this date falls in, or `None` when the published year is outside the window a
    /// season may open in: [`published_date`] admits four-digit years the domain does not, and such a
    /// date is dropped rather than filed under a year no source published.
    pub fn school_year(&self) -> Option<SchoolYear> {
        SchoolYear::containing(self.year, self.month)
    }
}

/// One `Meet Date` cell's published date, or `None` when the cell does not hold one this reader can
/// place on a calendar. The cell's markup is stripped here, so a caller may pass the cell as served.
///
/// Month names are matched by their first three letters, which covers both the abbreviation the
/// list rows use (`Mar 28, 2026`) and a spelled-out name (`March 28, 2026`, `Sept 12, 2026`). A
/// published day range (`June 20-21, 2026`) dates the meet by its first day, which is the day the
/// crate's meet identity uses everywhere else.
pub fn published_date(text: &str) -> Option<PublishedDate> {
    let raw = text_of(text);
    let mut tokens = raw.split(' ').filter(|token| !token.is_empty());
    let month = month_number(tokens.next()?)?;
    let day_token = tokens.next()?.trim_end_matches(',');
    let day: String = day_token.chars().take_while(char::is_ascii_digit).collect();
    let day: u8 = day.parse().ok()?;
    let year: i16 = tokens.next()?.trim_end_matches(',').parse().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || !(1900..=2999).contains(&year) {
        return None;
    }
    Some(PublishedDate {
        raw,
        iso: format!("{year:04}-{month:02}-{day:02}"),
        year,
        month,
        day,
    })
}

/// The month a published name opens with (`Jan`, `March`, `Sept`).
fn month_number(name: &str) -> Option<u8> {
    let lowered = name.trim().to_ascii_lowercase();
    MONTH_ABBREVIATIONS
        .iter()
        .position(|abbreviation| lowered.starts_with(abbreviation))
        .and_then(|index| index.checked_add(1))
        .and_then(|month| u8::try_from(month).ok())
}

/// The three-letter prefixes of the published month names, in calendar order.
const MONTH_ABBREVIATIONS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
