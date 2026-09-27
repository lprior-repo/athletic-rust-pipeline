use super::html::text_of;
use census_domain::model::SchoolYear;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PublishedDate {
    pub raw: String,
    pub iso: String,
    pub year: i16,
    pub month: u8,
    pub day: u8,
}

impl PublishedDate {
    pub fn school_year(&self) -> Option<SchoolYear> {
        SchoolYear::containing(self.year, self.month)
    }
}

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

fn month_number(name: &str) -> Option<u8> {
    let lowered = name.trim().to_ascii_lowercase();
    MONTH_ABBREVIATIONS
        .iter()
        .position(|abbreviation| lowered.starts_with(abbreviation))
        .and_then(|index| index.checked_add(1))
        .and_then(|month| u8::try_from(month).ok())
}

const MONTH_ABBREVIATIONS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];
