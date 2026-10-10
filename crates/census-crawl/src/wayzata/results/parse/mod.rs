use crate::CrawlResult;
use regex::Regex;

mod pattern;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Performance {
    pub event: String,
    pub athlete: String,
    pub team: String,
    pub mark: String,
}

pub fn parse_performances(body: &str) -> CrawlResult<Vec<Performance>> {
    super::super::budget::check(
        "Wayzata result body bytes",
        body.len(),
        super::super::budget::MAX_PAGE_BYTES,
    )?;
    let Some(results) = results_body(body)? else {
        return Ok(Vec::new());
    };
    let mut performances = Vec::new();
    for found in pattern::row()?.find_iter(results) {
        let Some(performance) = parse_performance(found.as_str())? else {
            continue;
        };
        performances.push(performance);
    }
    Ok(performances)
}

fn parse_performance(row: &str) -> CrawlResult<Option<Performance>> {
    let event = cell(pattern::event_cell(), row)?;
    let athlete = cell(pattern::athlete_cell(), row)?;
    let team = cell(pattern::team_cell(), row)?;
    let mark = cell(pattern::mark_cell(), row)?;
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

fn cell(source: CrawlResult<&'static Regex>, row: &str) -> CrawlResult<String> {
    let regex = source?;
    let Some(found) = regex
        .captures(row)
        .and_then(|captures| pattern::value(&captures))
    else {
        return Ok(String::new());
    };
    Ok(normalize(found.as_str()))
}

fn normalize(text: &str) -> String {
    let text = text.replace('\n', " ");
    let text = text.replace('\r', " ");
    let parts = text.split(' ');
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
    let table_open = pattern::results_table_open()?
        .find(body)
        .map(|found| found.start());
    let Some(start) = table_open else {
        return Ok(None);
    };
    let table_end = pattern::table_close()?
        .find(&body[start..])
        .map(|found| found.end() + start);
    Ok(table_end.map(|end| &body[start..end]))
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

    const RESULTS_TABLE: &str = r#"
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

    #[test]
    fn a_results_table_projects_event_athlete_team_and_mark() -> TestResult {
        let performances = parse_performances(RESULTS_TABLE)?;
        if performances.len() != 2 {
            return Err(format!("row count: {}", performances.len()).into());
        }
        let first = performances.first().ok_or("first row is absent")?;
        if first.event != "100 Meter Dash"
            || first.athlete != "John Smith"
            || first.team != "Wayzata"
            || first.mark != "11.23"
        {
            return Err(format!("first row: {first:?}").into());
        }
        Ok(())
    }

    #[test]
    fn a_body_without_a_results_table_yields_no_performances() -> TestResult {
        let performances = parse_performances("")?;
        if !performances.is_empty() {
            return Err(format!("row count: {}", performances.len()).into());
        }
        Ok(())
    }
}
