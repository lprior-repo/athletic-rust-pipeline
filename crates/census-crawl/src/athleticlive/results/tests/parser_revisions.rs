use super::*;
use serde_json::{json, Value};

const HISTORICAL_EFFECT_PHASE: &str = "athleticlive_results_effect_v2";
const HISTORICAL_ROW_PHASE: &str = "athleticlive_projection_v4";
const HISTORICAL_PARSER: &str = "athleticlive_results_v4";

fn snapshot(store: &Store, phase: &str) -> TestResult<BTreeMap<String, Value>> {
    store
        .journal_keys(phase)?
        .into_iter()
        .map(|key| {
            let payload = store
                .journal_payload(phase, &key)?
                .ok_or("receipt payload")?;
            Ok((key, payload))
        })
        .collect()
}

fn one_row_body() -> TestResult<String> {
    let mut document: Value = serde_json::from_str(XC_STATE)?;
    document
        .pointer_mut("/_source/r")
        .and_then(Value::as_array_mut)
        .ok_or("published rows")?
        .truncate(1);
    Ok(document.to_string())
}

fn options(path: &str) -> ResultOptions {
    ResultOptions {
        documents: vec![path.into()],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    }
}

fn historical_key(payload: &Value) -> TestResult<String> {
    let mut identity = payload.as_object().ok_or("effect payload")?.clone();
    identity.remove("path").ok_or("effect path")?;
    identity.remove("metadata").ok_or("effect metadata")?;
    let digest = census_domain::model::serialized_digest(&Value::Object(identity))?;
    let meet = payload["meet"].as_str().ok_or("effect owner")?;
    let role = payload["role"].as_str().ok_or("effect role")?;
    Ok(format!("{meet}:{role}:{digest}"))
}

async fn seed_history(
    store: &Store,
    fetcher: &Fetcher,
    options: &ResultOptions,
    capture: &crate::ResultsCaptureContext,
) -> TestResult {
    let recording = crate::recording::Recording::new();
    let staged = AdapterContext {
        recording: Some(&recording),
        ..context(store, fetcher, capture)?
    };
    let report = collect(&staged, options).await?;
    check!(eq; report.errors, 0, "{}", joined(&report));
    check!(eq; report.rows, 1);
    let mut batch = store.write_batch();
    let recorded = recording.drain();
    for rows in recorded.rows {
        batch.record_many(rows.table, &rows.rows)?;
    }
    for entry in recorded.journal {
        let (phase, key, payload) = if entry.phase == super::super::EFFECT_PHASE {
            let mut payload = entry.payload;
            payload["parser"] = json!(HISTORICAL_PARSER);
            (HISTORICAL_EFFECT_PHASE, historical_key(&payload)?, payload)
        } else if entry.phase == super::super::ROW_PHASE {
            (HISTORICAL_ROW_PHASE, entry.key, entry.payload)
        } else {
            (entry.phase.as_str(), entry.key, entry.payload)
        };
        batch.journal_done(phase, &key, &payload)?;
    }
    batch.commit()?;
    Ok(())
}

fn prepare(dir: &tempfile::TempDir, store: &Store) -> TestResult<ResultOptions> {
    let body = one_row_body()?;
    let doc = parse_event_document(&event_doc_url(2_150_205), &body)?;
    write_schools(
        store,
        &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
    )?;
    let path = stage_capture(dir, "historical-xc.json", &body)?;
    Ok(options(&path))
}

#[test]
fn historical_v2_effect_and_v4_rows_cannot_complete_v3_effect_and_v5_projection() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let options = prepare(&dir, &store)?;
            seed_history(&store, &fetcher, &options, &capture).await?;
            let history = snapshot(&store, HISTORICAL_EFFECT_PHASE)?;
            let old_rows = snapshot(&store, HISTORICAL_ROW_PHASE)?;
            let raw = snapshot(&store, super::super::CAPTURE_PHASE)?;
            check!(eq; history.len(), 1);
            check!(eq; super::super::CAPTURE_PHASE, "athleticlive_results_capture_v1");
            check!(eq; super::super::EFFECT_PHASE, "athleticlive_results_effect_v3");
            check!(eq; super::super::ROW_PHASE, "athleticlive_projection_v5");
            check!(eq; super::super::PARSER, "athleticlive_results_v5");
            check!(eq; receipts(&store)?, BTreeMap::new());
            let applied = collect(&context(&store, &fetcher, &capture)?, &options).await?;
            check!(eq; applied.errors, 0, "{}", joined(&applied));
            check!(eq; applied.rows, 1);
            check!(!joined(&applied).contains("captures already journaled"));
            let current = receipts(&store)?;
            check!(eq; current.len(), 1);
            let (key, payload) = current.iter().next().ok_or("new receipt")?;
            check!(eq; payload["parser"], "athleticlive_results_v5");
            check!(eq; payload["complete"], true);
            check!(!history.contains_key(key));
            check!(eq; store.walk_table(Table::Performances)?.rows, 2);
            check!(eq; snapshot(&store, HISTORICAL_EFFECT_PHASE)?, history);
            check!(eq; snapshot(&store, HISTORICAL_ROW_PHASE)?, old_rows);
            check!(eq; snapshot(&store, super::super::CAPTURE_PHASE)?, raw);
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            let repeated = collect(&context(&store, &fetcher, &capture)?, &options).await?;
            check!(eq; repeated.errors, 0);
            check!(eq; repeated.rows, 0);
            check!(joined(&repeated).contains("captures already journaled by an earlier run: 1"));
            check!(eq; store.walk_table(Table::Performances)?.rows, 2);
            check!(eq; receipts(&store)?, current);
            check!(eq; snapshot(&store, HISTORICAL_EFFECT_PHASE)?, history);
            check!(eq; snapshot(&store, HISTORICAL_ROW_PHASE)?, old_rows);
            check!(eq; snapshot(&store, super::super::CAPTURE_PHASE)?, raw);
            Ok(())
        })
}

