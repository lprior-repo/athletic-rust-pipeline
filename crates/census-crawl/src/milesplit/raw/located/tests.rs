use super::*;
use crate::CrawlError;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const URL: &str = "https://mi.milesplit.com/meets/123/results/456/raw";

fn document(block: &str) -> String {
    format!(
        "<link rel=\"canonical\" href=\"{URL}\"><script type=\"application/ld+json\">{{\"@type\":\"SportsEvent\",\"name\":\"Meet\",\"startDate\":\"2026-05-15\",\"endDate\":\"2026-05-15\",\"sport\":\"Outdoor Track\",\"url\":\"{URL}\"}}</script><pre>{block}</pre>"
    )
}

fn check(value: &str, field: &str) -> CrawlResult<()> {
    if value != URL {
        return Err(CrawlError::Schema {
            url: URL.into(),
            detail: field.into(),
        });
    }
    Ok(())
}

fn header() -> String {
    format!(
        "  Pl {:<24} {:<3} {:<24}{:>9}",
        "Name", "Yr", "Team", "Time"
    )
}

fn row(name: &str, mark: &str) -> String {
    format!(
        "   1 {name:<24} {:<3} {:<24}{mark:>9} 8 (1)",
        "12", "Retained School"
    )
}

fn mixed() -> (String, Vec<String>) {
    let rows = vec![
        row("MÜLLER, Émile", "13.57a"),
        row("Other Runner", "14.10a"),
        row("Third Runner", "36.23a"),
    ];
    let bad = row("Rejected Runner", "invalid");
    let block = format!(
        "Boys 110 Hurdles\r\n{}\r\n{}\r\n{bad}\r\nFinals\r\n{}\r\n{}\r\nBoys 300 Hurdles\r\n{}\r\n{}\r\n",
        header(), rows[0], header(), rows[1], header(), rows[2],
    );
    (document(&block), rows)
}

#[test]
fn located_route_preserves_legacy_rows_and_exact_success_spans() -> TestResult {
    let (html, original_rows) = mixed();
    let legacy = super::super::parse_bound_with_dates(&html, URL, &check)?;
    let mut found = Vec::new();
    let located = parse_raw_located_bound(&html, URL, &check, &mut |locator, row| {
        found.push((locator, row.clone()));
        Ok(())
    })?;
    assert_eq!(located.page, legacy.0);
    assert_eq!(located.published_start, legacy.1);
    assert_eq!(located.published_end, legacy.2);
    assert_eq!(found.len(), 3);
    assert_eq!(located.page.meet.rows_skipped, 1);
    for (index, ((locator, row), original)) in found.iter().zip(&original_rows).enumerate() {
        assert_eq!(locator.event_index, index);
        assert_eq!(locator.row_index, 0);
        assert_eq!(locator.row.byte_length, original.len());
        assert_eq!(
            locator.row.byte_offset,
            html.find(original).ok_or("original span")?
        );
        let end = locator
            .row
            .byte_offset
            .checked_add(locator.row.byte_length)
            .ok_or("span end")?;
        assert_eq!(
            html.get(locator.row.byte_offset..end),
            Some(original.as_str())
        );
        assert_eq!(located.page.meet.events[index].rows[0], *row);
    }
    assert_eq!(
        found
            .iter()
            .map(|(span, _)| span.row.ordinal)
            .collect::<Vec<_>>(),
        vec![3, 7, 10]
    );
    Ok(())
}

#[test]
fn later_visitor_refusal_cannot_return_a_completed_document() -> TestResult {
    let (html, _) = mixed();
    let mut visited = 0usize;
    let result = parse_raw_located_bound(&html, URL, &check, &mut |_, _| {
        visited += 1;
        if visited == 2 {
            return Err(CrawlError::Schema {
                url: URL.into(),
                detail: "late locator admission".into(),
            });
        }
        Ok(())
    });
    assert_eq!(visited, 2);
    assert!(
        matches!(result, Err(CrawlError::Schema { detail, .. }) if detail == "late locator admission")
    );
    assert_eq!(
        super::super::parse_bound_with_dates(&html, URL, &check)?
            .0
            .meet
            .rows_parsed,
        3
    );
    Ok(())
}

#[test]
fn later_foreign_metadata_is_rejected_before_any_success_callback() -> TestResult {
    let (mut html, _) = mixed();
    html.push_str(
        "<link rel=\"canonical\" href=\"https://mi.milesplit.com/meets/999/results/456/raw\">",
    );
    let mut visited = 0usize;
    let result = parse_raw_located_bound(&html, URL, &check, &mut |_, _| {
        visited += 1;
        Ok(())
    });
    let legacy = super::super::parse_bound_with_dates(&html, URL, &check);
    assert_eq!(visited, 0);
    assert_eq!(
        result.err().ok_or("located refusal")?.to_string(),
        legacy.err().ok_or("legacy refusal")?.to_string()
    );
    Ok(())
}
