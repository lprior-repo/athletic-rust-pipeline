use super::{extract_school_name, parse_directory, parse_sport_label};
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};

pub(in crate::riil) fn parse_capture(
    capture: &FetchOutcome,
) -> CrawlResult<Vec<super::SchoolTable>> {
    let html = std::str::from_utf8(&capture.body)
        .map_err(|error| malformed(capture, format!("invalid UTF-8: {error}")))?;
    let opened = html.matches("<details").count();
    if opened != html.matches("</details>").count() {
        return Err(malformed(capture, "unclosed or unmatched school section"));
    }
    html.split("<details").skip(1).try_for_each(|section| {
        let (section, _) = section
            .split_once("</details>")
            .ok_or_else(|| malformed(capture, "unclosed school section"))?;
        validate_school(section, capture)
    })?;
    let tables = parse_directory(html);
    if tables.is_empty() && !published_empty_directory(html) {
        return Err(malformed(
            capture,
            "response is not a published school directory",
        ));
    }
    Ok(tables)
}

fn validate_school(section: &str, capture: &FetchOutcome) -> CrawlResult<()> {
    if extract_school_name(section).is_empty() {
        return Err(malformed(capture, "school section has no published name"));
    }
    if section.matches("<table").count() != section.matches("</table>").count() {
        return Err(malformed(
            capture,
            "school section has an incomplete staff table",
        ));
    }
    if section.matches("<tr").count() != section.matches("</tr>").count() {
        return Err(malformed(
            capture,
            "school section has an incomplete staff row",
        ));
    }
    section
        .split("</tr>")
        .filter(|row| row.contains("<tr"))
        .try_for_each(|row| validate_row(row, capture))
}

fn validate_row(row: &str, capture: &FetchOutcome) -> CrawlResult<()> {
    let cells = row.matches("<td").count();
    if cells != row.matches("</td>").count() {
        return Err(malformed(capture, "staff row has an incomplete cell"));
    }
    let sport = super::parse_coach_name(super::extract_td_text(row, 0));
    let role = super::parse_coach_name(super::extract_td_text(row, 1));
    if parse_sport_label(&sport).is_some() && role.contains("Head Coach") && cells < 4 {
        return Err(malformed(
            capture,
            "target appointment row lacks published staff columns",
        ));
    }
    Ok(())
}

fn published_empty_directory(html: &str) -> bool {
    ["id=\"BodyWrapper\"", "id='BodyWrapper'"]
        .into_iter()
        .filter_map(|marker| html.split_once(marker))
        .filter_map(|(_, body)| body.split_once("</div>"))
        .filter_map(|(body, _)| body.split_once("<ul"))
        .filter_map(|(_, list)| list.split_once('>'))
        .filter_map(|(_, list)| list.split_once("</ul>"))
        .any(|(entries, _)| entries.trim().is_empty())
}

fn malformed(capture: &FetchOutcome, detail: impl Into<String>) -> CrawlError {
    CrawlError::Schema {
        url: capture.url.clone(),
        detail: detail.into(),
    }
}
