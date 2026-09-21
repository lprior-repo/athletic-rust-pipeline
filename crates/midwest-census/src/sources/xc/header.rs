//! The file's own name and date: the first heading line that is neither stamp nor credit, and the
//! first recognised date, with a walk of the page footer when the header carries none.
use super::patterns::{date_named, date_slash, page_stamp};

/// Meet name and date from the file's own header lines.
///
/// The name is the first line that is not a page stamp, a date, a rule or a "results provided by"
/// credit; the date is the first recognised date, because the state meet prints it under the course
/// name, AccuRace prints it under the host school, and the Chrome-printed sectionals print it in
/// the page footer (`October 24, 2025        Page 4 of 4`).
pub(super) fn header(lines: &[String]) -> Option<(String, Option<String>)> {
    let mut name: Option<String> = None;
    let mut date: Option<String> = None;
    for line in lines.iter().take(15) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if date.is_none() {
            date = parse_date(trimmed);
        }
        if name.is_none() {
            let cleaned = page_stamp().ok()?.replace(trimmed, "");
            let cleaned = date_named().ok()?.replace(&cleaned, "");
            let cleaned = date_slash().ok()?.replace(&cleaned, "");
            let candidate = cleaned.trim();
            let lowered = candidate.to_ascii_lowercase();
            if candidate.len() >= 6
                && !candidate.chars().all(|ch| ch.is_ascii_digit())
                && !lowered.starts_with("results provided")
                && !lowered.starts_with("www.")
                && !lowered.starts_with("hosted by")
                && !lowered.contains("=====")
                && candidate.chars().any(char::is_alphabetic)
            {
                name = Some(candidate.to_string());
            }
        }
        if name.is_some() && date.is_some() {
            break;
        }
    }
    // Some sectional files publish the date only in a page footer, so the fallback walks the file.
    if date.is_none() {
        date = lines.iter().rev().find_map(|line| parse_date(line.trim()));
    }
    Some((name?, date))
}

const MONTHS: [&str; 12] = [
    "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
];

fn parse_date(line: &str) -> Option<String> {
    if let Some(captures) = date_named().ok()?.captures(line) {
        let month_token = captures.get(1)?.as_str().to_ascii_lowercase();
        let month = MONTHS
            .iter()
            .position(|month| month_token.starts_with(month))?
            .checked_add(1)?;
        return Some(format!(
            "{:04}-{:02}-{:02}",
            captures.get(3)?.as_str().parse::<i32>().ok()?,
            month,
            captures.get(2)?.as_str().parse::<u32>().ok()?
        ));
    }
    let captures = date_slash().ok()?.captures(line)?;
    Some(format!(
        "{:04}-{:02}-{:02}",
        captures.get(3)?.as_str().parse::<i32>().ok()?,
        captures.get(1)?.as_str().parse::<u32>().ok()?,
        captures.get(2)?.as_str().parse::<u32>().ok()?
    ))
}
