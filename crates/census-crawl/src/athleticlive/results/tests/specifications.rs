use super::*;
use serde_json::{json, Value};

fn event_document(capture_id: u64, short_name: &str, abbreviation: &str) -> TestResult<String> {
    let mut document: Value = serde_json::from_str(HJ_MITS)?;
    let source = document
        .get_mut("_source")
        .and_then(Value::as_object_mut)
        .ok_or("source")?;
    let mut row = source
        .get("r")
        .and_then(Value::as_array)
        .and_then(|rows| rows.first())
        .ok_or("captured row")?
        .clone();
    let fields = row.as_object_mut().ok_or("row")?;
    fields.insert("m".into(), json!("40-0"));
    fields.insert("vm".into(), json!(1));
    fields
        .get_mut("a")
        .and_then(Value::as_object_mut)
        .ok_or("athlete")?
        .insert("y".into(), json!("11"));
    source.insert("r".into(), json!([row]));
    source.insert("i".into(), json!(capture_id));
    source.insert("n".into(), json!("Girls Shot Put"));
    source.insert("sn".into(), json!(short_name));
    source.insert("ab".into(), json!(abbreviation));
    source.insert("un".into(), json!("Shot Put"));
    Ok(serde_json::to_string(&document)?)
}

async fn project_documents(
    dir: &tempfile::TempDir,
    store: &Store,
    fetcher: &Fetcher,
    contexts: &[(u64, &str, &str)],
) -> TestResult<AdapterReport> {
    let documents = contexts
        .iter()
        .map(|(id, label, abbreviation)| {
            let body = event_document(*id, label, abbreviation)?;
            let doc = parse_event_document("specification.json", &body)?;
            write_schools(
                store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Michigan)),
            )?;
            stage_capture(dir, &format!("event-doc-{id}.json"), &body)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let options = ResultOptions {
        documents,
        ..ResultOptions::for_meet(mits_meet(), OBSERVED_ON)
    };
    Ok(collect(&context(store, fetcher)?, &options).await?)
}

fn event_for<'a>(
    events: &'a [CanonicalEvent],
    performances: &[CanonicalPerformance],
    capture: u64,
) -> TestResult<&'a CanonicalEvent> {
    let key = format!("athleticlive:{capture}:row0:");
    let performance = performances
        .iter()
        .find(|row| row.source_key.starts_with(&key))
        .ok_or("published row reference")?;
    events
        .iter()
        .find(|event| event.id == performance.event)
        .ok_or("specification-qualified event".into())
}

#[test]
fn published_short_names_qualify_refs_before_unknown_abbreviations_and_preserve_equivalent_aliases(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let report = project_documents(
                &dir,
                &store,
                &fetcher,
                &[
                    (2_254_280, "Girls Shot Put (4kg)", "source abbreviation"),
                    (2_254_281, "Girls Shot Put (3kg)", "SP"),
                    (2_254_282, "Girls Shot Put (4000g)", "SP"),
                    (2_254_283, "Girls Shot Put", "SP"),
                ],
            )
            .await?;
            check!(eq; report.errors, 0, "{}", joined(&report));
            let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
            let (four, three, equivalent, unknown) = (
                event_for(&events, &performances, 2_254_280)?,
                event_for(&events, &performances, 2_254_281)?,
                event_for(&events, &performances, 2_254_282)?,
                event_for(&events, &performances, 2_254_283)?,
            );
            check!(eq; four.kind, EventKind::ShotPut);
            check!(eq; four.id, equivalent.id);
            check!(four.id != three.id && four.id != unknown.id);
            check!(eq; four.specification.implement.ok_or("mass")?.micrograms(), 4_000_000_000);
            check!(eq; three.specification.implement.ok_or("mass")?.micrograms(), 3_000_000_000);
            check!(eq; unknown.specification.implement, None);
            check!(four
                .source_labels
                .iter()
                .any(|label| label.label == "Girls Shot Put (4000g)"));
            check!(four
                .source_labels
                .iter()
                .any(|label| label.label == "source abbreviation"));
            check!(eq; four.resolved_specification()?, equivalent.resolved_specification()?);
            Ok(())
        })
}

#[test]
fn malformed_qualified_short_name_is_unfinished_without_hiding_later_valid_document() -> TestResult
{
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        let (dir, store, fetcher) = scratch()?;
        let report = project_documents(&dir, &store, &fetcher, &[
            (2_254_280, "Girls Shot Put (0kg)", "SP"),
            (2_254_281, "Girls Shot Put (4kg)", "SP"),
        ]).await?;
        check!(eq; report.errors, 1, "{}", joined(&report));
        let performances: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
        let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
        let valid = event_for(&events, &performances, 2_254_281)?;
        check!(eq; valid.specification.implement.ok_or("later published mass")?.micrograms(), 4_000_000_000);
        check!(performances.iter().all(|row| !row.source_key.starts_with("athleticlive:2254280:")));
        check!(joined(&report).contains("published implement mass"));
        Ok(())
    })
}
