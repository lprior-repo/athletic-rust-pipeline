use super::{Harness, TestResult, EMPTY};
use census_domain::model::CanonicalPerformance;
use census_store::Table;

const RESULTS: &str = r#"<table class="results">
<tr><td class="event">800 Meter Run</td><td class="athlete">Casey Runner</td><td class="team">Wayzata</td><td class="mark">2:01.00</td></tr>
</table>"#;

fn schedule_with_link(slug: &str) -> String {
    format!(
        "<table class=\"schedule\"><tr class=\"month-title\"><td>September</td></tr>\
         <tr class=\"event-row\"><td class=\"date\">5</td>\
         <td class=\"awayteam\"><a href=\"/links/{slug}\">Wayzata Invitational</a></td>\
         <td class=\"hometeam\">University of Minnesota</td></tr></table>"
    )
}

#[tokio::test]
async fn collected_result_rows_are_committed_with_the_schedule_meet() -> TestResult {
    let harness = Harness::new(&schedule_with_link("abc123"), Some(EMPTY))?;
    super::super::seed_cache(
        &harness.store.http_cache_dir(),
        "https://www.wayzataresults.com/links/abc123",
        RESULTS,
    )?;
    let report = harness.run(&[2026], &[], None, "2026-09-20").await?;
    let performances: Vec<CanonicalPerformance> = harness.store.scan(Table::Performances)?;
    check!(eq; performances.len(), 1);
    let performance = performances.first().ok_or("no performance was committed")?;
    check!(
        eq;
        performance.source_key,
        "wayzata|Wayzata Invitational|2026-09-05|2:01.00"
    );
    check!(eq; performance.mark, census_domain::model::Mark::Raw("2:01.00".to_string()));
    check!(eq; report.rows, 2);
    Ok(())
}
