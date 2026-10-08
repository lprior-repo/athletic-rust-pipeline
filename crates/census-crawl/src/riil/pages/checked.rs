use super::{extract_school_name, parse_coach_row, parse_sport_label, SchoolTable};
use crate::directory::acquisition::text;
use crate::net::FetchOutcome;
use crate::{CrawlError, CrawlResult};

pub(in crate::riil) enum DirectoryRecord {
    School(SchoolTable),
    Coach(super::CoachRow),
    Complete,
}

pub(in crate::riil) fn visit_capture(
    capture: &FetchOutcome,
    mut visit: impl FnMut(&str, CrawlResult<DirectoryRecord>) -> CrawlResult<()>,
) -> CrawlResult<()> {
    let html = match text(capture) {
        Ok(html) => html,
        Err(error) => return visit(&capture.url, Err(error)),
    };
    let opened = html.matches("<details").count();
    if opened == 0 && !published_empty_directory(html) {
        return visit(
            &capture.url,
            Err(malformed(
                &capture.url,
                "response is not a published school directory",
            )),
        );
    }
    let closed = html.trim_end().ends_with("</div>") || html.trim_end().ends_with("</html>");
    let envelope_complete = closed && opened == html.matches("</details>").count();
    html.split("<details")
        .skip(1)
        .enumerate()
        .try_for_each(|(ordinal, section)| {
            visit_school(
                section,
                &format!("{}#school={ordinal}", capture.url),
                envelope_complete,
                &mut visit,
            )
        })?;
    if opened > 0 && !closed {
        visit(
            &capture.url,
            Err(malformed(&capture.url, "directory envelope is incomplete")),
        )?;
    }
    if opened != html.matches("</details>").count() {
        visit(
            &capture.url,
            Err(malformed(
                &capture.url,
                "unclosed or unmatched school section",
            )),
        )?;
    }
    Ok(())
}

fn visit_school(
    section: &str,
    locator: &str,
    envelope_complete: bool,
    visit: &mut impl FnMut(&str, CrawlResult<DirectoryRecord>) -> CrawlResult<()>,
) -> CrawlResult<()> {
    let name = extract_school_name(section);
    if name.is_empty() {
        return visit(
            locator,
            Err(malformed(locator, "school section has no published name")),
        );
    }
    visit(
        locator,
        Ok(DirectoryRecord::School(SchoolTable {
            name,
            coach_rows: Vec::new(),
        })),
    )?;
    let mut complete =
        envelope_complete && section.contains("</details>") && section.contains("<table");
    section
        .split_inclusive("</tr>")
        .filter(|row| row.contains("<tr"))
        .enumerate()
        .try_for_each(|(ordinal, row)| {
            let row_locator = format!("{locator}&row={ordinal}");
            match validate_row(row, &row_locator) {
                Ok(()) => {
                    if let Some(coach) = parse_coach_row(row) {
                        visit(&row_locator, Ok(DirectoryRecord::Coach(coach)))?;
                    }
                }
                Err(error) => {
                    complete = false;
                    visit(&row_locator, Err(error))?;
                }
            }
            Ok::<_, CrawlError>(())
        })?;
    if section.matches("<table").count() != section.matches("</table>").count()
        || section.matches("<tr").count() != section.matches("</tr>").count()
    {
        complete = false;
    }
    if complete {
        visit(locator, Ok(DirectoryRecord::Complete))
    } else {
        visit(
            locator,
            Err(malformed(locator, "incomplete staff table or row")),
        )
    }
}

fn validate_row(row: &str, locator: &str) -> CrawlResult<()> {
    let cells = row.matches("<td").count();
    if !row.ends_with("</tr>") || cells != row.matches("</td>").count() {
        return Err(malformed(locator, "staff row has an incomplete cell"));
    }
    let sport = super::parse_coach_name(super::extract_td_text(row, 0));
    let role = super::parse_coach_name(super::extract_td_text(row, 1));
    if parse_sport_label(&sport).is_some() && role.contains("Head Coach") && cells < 4 {
        return Err(malformed(
            locator,
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

fn malformed(url: &str, detail: impl Into<String>) -> CrawlError {
    CrawlError::Schema {
        url: url.to_owned(),
        detail: detail.into(),
    }
}
