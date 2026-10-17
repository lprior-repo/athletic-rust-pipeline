use super::{parse_raw, ParsedRow, RawPage, TestResult};

pub(super) const URL: &str = "https://mi.milesplit.com/meets/123/results/456/raw";

pub(super) fn document(block: &str) -> String {
    format!(
        "<link rel=\"canonical\" href=\"{URL}\"><script type=\"application/ld+json\">{{\"@type\":\"SportsEvent\",\"name\":\"Synthetic Meet\",\"startDate\":\"2026-05-15\",\"sport\":\"Outdoor Track\",\"url\":\"{URL}\"}}</script><pre>{block}</pre>"
    )
}

pub(super) fn base_header() -> String {
    format!(
        "  Pl {:<24} {:<3} {:<24}{:>9}",
        "Name", "Yr", "Team", "Time"
    )
}

pub(super) fn base_row(name: &str, mark: &str) -> String {
    format!(
        "   1 {name:<24} {:<3} {:<24}{mark:>9}",
        "12", "Synthetic School"
    )
}

pub(super) fn page(header: &str, row: &str) -> TestResult<RawPage> {
    let block = format!("Boys 110 Hurdles Finals\n{header}\n{row}\n");
    Ok(parse_raw(&document(&block), URL)?)
}

pub(super) fn one_row(page: &RawPage) -> TestResult<&ParsedRow> {
    check!(eq; page.meet.rows_parsed, 1);
    check!(eq; page.meet.rows_skipped, 0);
    check!(eq; page.skipped.len(), 0);
    check!(eq; page.grade_issues.len(), 0);
    check!(eq; page.meet.events.len(), 1);
    let event = page.meet.events.first().ok_or("event")?;
    check!(eq; event.rows.len(), 1);
    Ok(event.rows.first().ok_or("row")?)
}

pub(super) fn refused(header: &str, row: &str, reason: &str) -> TestResult {
    let block = format!("Boys 110 Hurdles Finals\n{header}\n{row}\n");
    let html = document(&block);
    let parsed = parse_raw(&html, URL)?;
    check!(eq; parsed.meet.rows_parsed, 0);
    check!(eq; parsed.meet.rows_skipped, 1);
    check!(eq; parsed.skipped.len(), 1);
    let offset = html.find(row).ok_or("original row")?;
    check!(eq;
        parsed.skipped.first().ok_or("skip")?,
        &format!("pre line=3 byte_offset={offset} byte_length={}: {reason}; raw={row:?}", row.len())
    );
    check!(parsed.meet.events.iter().all(|event| event.rows.is_empty()));
    Ok(())
}
