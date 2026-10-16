use super::{Expected, TestResult};
use calamine::{open_workbook, Data, Reader, Xlsx};
use std::path::Path;

pub(super) fn pr(path: &Path, expected: &Expected) -> TestResult {
    let [eligible_source_url, _] = &expected.result_capture_urls;
    let mut book: Xlsx<_> = open_workbook(path)?;
    let range = book.worksheet_range("PRs")?;
    check!(eq; range.height(), 2);
    let mut rows = range.rows();
    let headers = rows.next().ok_or("PR headers")?;
    let row = rows.next().ok_or("eligible fallback PR")?;
    let column = |label: &str| {
        headers
            .iter()
            .position(|cell| cell.to_string() == label)
            .ok_or("PR column")
    };
    for (label, value) in [
        ("Athlete ID", expected.public_id.as_str()),
        ("Canonical Candidate ID", expected.eligible.athlete.as_str()),
        ("Athlete", "Synthetic Participant"),
        ("Calculated PR", "13-9"),
        ("Unit", "m"),
        ("Performance ID", expected.eligible.id.as_str()),
        ("Source Key", "milesplit_result:201782263"),
        ("PR date", "2026-03-27"),
        ("Meet ID", expected.eligible.meet.as_str()),
        ("Meet", "Synthetic Michigan March Meet"),
        ("Result URL", eligible_source_url.as_str()),
        ("School at performance", "Detroit Catholic Central"),
        ("State at performance", "MI"),
        ("Current school", "Detroit Catholic Central"),
        ("Current state", "MI"),
        ("Current school as-of", "2026-10-01"),
    ] {
        check!(eq; row.get(column(label)?).map(ToString::to_string).as_deref(), Some(value));
    }
    check!(eq; row.get(column("Mark Value")?), Some(&Data::Float(4.191)));
    let athletes = book.worksheet_range("Athletes")?;
    check!(athletes.rows().any(|row| row
        .iter()
        .any(|cell| cell.to_string() == expected.public_id)));
    Ok(())
}
