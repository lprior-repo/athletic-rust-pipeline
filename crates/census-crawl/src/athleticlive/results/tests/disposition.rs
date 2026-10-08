use super::*;
use serde_json::{json, Value};

fn cohort_document(fixture: &str, field: &str, value: Value) -> TestResult<String> {
    let mut document: Value = serde_json::from_str(fixture)?;
    let source = document.get_mut("_source").ok_or("missing event source")?;
    let rows = source
        .get_mut("r")
        .and_then(Value::as_array_mut)
        .ok_or("missing rows")?;
    let mut row = rows
        .iter()
        .find(|row| row.pointer("/a/y").and_then(Value::as_str) == Some("SO"))
        .cloned()
        .ok_or("missing captured sophomore")?;
    row.as_object_mut()
        .ok_or("row object")?
        .insert(field.to_string(), value);
    *rows = vec![row];
    Ok(document.to_string())
}

#[test]
fn stale_numeric_no_results_keep_the_cohort_athlete_and_refused_locator() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for status in ["DQ", "DNF", "DNS", "NH", "FOUL"] {
                let body = cohort_document(XC_STATE, "m", json!(status))?;
                let mut document: Value = serde_json::from_str(&body)?;
                document
                    .pointer_mut("/_source/r/0")
                    .and_then(Value::as_object_mut)
                    .ok_or("row object")?
                    .insert("vm".into(), json!(0));
                let body = document.to_string();
                let doc = parse_event_document(&event_doc_url(2_150_205), &body)?;
                let (dir, store, fetcher) = scratch()?;
                write_schools(
                    &store,
                    &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
                )?;
                let path = stage_capture(&dir, "no-result.json", &body)?;
                let options = ResultOptions {
                    documents: vec![path.clone()],
                    ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
                };
                collect(&context(&store, &fetcher)?, &options).await?;
                let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
                check!(eq; performances.len(), 1, "{status}");
                let performance = performances.first().ok_or("retained raw row")?;
                check!(eq; performance.mark, Mark::Raw(status.into()));
                check!(performance
                    .retained_conflicts
                    .iter()
                    .any(|conflict| conflict.subject_id == performance.id.as_str()));
                let athletes: Vec<CanonicalAthlete> = store.scan(Table::Athletes)?;
                let [athlete] = athletes.as_slice() else {
                    return Err(format!("{status}: cohort athlete was dropped").into());
                };
                check!(eq; athlete.grad_year, GradYear::new(2028).ok_or("2028 is a cohort")?);
                check!(eq; performance.athlete, athlete.id);
                check!(performance.evidence.iter().any(|evidence| evidence
                    .note
                    .as_deref()
                    .is_some_and(
                        |note| note.contains(status) && note.contains("numeric winner withheld")
                    )
                    && evidence.source.url.as_deref() == Some(event_doc_url(2_150_205).as_str())));
                check!(eq; std::fs::read_to_string(path)?, body, "capture must remain intact");
            }
            Ok(())
        })
}

#[test]
fn invalid_validity_and_disagreeing_mark_channels_never_mint_a_numeric_mark() -> TestResult {
    for (field, value, expected) in [
        ("vm", json!(0), "invalid vm"),
        ("vm", json!(2), "invalid vm"),
        ("m", json!("1:00.00"), "numeric mark channels"),
    ] {
        let body = cohort_document(XC_STATE, field, value)?;
        let doc = parse_event_document(&event_doc_url(2_150_205), &body)?;
        let row = doc.rows.first().ok_or("fixture row")?;
        check!(eq; row.canonical_mark(&EventKind::CrossCountry), Some(Mark::Raw(row.mark.clone().ok_or("published value")?)));
        check!(eq; row.mark_contradiction(&EventKind::CrossCountry), Some(expected));
    }
    let body = cohort_document(XC_STATE, "vm", json!(1))?;
    let doc = parse_event_document(&event_doc_url(2_150_205), &body)?;
    let row = doc.rows.first().ok_or("fixture row")?;
    let published = crate::hytek::parse_time(row.mark.as_deref().ok_or("display")?)
        .ok_or("valid displayed time")?;
    check!(eq; row.canonical_mark(&EventKind::CrossCountry), Some(Mark::TimeSeconds(published)));
    for (display, integer) in [("10.941", 10_940), ("10.949", 10_940), ("10.94", 10_940)] {
        let mut row = row.clone();
        row.mark = Some(display.into());
        row.mark_int = Some(json!(integer));
        check!(eq; row.canonical_mark(&EventKind::CrossCountry), Some(Mark::TimeSeconds(ExactSeconds::parse(display)?)));
        check!(eq; row.mark_contradiction(&EventKind::CrossCountry), None);
    }
    Ok(())
}

#[test]
fn field_no_result_with_a_stale_integer_retains_raw_status_and_contradiction() -> TestResult {
    let mut document: Value = serde_json::from_str(HJ_MITS)?;
    let row = document
        .pointer_mut("/_source/r/0")
        .and_then(Value::as_object_mut)
        .ok_or("captured field result")?;
    row.insert("m".into(), json!("NH"));
    row.insert("vm".into(), json!(0));
    let doc = parse_event_document(&event_doc_url(2_254_280), &document.to_string())?;
    let row = doc.rows.first().ok_or("captured field row")?;
    check!(eq; row.canonical_mark(&EventKind::HighJump), Some(Mark::Raw("NH".into())));
    check!(eq; row.mark_contradiction(&EventKind::HighJump), Some("NH"));
    Ok(())
}
