use super::*;
use crate::milesplit::raw::located::parse_raw_located_bound;
use crate::{CrawlError, CrawlResult};

fn owner_check(value: &str, field: &str) -> CrawlResult<()> {
    if value == support::URL {
        Ok(())
    } else {
        Err(CrawlError::Schema {
            url: support::URL.to_string(),
            detail: field.to_string(),
        })
    }
}

#[test]
fn corrected_wind_rows_keep_exact_original_unicode_and_crlf_locators() -> TestResult {
    let header = format!("{}{:>8}{:>6}", base_header(), "Wind", "H#");
    let first = format!(
        "{}{:>8}{:>6}",
        base_row("MÜLLER, Émile", "13.26"),
        "+0.2",
        "2"
    );
    let bad = format!(
        "{}{:>8}{:>6}",
        base_row("Rejected Runner", "13.27"),
        "NaN",
        "2"
    );
    let last = format!(
        "{}{:>8}{:>6}",
        base_row("Last Runner", "13.28"),
        "-0.4",
        "3"
    );
    let block = format!("Boys 110 Hurdles Finals\r\n{header}\r\n{first}\r\n{bad}\r\n{last}\r\n");
    let html = document(&block);
    let mut found = Vec::new();
    let located =
        parse_raw_located_bound(&html, support::URL, &owner_check, &mut |locator, row| {
            found.push((locator, row.clone()));
            Ok(())
        })?;
    check!(eq; located.page, parse_raw(&html, support::URL)?);
    check!(eq; located.page.meet.rows_parsed, 2);
    check!(eq; located.page.meet.rows_skipped, 1);
    check!(eq; found.len(), 2);
    for ((locator, row), (original, ordinal, index, wind, heat)) in found
        .iter()
        .zip([(&first, 3, 0, 0.2, "2"), (&last, 5, 1, -0.4, "3")])
    {
        check!(eq; locator.event_index, 0);
        check!(eq; locator.row_index, index);
        check!(eq; locator.row.ordinal, ordinal);
        check!(eq; locator.row.byte_length, original.len());
        check!(eq; locator.row.byte_offset, html.find(original).ok_or("original bytes")?);
        let end = locator
            .row
            .byte_offset
            .checked_add(locator.row.byte_length)
            .ok_or("span end")?;
        check!(eq; html.get(locator.row.byte_offset..end), Some(original.as_str()));
        check!(eq; row.wind_mps, Some(wind));
        check!(eq; row.heat.as_deref(), Some(heat));
    }
    check!(eq; found.first().ok_or("first success")?.1.name, "MÜLLER, Émile");
    let expected_offset = html.find(&bad).ok_or("rejected bytes")?;
    check!(eq;
        located.page.skipped.first().ok_or("located refusal")?,
        &format!("pre line=4 byte_offset={expected_offset} byte_length={}: invalid declared wind field; raw={bad:?}", bad.len())
    );
    Ok(())
}
