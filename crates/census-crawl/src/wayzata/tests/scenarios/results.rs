use super::super::{
    EVENT_SUMMARY_79065, FULL_XC_2026, MEET_DOC_79065, RESULT_DOCUMENT_2930235,
    RESULT_DOCUMENT_2930238,
};
use super::{Harness, TestResult, EMPTY};
use crate::athleticlive::{event_doc_url, event_summary_url};
use crate::wayzata::results::{search_query, search_url, RESULTS_HOST};
use crate::wayzata::BASE;
use crate::CollectionDisposition;
use census_domain::model::{Grade, Mark};
use census_domain::UsJurisdiction;
use serde_json::{json, Value};

const MEET: u64 = 79065;
const EVENT_DOCUMENT: u64 = 2930238;
const SECOND_EVENT_DOCUMENT: u64 = 2930235;
const MISSING_EVENT_DOCUMENT: u64 = 2930241;
const SLUG: &str = "v70kmd";
const REPORTED_TIME: &str = "9:59.41";
const WITHHELD_TIME: &str = "10:34.11";
const WITHHELD_ROW: usize = 6;
const MAPPED_ATHLETE: &str = "Carter Hering";
const WITHHELD_ATHLETE: &str = "Bilal Abdoullah";

fn lane_schedule() -> String {
    format!(
        concat!(
            "<table class=\"schedule\">",
            "<tr class=\"month-title\"><td>October</td></tr>",
            "<tr class=\"event-row\"><td class=\"date\">9</td>",
            "<td class=\"awayteam\">Hopkins Harvest Hoedown</td>",
            "<td class=\"hometeam\">University of Minnesota</td>",
            "<td><a href=\"/links/{slug}\" aria-label=\"Results\">Results</a></td></tr>",
            "</table>"
        ),
        slug = SLUG
    )
}

fn lane_harness() -> TestResult<Harness> {
    let harness = Harness::new_lane(&lane_schedule(), Some(EMPTY))?;
    harness.seed_school("Hopkins", UsJurisdiction::Minnesota)?;
    harness.seed_redirect(
        &format!("{BASE}/links/{SLUG}"),
        &format!("https://{RESULTS_HOST}/meets/{MEET}"),
        "<app-root></app-root>",
    )?;
    harness.seed_post_json(
        &search_url(),
        &serde_json::to_string(&search_query(MEET))?,
        MEET_DOC_79065,
    )?;
    harness.seed_capture(&event_summary_url(MEET), EVENT_SUMMARY_79065)?;
    harness.seed_capture(&event_doc_url(EVENT_DOCUMENT), RESULT_DOCUMENT_2930238)?;
    harness.seed_capture(
        &event_doc_url(SECOND_EVENT_DOCUMENT),
        RESULT_DOCUMENT_2930235,
    )?;
    Ok(harness)
}

fn document_with_withheld_validity() -> TestResult<String> {
    let mut document: Value = serde_json::from_str(RESULT_DOCUMENT_2930238)?;
    let rows = document
        .get_mut("_source")
        .and_then(|source| source.get_mut("r"))
        .and_then(Value::as_array_mut)
        .ok_or("published rows")?;
    let withheld = rows.get_mut(WITHHELD_ROW).ok_or("published row")?;
    withheld["vm"] = json!(0);
    Ok(serde_json::to_string(&document)?)
}

#[test]
fn wayzata_lane_reads_all_published_cross_country_links() -> TestResult {
    let rows = crate::wayzata::schedule_rows(FULL_XC_2026, 2026)?;
    let links = rows.iter().filter(|row| row.slug.is_some()).count();
    check!(eq; links, 110, "every published cross-country row carries a provider link");
    check!(rows.iter().any(|row| row.slug.as_deref() == Some(SLUG)));
    Ok(())
}

#[tokio::test]
async fn wayzata_lane_resolves_each_published_slug_to_its_tenant_meet() -> TestResult {
    let harness = lane_harness()?;
    let report = harness.run(&[2026], &[], None, "2026-10-11").await?;
    check!(eq; report.rows, 1);
    let meets = harness.meets()?;
    let meet = meets.first().ok_or("projected meet")?;
    let ledger = harness.ledger()?;
    check!(eq; ledger.len(), 1);
    let record = ledger.first().ok_or("ledger record")?;
    check!(eq; record["slug"], SLUG);
    check!(eq; record["meet_id"], meet.id.as_str());
    check!(eq; record["meet_name"], meet.name);
    check!(eq; record["athleticlive_meet_id"], MEET);
    check!(record["athletic_net_meet_id"].as_u64().is_some());
    check!(eq; record["tenant_meet_name"], "Hopkins Harvest Hoedown");
    check!(record["tenant_meet_key"]
        .as_str()
        .is_some_and(|key| !key.is_empty()));
    check!(
        eq;
        record["link_response_url"],
        format!("https://{RESULTS_HOST}/meets/{MEET}")
    );
    check!(eq; record["documents"].as_array().map(Vec::len), Some(2));
    Ok(())
}

