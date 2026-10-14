#[cfg(test)]
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

#[cfg(test)]
pub fn published_date(text: &str) -> Option<PublishedDate> {
    from_label(text_of(text))
}

pub(super) fn from_label(raw: String) -> Option<PublishedDate> {
    let mut tokens = raw.split_whitespace();
    let month = month_number(tokens.next()?)?;
    let day_token = tokens.next()?;
    let day_token = day_token.strip_suffix(',').map_or(day_token, |value| value);
    let year_token = tokens.next()?;
    if tokens.next().is_some()
        || !(1..=2).contains(&day_token.len())
        || !day_token.bytes().all(|byte| byte.is_ascii_digit())
        || year_token.len() != 4
        || !year_token.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let day: u8 = day_token.parse().ok()?;
    let year: i16 = year_token.parse().ok()?;
    if !(1900..=2999).contains(&year) {
        return None;
    }
    let date = chrono::NaiveDate::from_ymd_opt(i32::from(year), u32::from(month), u32::from(day))?;
    Some(PublishedDate {
        raw,
        iso: date.to_string(),
        year,
        month,
        day,
    })
}

fn month_number(name: &str) -> Option<u8> {
    let name = name.strip_suffix('.').map_or(name, |value| value);
    if name.eq_ignore_ascii_case("sept") {
        return Some(9);
    }
    MONTH_NAMES
        .iter()
        .position(|(short, full)| name.eq_ignore_ascii_case(short) || name.eq_ignore_ascii_case(full))
        .and_then(|index| index.checked_add(1))
        .and_then(|month| u8::try_from(month).ok())
}

const MONTH_NAMES: [(&str, &str); 12] = [
    ("jan", "january"),
    ("feb", "february"),
    ("mar", "march"),
    ("apr", "april"),
    ("may", "may"),
    ("jun", "june"),
    ("jul", "july"),
    ("aug", "august"),
    ("sep", "september"),
    ("oct", "october"),
    ("nov", "november"),
    ("dec", "december"),
];