#[test]
fn historical_v2_claim_remains_binding_for_the_original_meet_and_provider() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let options = prepare(&dir, &store)?;
            seed_history(&store, &fetcher, &options, &capture).await?;
            let history = snapshot(&store, HISTORICAL_EFFECT_PHASE)?;
            let mut foreign = options.clone();
            foreign.meet.as_mut().ok_or("meet owner")?.tenant = "another_timer".into();
            let refused = collect(&context(&store, &fetcher, &capture)?, &foreign).await?;
            check!(eq; refused.errors, 1);
            check!(eq; refused.rows, 0);
            check!(joined(&refused)
                .contains("capture belongs to another meet/provider; refusing to project"));
            check!(eq; receipts(&store)?, BTreeMap::new());
            check!(eq; snapshot(&store, HISTORICAL_EFFECT_PHASE)?, history);
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
            Ok(())
        })
}

async fn corrupt_history(parser: &str, replace_key: bool) -> TestResult {
    let (dir, store, fetcher) = scratch()?;
    let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
    let options = prepare(&dir, &store)?;
    seed_history(&store, &fetcher, &options, &capture).await?;
    let history = snapshot(&store, HISTORICAL_EFFECT_PHASE)?;
    let mut payload = history.values().next().ok_or("historical receipt")?.clone();
    payload["parser"] = json!(parser);
    let key = if replace_key {
        "corrupt historical identity".into()
    } else {
        historical_key(&payload)?
    };
    store.journal_done(HISTORICAL_EFFECT_PHASE, &key, &payload)?;
    let corrupt = snapshot(&store, HISTORICAL_EFFECT_PHASE)?;
    let refused = collect(&context(&store, &fetcher, &capture)?, &options).await?;
    check!(eq; refused.errors, 1);
    check!(eq; refused.rows, 0);
    let detail = if replace_key {
        "receipt key disagrees with projection payload"
    } else {
        "ambiguous or incompatible LIVE projection receipt shape"
    };
    check!(joined(&refused).contains(detail), "{}", joined(&refused));
    check!(eq; receipts(&store)?, BTreeMap::new());
    check!(eq; snapshot(&store, HISTORICAL_EFFECT_PHASE)?, corrupt);
    check!(eq; store.walk_table(Table::Performances)?.rows, 1);
    Ok(())
}

#[test]
fn historical_v2_corrupt_key_is_refused_without_rewriting_it() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(corrupt_history(HISTORICAL_PARSER, true))
}

#[test]
fn historical_v2_shape_is_frozen_and_cannot_claim_the_current_parser() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(corrupt_history("athleticlive_results_v5", false))
}

#[test]
fn current_v5_parser_recognizes_xc_labels_without_an_explicit_provider_sport_flag() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let mut document: Value = serde_json::from_str(&one_row_body()?)?;
            let source = document["_source"]
                .as_object_mut()
                .ok_or("published source")?;
            source.insert("xc".into(), json!(false));
            for field in ["n", "sn", "ab", "un"] {
                source.insert(field.into(), json!("Girls XC 5K"));
            }
            let body = document.to_string();
            let doc = parse_event_document(&event_doc_url(2_150_205), &body)?;
            check!(eq; doc.kind(), EventKind::CrossCountry);
            write_schools(
                &store,
                &labels_of(&labelled_schools(&doc, UsJurisdiction::Iowa)),
            )?;
            let path = stage_capture(&dir, "label-only-xc.json", &body)?;
            let report = collect(&context(&store, &fetcher, &capture)?, &options(&path)).await?;
            check!(eq; report.errors, 0, "{}", joined(&report));
            check!(eq; report.rows, 1);
            let events: Vec<CanonicalEvent> = store.scan(Table::Events)?;
            check!(eq; events.len(), 1);
            check!(eq; events.first().ok_or("published event")?.kind, EventKind::CrossCountry);
            check!(eq; receipts(&store)?.values().next().ok_or("current receipt")?["parser"],
            "athleticlive_results_v5");
            check!(eq; snapshot(&store, super::super::CAPTURE_PHASE)?.len(), 1);
            Ok(())
        })
}

#[test]
fn historical_v2_claim_refuses_rebinding_the_capture_to_a_different_meet() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher) = scratch()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let options = prepare(&dir, &store)?;
            seed_history(&store, &fetcher, &options, &capture).await?;
            let history = snapshot(&store, HISTORICAL_EFFECT_PHASE)?;
            let mut foreign = options.clone();
            foreign.meet = Some(mits_meet());
            let refused = collect(&context(&store, &fetcher, &capture)?, &foreign).await?;
            check!(eq; refused.errors, 1);
            check!(eq; refused.rows, 0);
            check!(joined(&refused)
                .contains("capture belongs to another meet/provider; refusing to project"));
            check!(eq; receipts(&store)?, BTreeMap::new());
            check!(eq; snapshot(&store, HISTORICAL_EFFECT_PHASE)?, history);
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
            Ok(())
        })
}
