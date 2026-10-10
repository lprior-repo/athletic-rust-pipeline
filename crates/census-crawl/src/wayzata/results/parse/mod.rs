use crate::{CrawlError, CrawlResult};
use std::sync::LazyLock;

mod pattern;

/// Performance record extracted from a Wayzata result page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Performance {
    pub event: String,
    pub athlete: String,
    pub team: String,
    pub mark: String,
}

/// Parse individual performances from a Wayzata result page body.
pub fn parse_performances(body: &str) -> CrawlResult<Vec<Performance>> {
    super::super::budget::check(
        "Wayzata result body bytes",
        body.len(),
        super::super::budget::MAX_PAGE_BYTES,
    )?;
    let results = results_body(body)?;
    let Some(body) = results else {
        return Ok(Vec::new());
    };
    let rows = pattern::row()?.find_iter(body);
    let mut performances = Vec::new();
    for row_match in rows {
        let row_text = row_match.as_str();
        let Some(performance) = parse_performance(row_text)? else {
            continue;
        };
        performances.push(performance);
    }
    Ok(performances)
}

fn parse_performance(row: &str) -> CrawlResult<Option<Performance>> {
    let event = pattern::event_cell()?
        .captures(row)
        .and_then(|c| pattern::value(&c))
        .map(|m| normalize(m.as_str()))
        .unwrap_or_default();
    let athlete = pattern::athlete_cell()?
        .captures(row)
        .and_then(|c| pattern::value(&c))
        .map(|m| normalize(m.as_str()))
        .unwrap_or_default();
    let team = pattern::team_cell()?
        .captures(row)
        .and_then(|c| pattern::value(&c))
        .map(|m| normalize(m.as_str()))
        .unwrap_or_default();
    let mark = pattern::mark_cell()?
        .captures(row)
        .and_then(|c| pattern::value(&c))
        .map(|m| normalize(m.as_str()))
        .unwrap_or_default();
    if athlete.is_empty() && mark.is_empty() {
        return Ok(None);
    }
    Ok(Some(Performance {
        event,
        athlete,
        team,
        mark,
    }))
}

fn normalize(text: &str) -> String {
    let text = text.replace('\n', " ");
    let text = text.replace('\r', " ");
    let mut parts = text.split(' ');
    let mut result = String::new();
    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if !result.is_empty() {
            result.push(' ');
        }
        result.push_str(part);
    }
    result
}

fn results_body(body: &str) -> CrawlResult<Option<&str>> {
    let results_table = pattern::results_table_open()?
        .find(body)
        .map(|m| m.start());
    let Some(start) = results_table else {
        return Ok(None);
    };
    let table_close = pattern::table_close()?
        .find(&body[start..])
        .map(|m| m.end() + start);
    Ok(table_close.map(|end| &body[start..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_performance() {
        let html = r#"
        <table class="results">
            <tr>
                <td class="event">100 Meter Dash</td>
                <td class="athlete">John Smith</td>
                <td class="team">Wayzata</td>
                <td class="mark">11.23</td>
            </tr>
            <tr>
                <td class="event">200 Meter Dash</td>
                <td class="athlete">Jane Doe</td>
                <td class="team">Wayzata</td>
                <td class="mark">23.45</td>
            </tr>
        </table>"#;
        let performances = parse_performances(html).unwrap();
        assert_eq!(performances.len(), 2);
        assert_eq!(performances[0].event, "100 Meter Dash");
        assert_eq!(performances[0].athlete, "John Smith");
        assert_eq!(performances[0].team, "Wayzata");
        assert_eq!(performances[0].mark, "11.23");
    }

    #[test]
    fn test_parse_empty_body() {
        let performances = parse_performances("").unwrap();
        assert_eq!(performances.len(), 0);
    }
}