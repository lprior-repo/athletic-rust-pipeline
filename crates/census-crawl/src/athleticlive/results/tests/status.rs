use super::*;
use serde_json::{json, Value};

fn status_document() -> TestResult<String> {
    let mut document: Value = serde_json::from_str(HJ_MITS)?;
    let source = document
        .get_mut("_source")
        .and_then(Value::as_object_mut)
        .ok_or("source")?;
    let template = source
        .get("r")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("captured row")?
        .clone();
    let mut rows = ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR", "X"]
        .into_iter()
        .map(|status| {
            let mut row = template.clone();
            let fields = row.as_object_mut().ok_or("row")?;
            fields.insert("m".into(), json!(status));
            fields.insert("im".into(), json!(2_000_000));
            fields.insert("vm".into(), json!(1));
            let athlete = fields
                .get_mut("a")
                .and_then(Value::as_object_mut)
                .ok_or("athlete")?;
            athlete.insert("y".into(), json!("11"));
            Ok(row)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let mut valid = template;
    valid
        .pointer_mut("/a/y")
        .ok_or("valid neighbor grade")?
        .clone_from(&json!("11"));
    rows.push(valid);
    source.insert("r".into(), json!(rows));
    Ok(serde_json::to_string(&document)?)
}

#[test]
fn parsed_invalid_rows_with_stale_positive_integer_marks_retain_athlete_status_and_contradiction(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let body = status_document()?;
            let doc = parse_event_document("status.json", &body)?;
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Michigan)),
            )?;
            let path = stage_capture(&dir, "event-doc-2254280-status.json", &body)?;
            let options = ResultOptions {
                documents: vec![path],
                ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
            };
            let report = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; report.errors, 0, "{}", joined(&report));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
            let expected_name = doc
                .rows
                .first()
                .and_then(|row| row.athlete.as_ref())
                .and_then(|athlete| athlete.name.as_deref())
                .ok_or("captured athlete")?;
            let athlete = athletes
                .iter()
                .find(|athlete| athlete.canonical_name == expected_name)
                .ok_or("invalid-status athlete retained")?;
            for status in ["DQ", "DNF", "DNS", "NH", "FOUL", "NM", "NT", "SCR", "X"] {
                let performance = performances
                    .iter()
                    .find(|row| row.mark == Mark::Raw(status.into()))
                    .ok_or("published invalid status retained")?;
                check!(eq; performance.athlete, athlete.id);
                let conflict = performance
                    .retained_conflicts
                    .iter()
                    .find(|row| row.family == "Published result status")
                    .ok_or("status contradiction")?;
                check!(eq; conflict.subject_id, performance.id.as_str());
                check!(conflict.detail.contains(status) && conflict.detail.contains("2000000"));
                check!(performance.evidence.iter().any(|evidence| evidence
                    .note
                    .as_deref()
                    .is_some_and(|note| note.contains(status) && note.contains("2000000"))));
            }
            let neighbor = performances
                .iter()
                .find(|performance| matches!(performance.mark, Mark::FieldImperial { .. }))
                .ok_or("valid same-event numeric neighbor retained")?;
            check!(neighbor.retained_conflicts.is_empty());
            let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
            check!(events
                .iter()
                .all(|event| event.retained_conflicts.is_empty()));
            Ok(())
        })
}

#[test]
fn parsed_standings_invalid_status_overrides_stale_numeric_raw_time_and_retains_both() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            write_schools(&store, &[(UsJurisdiction::Iowa, "Waukon")])?;
            let doc_path = stage_capture(&dir, "event-doc-2150205.json", XC_STATE)?;
            let mut payload: Value =
                serde_json::from_str(&standings_payload("Miriam Downing", "SO", "Waukon"))?;
            let row = payload
                .get_mut("zx91")
                .and_then(Value::as_object_mut)
                .ok_or("standing row")?;
            row.insert("m".into(), json!("DQ"));
            row.insert("rtm".into(), json!("1225.700"));
            let standings_path = stage_capture(
                &dir,
                "standings-status.json",
                &serde_json::to_string(&payload)?,
            )?;
            let options = ResultOptions {
                documents: vec![doc_path],
                standings: vec![StandingsCapture {
                    run_id: "1-1".into(),
                    path: standings_path,
                }],
                ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
            };
            let report = collect(&context(&store, &fetcher)?, &options).await?;
            check!(eq; report.errors, 0, "{}", joined(&report));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let rejected_winner = performances
                .iter()
                .find(|row| row.mark == Mark::Raw("DQ".into()))
                .ok_or("standings status retained")?;
            let conflict = rejected_winner
                .retained_conflicts
                .iter()
                .find(|row| row.family == "Published result status")
                .ok_or("stale raw time conflict")?;
            check!(conflict.detail.contains("DQ") && conflict.detail.contains("1225.700"));
            let url =
                crate::athleticlive::wire::standings_url(STATE_MEET, "1-1").ok_or("run URL")?;
            check!(rejected_winner
                .evidence
                .iter()
                .any(|evidence| evidence.source.url.as_deref() == Some(url.as_str())));
            Ok(())
        })
}