#[tokio::test]
async fn wayzata_lane_owns_unfetched_event_documents_as_unfinished_work() -> TestResult {
    let harness = lane_harness()?;
    let report = harness.run(&[2026], &[], None, "2026-10-11").await?;
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(report
        .unfinished
        .iter()
        .any(|locator| locator == &event_doc_url(MISSING_EVENT_DOCUMENT)));
    check!(report
        .notes
        .iter()
        .all(|note| !note.starts_with("capture refused:")));
    let ledger = harness.ledger()?;
    let record = ledger.first().ok_or("ledger record")?;
    check!(eq; record["refused_documents"].as_array().map(Vec::len), Some(1));
    check!(eq; record["owed"].as_array().map(Vec::len), Some(1));
    check!(eq; record["errors"], 0);
    Ok(())
}

#[tokio::test]
async fn wayzata_marks_are_typed_from_the_published_rows() -> TestResult {
    let harness = lane_harness()?;
    harness.run(&[2026], &[], None, "2026-10-11").await?;
    let athlete = harness
        .athletes()?
        .into_iter()
        .find(|athlete| athlete.canonical_name == MAPPED_ATHLETE)
        .ok_or("published athlete")?;
    let performance = harness
        .performances()?
        .into_iter()
        .find(|performance| performance.athlete == athlete.id)
        .ok_or("published performance")?;
    check!(eq; performance.place, Some(1));
    check!(eq; performance.wind_mps, None);
    check!(eq; performance.heat.as_deref(), Some("2"));
    check!(performance
        .round
        .as_deref()
        .is_some_and(|round| round.eq_ignore_ascii_case("finals")));
    check!(eq; performance.observed_grade, Grade::new(12));
    let reported = crate::hytek::parse_time(REPORTED_TIME).ok_or("published time")?;
    check!(eq; performance.mark, Mark::TimeSeconds(reported));
    Ok(())
}

#[tokio::test]
async fn wayzata_marks_retain_withheld_validity_as_raw_with_a_reason() -> TestResult {
    let harness = lane_harness()?;
    harness.seed_capture(
        &event_doc_url(EVENT_DOCUMENT),
        &document_with_withheld_validity()?,
    )?;
    harness.run(&[2026], &[], None, "2026-10-11").await?;
    let athlete = harness
        .athletes()?
        .into_iter()
        .find(|athlete| athlete.canonical_name == WITHHELD_ATHLETE)
        .ok_or("published athlete")?;
    let performance = harness
        .performances()?
        .into_iter()
        .find(|performance| performance.athlete == athlete.id)
        .ok_or("published performance")?;
    check!(eq; performance.mark, Mark::Raw(WITHHELD_TIME.to_string()));
    check!(eq; performance.retained_conflicts.len(), 1);
    let ledger = harness.ledger()?;
    let record = ledger.first().ok_or("ledger record")?;
    let notes = record["notes"].as_array().ok_or("ledger notes")?;
    check!(notes.iter().any(|note| note
        .as_str()
        .is_some_and(|line| line.starts_with("source status/numeric conflicts:"))));
    Ok(())
}

#[tokio::test]
async fn wayzata_lane_replays_without_duplicating_projections() -> TestResult {
    let harness = lane_harness()?;
    let first = harness.run(&[2026], &[], None, "2026-10-11").await?;
    check!(eq; first.rows, 1);
    let projected = harness.performances()?.len();
    check!(projected > 0, "the first run publishes performance rows");
    let replay = harness.run(&[2026], &[], None, "2026-10-11").await?;
    check!(eq; replay.rows, 0, "the committed schedule row is not projected twice");
    check!(eq; harness.meets()?.len(), 1);
    check!(eq; harness.ledger()?.len(), 1);
    check!(eq; harness.performances()?.len(), projected);
    Ok(())
}

#[tokio::test]
async fn wayzata_lane_stays_off_without_an_operator_grant() -> TestResult {
    let harness = Harness::new(&lane_schedule(), Some(EMPTY))?;
    harness.seed_school("Hopkins", UsJurisdiction::Minnesota)?;
    harness.seed_redirect(
        &format!("{BASE}/links/{SLUG}"),
        &format!("https://{RESULTS_HOST}/meets/{MEET}"),
        "<app-root></app-root>",
    )?;
    let report = harness.run(&[2026], &[], None, "2026-10-11").await?;
    check!(eq; report.rows, 1);
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("results lane off:")));
    check!(harness.ledger()?.is_empty());
    check!(harness.performances()?.is_empty());
    Ok(())
}

#[tokio::test]
async fn wayzata_schedule_date_cannot_join_a_different_tenant_meet() -> TestResult {
    let harness = lane_harness()?;
    let schedule = lane_schedule().replace("class=\"date\">9", "class=\"date\">8");
    harness.seed(crate::wayzata::ScheduleSport::Track, 2026, &schedule)?;
    let report = harness.run(&[2026], &[], None, "2026-10-11").await?;
    check!(eq; report.disposition, CollectionDisposition::Partial);
    check!(harness.performances()?.is_empty());
    check!(harness.ledger()?.is_empty());
    check!(report
        .notes
        .iter()
        .any(|note| note.contains("tenant meet date")));
    Ok(())
}
