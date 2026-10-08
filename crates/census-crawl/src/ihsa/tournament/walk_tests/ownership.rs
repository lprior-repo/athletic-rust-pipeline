use super::{options, seed, trimmed_events, Harness, TestResult, API, FIXTURE_HJ, FIXTURE_MEETS};
use census_domain::model::CanonicalPerformance;
use census_store::Table;
use serde_json::{json, Value};

#[test]
fn contradictory_summary_ownership_retains_an_unfinished_meet() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for (field, value) in [
                ("meetId", json!(99999)),
                ("eventId", json!("999999")),
                ("gender", json!("F")),
                ("classDivision", json!("2A")),
                ("round", json!("P")),
                ("roundLabel", json!("Prelims")),
                ("eventType", json!("RelayEvent")),
                ("eventName", json!("Boys Long Jump 1A - Finals")),
                ("scheduledDate", json!("2026-05-29T00:00:00.000Z")),
                ("hasResults", json!(false)),
            ] {
                let harness = Harness::new()?;
                replace(&harness, "meets", &trimmed_meets()?)?;
                let mut summary: Value = serde_json::from_str(FIXTURE_HJ)?;
                *summary.get_mut(field).ok_or("missing summary field")? = value;
                replace(&harness, "events/2790204/summary", &summary.to_string())?;
                let report = harness.run(&options()).await?;
                check!(eq; report.errors, 1, "{field}: {:?}", report.notes);
                check!(eq; harness.scan::<CanonicalPerformance>(Table::Performances)?.len(), 0);
                check!(report
                    .notes
                    .iter()
                    .any(|note| note.contains("2790204/summary")));
                replace(&harness, "events/2790204/summary", FIXTURE_HJ)?;
                let recovered = harness.run(&options()).await?;
                check!(eq; recovered.rows, 20, "{field}: no done receipt may hide recovery");
                check!(eq; recovered.errors, 0);
            }
            Ok(())
        })
}

#[test]
fn foreign_event_index_maps_no_requested_meet_and_can_recover() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for foreign_gender in [false, true] {
                let harness = Harness::new()?;
                replace(&harness, "meets", &trimmed_meets()?)?;
                let original = trimmed_events()?;
                let mut events: Value = serde_json::from_str(&original)?;
                if foreign_gender {
                    let rows = events.get_mut("data").and_then(Value::as_array_mut)
                        .ok_or("missing event rows")?;
                    for row in rows {
                        *row.get_mut("gender").ok_or("missing gender")? = json!("F");
                    }
                } else {
                    *events.get_mut("meetId").ok_or("missing meet owner")? = json!(99999);
                }
                replace(&harness, "meets/2026/events?gender=Boys", &events.to_string())?;
                let report = harness.run(&options()).await?;
                check!(eq; report.errors, 1);
                check!(eq; harness.scan::<CanonicalPerformance>(Table::Performances)?.len(), 0);
                check!(eq; harness.scan::<census_domain::model::CanonicalMeet>(Table::Meets)?.len(), 0);
                replace(&harness, "meets/2026/events?gender=Boys", &original)?;
                let recovered = harness.run(&options()).await?;
                check!(eq; recovered.rows, 20);
                check!(eq; recovered.errors, 0);
            }
            Ok(())
        })
}

fn trimmed_meets() -> TestResult<String> {
    let mut document: Value = serde_json::from_str(FIXTURE_MEETS)?;
    let rows = document
        .get_mut("data")
        .and_then(Value::as_array_mut)
        .ok_or("missing meet rows")?;
    rows.truncate(1);
    document["count"] = Value::from(rows.len());
    Ok(document.to_string())
}

fn replace(harness: &Harness, route: &str, body: &str) -> TestResult {
    seed(
        &harness.dir.path().join("http"),
        &[(format!("{API}/v1/track-field/{route}"), body)],
    )
}
