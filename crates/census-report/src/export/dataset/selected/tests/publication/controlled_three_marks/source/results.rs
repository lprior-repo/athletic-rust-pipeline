use super::{
    capture, protected_state, ControlledMark, RowLocator, SourceReference, TestResult, MARKS,
};
use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::recorded::record_retained_html_result_capture;
use census_crawl::participation_evidence::result_ownership::read_retained_html_result_rows;
use census_crawl::Recording;
use census_store::{RetainedCaptureRootSchemaV1, Store};

const HEADERS: &[u8] = b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=UTF-8\r\nX-Controlled-Fixture: three-marks-results\r\n\r\n";

fn document(meet: u32, mark: ControlledMark, url: &str) -> String {
    let day = mark.date;
    let whole = mark.time / 100;
    let fraction = mark.time % 100;
    format!(
        r#"<!doctype html>
<html><head>
<link rel="canonical" href="{url}">
<script type="application/ld+json">{{"@type":"SportsEvent","name":"Controlled Track Meet {meet}","startDate":"{day}","endDate":"{day}","sport":"Outdoor Track & Field","location":{{"name":"Controlled Track Venue","address":{{"addressRegion":"MI","addressLocality":"Detroit"}}}},"url":"{url}"}}</script>
</head><body>
<p>SYNTHETIC CONTROLLED FIXTURE. Not a genuine athlete result.</p>
<pre>Boys Varsity 400 Meter Dash
======================================================================================
{place:<4} {name:<26} {year:<3} {team:<38} {time:>10}
======================================================================================
{rank:<4} {athlete:<26} {grade:<3} {school:<38} {whole:>6}.{fraction:02}a
</pre></body></html>"#,
        place = "Pl",
        name = "Athlete",
        year = "Yr",
        team = "Team",
        time = "Time",
        rank = 1,
        athlete = "Controlled Target",
        grade = 11,
        school = "Detroit Catholic Central",
    )
}

fn locator(html: &str) -> TestResult<SourceRowLocator> {
    let start = html
        .find("<pre>")
        .ok_or("controlled raw block absent")?
        .checked_add(5)
        .ok_or("controlled block offset overflow")?;
    let block = html.get(start..).ok_or("controlled block bounds")?;
    let end = block
        .find("</pre>")
        .ok_or("controlled raw block end absent")?;
    let block = block.get(..end).ok_or("controlled raw block bounds")?;
    let row = block
        .find("1    Controlled Target")
        .ok_or("controlled row absent")?;
    let ordinal = u32::try_from(block[..row].matches('\n').count())?
        .checked_add(1)
        .ok_or("controlled row ordinal overflow")?;
    let length = block[row..].find('\n').ok_or("controlled row end absent")?;
    Ok(SourceRowLocator {
        ordinal,
        byte_offset: start
            .checked_add(row)
            .ok_or("controlled row offset overflow")?,
        byte_length: length,
    })
}

pub(super) fn install(store: &Store) -> TestResult<(Vec<SourceReference>, Vec<RowLocator>)> {
    let mut sources = Vec::new();
    let mut locators = Vec::new();
    for (index, (meet, mark)) in [495051, 502100, 502101].into_iter().zip(MARKS).enumerate() {
        let url = format!(
            "https://mi.milesplit.com/meets/{meet}-controlled-meet-2026/results/{meet}0/raw"
        );
        let html = document(meet, mark, &url);
        let capture = capture(
            &url,
            html.as_bytes(),
            HEADERS,
            "2026-10-16T00:06:38Z",
            "2026-10-16T00:06:57Z",
        )?;
        let row = locator(&html)?;
        let document = read_retained_html_result_rows(&capture, &[row])?;
        let key = document.context_reference().to_owned();
        let before = protected_state(store)?;
        let recording = Recording::new();
        record_retained_html_result_capture(store, &recording, &document)?;
        check!(eq; protected_state(store)?, before);
        census_crawl::recording::prepare_recorded(
            recording.drain(),
            &format!("controlled-result-intake-{index}"),
        )?
        .apply(store)?;
        sources.push(SourceReference {
            schema: RetainedCaptureRootSchemaV1::HtmlResultCaptureV1,
            key: key.clone(),
            payload: serde_json::to_value(&capture)?,
        });
        locators.push(RowLocator {
            document_key: key,
            locator: row,
        });
    }
    Ok((sources, locators))
}
