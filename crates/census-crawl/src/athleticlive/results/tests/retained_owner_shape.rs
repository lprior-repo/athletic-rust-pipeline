use super::*;
use serde_json::{json, Value};

#[derive(Debug, Clone, Copy)]
enum Revision {
    Historical,
    Current,
}

impl Revision {
    fn phases(self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::Historical => (
                "athleticlive_results_effect_v2",
                "athleticlive_results_v4",
                "athleticlive_projection_v4",
            ),
            Self::Current => (
                super::super::EFFECT_PHASE,
                super::super::PARSER,
                super::super::ROW_PHASE,
            ),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum Damage {
    MissingDigest,
    MalformedDigest,
    UppercaseDigest,
    MissingBytes,
    NegativeBytes,
    TextBytes,
    ConflictingBytes,
    MissingMetadataDigest,
    ConflictingMetadataDigest,
    MissingMetadataBytes,
    ConflictingMetadataBytes,
}

fn damage(payload: &mut Value, case: Damage) -> TestResult {
    let bytes = payload
        .get("bytes")
        .and_then(Value::as_u64)
        .ok_or("writer bytes")?;
    let changed_bytes = bytes.checked_add(1).ok_or("changed byte count")?;
    let (field, replacement) = match case {
        Damage::MissingDigest | Damage::MissingMetadataDigest => ("digest", None),
        Damage::MalformedDigest => ("digest", Some(json!("not-a-content-digest"))),
        Damage::UppercaseDigest => ("digest", Some(json!("A".repeat(64)))),
        Damage::ConflictingMetadataDigest => ("digest", Some(json!("0".repeat(64)))),
        Damage::MissingBytes | Damage::MissingMetadataBytes => ("bytes", None),
        Damage::NegativeBytes => ("bytes", Some(json!(-1))),
        Damage::TextBytes => ("bytes", Some(json!(bytes.to_string()))),
        Damage::ConflictingBytes | Damage::ConflictingMetadataBytes => {
            ("bytes", Some(json!(changed_bytes)))
        }
    };
    let (object, field) = match case {
        Damage::MissingMetadataDigest | Damage::ConflictingMetadataDigest => (
            payload
                .get_mut("metadata")
                .and_then(Value::as_object_mut)
                .ok_or("writer metadata")?,
            "content_digest",
        ),
        Damage::MissingMetadataBytes | Damage::ConflictingMetadataBytes => (
            payload
                .get_mut("metadata")
                .and_then(Value::as_object_mut)
                .ok_or("writer metadata")?,
            field,
        ),
        _ => (payload.as_object_mut().ok_or("writer receipt")?, field),
    };
    match replacement {
        Some(value) => {
            object.insert(field.into(), value);
        }
        None => {
            object.remove(field).ok_or("field being removed")?;
        }
    }
    if matches!(case, Damage::UppercaseDigest) {
        payload
            .get_mut("metadata")
            .and_then(Value::as_object_mut)
            .ok_or("writer metadata")?
            .insert("content_digest".into(), json!("A".repeat(64)));
    }
    Ok(())
}

fn receipt_key(payload: &Value) -> TestResult<String> {
    let mut identity = payload.as_object().ok_or("writer receipt")?.clone();
    identity.remove("path").ok_or("writer path")?;
    identity.remove("metadata").ok_or("writer metadata")?;
    for field in ["digest", "bytes"] {
        identity.entry(field).or_insert(Value::Null);
    }
    let digest = census_domain::model::serialized_digest(&Value::Object(identity))?;
    let meet = payload
        .get("meet")
        .and_then(Value::as_str)
        .ok_or("writer meet")?;
    let role = payload
        .get("role")
        .and_then(Value::as_str)
        .ok_or("writer role")?;
    Ok(format!("{meet}:{role}:{digest}"))
}

fn snapshot(store: &Store, phase: &str) -> TestResult<BTreeMap<String, Value>> {
    store
        .journal_keys(phase)?
        .into_iter()
        .map(|key| {
            let payload = store
                .journal_payload(phase, &key)?
                .ok_or("retained receipt")?;
            Ok((key, payload))
        })
        .collect()
}

fn prepare(dir: &tempfile::TempDir, store: &Store) -> TestResult<(ResultOptions, ResultOptions)> {
    let mut document: Value = serde_json::from_str(XC_STATE)?;
    document
        .pointer_mut("/_source/r")
        .and_then(Value::as_array_mut)
        .ok_or("published rows")?
        .truncate(1);
    let body = document.to_string();
    let parsed = parse_event_document(&event_doc_url(2_150_205), &body)?;
    write_schools(
        store,
        &labels_of(&labelled_schools(&parsed, UsJurisdiction::Iowa)),
    )?;
    let original_path = stage_capture(dir, "original-owner.json", &body)?;
    let alias_path = stage_capture(dir, "aliased-owner.json", &body)?;
    let original = ResultOptions {
        documents: vec![original_path],
        ..ResultOptions::for_meet(state_meet(), OBSERVED_ON)
    };
    let mut alias = original.clone();
    alias.documents = vec![alias_path];
    alias.meet.as_mut().ok_or("alias meet")?.tenant = "another_timer".into();
    Ok((original, alias))
}

async fn retain(
    store: &Store,
    fetcher: &Fetcher,
    original: &ResultOptions,
    revision: Revision,
    corruption: Option<Damage>,
) -> TestResult {
    let recording = crate::recording::Recording::new();
    let ctx = AdapterContext {
        recording: Some(&recording),
        ..context(store, fetcher)?
    };
    let drafted = collect(&ctx, original).await?;
    check!(eq; (drafted.errors, drafted.rows), (0, 1), "{}", joined(&drafted));
    let (phase, parser, row_phase) = revision.phases();
    let mut batch = store.write_batch();
    let recorded = recording.drain();
    for rows in recorded.rows {
        batch.record_many(rows.table, &rows.rows)?;
    }
    for entry in recorded.journal {
        if entry.phase == super::super::EFFECT_PHASE {
            let mut payload = entry.payload;
            check!(eq; payload.get("digest"), payload.pointer("/metadata/content_digest"));
            check!(eq; payload.get("bytes"), payload.pointer("/metadata/bytes"));
            check!(eq; receipt_key(&payload)?, entry.key);
            payload
                .as_object_mut()
                .ok_or("writer payload")?
                .insert("parser".into(), json!(parser));
            if let Some(corruption) = corruption {
                damage(&mut payload, corruption)?;
            }
            batch.journal_done(phase, &receipt_key(&payload)?, &payload)?;
        } else if entry.phase == super::super::ROW_PHASE {
            batch.journal_done(row_phase, &entry.key, &entry.payload)?;
        } else {
            batch.journal_done(&entry.phase, &entry.key, &entry.payload)?;
        }
    }
    batch.commit()?;
    Ok(())
}

async fn refused_corruption(revision: Revision, corruption: Damage) -> TestResult {
    let (dir, store, fetcher) = scratch()?;
    let (original, alias) = prepare(&dir, &store)?;
    retain(&store, &fetcher, &original, revision, Some(corruption)).await?;
    let (phase, _, _) = revision.phases();
    let before = snapshot(&store, phase)?;
    let current = receipts(&store)?;
    let raw = snapshot(&store, super::super::CAPTURE_PHASE)?;
    check!(eq; before.len(), 1);
    check!(eq; raw.len(), 1);
    let refused = collect(&context(&store, &fetcher)?, &alias).await?;
    check!(eq; (refused.errors, refused.rows), (1, 0), "{revision:?}/{corruption:?}: {}", joined(&refused));
    check!(
        joined(&refused).contains("ambiguous or incompatible LIVE projection receipt shape"),
        "{revision:?}/{corruption:?}: {}",
        joined(&refused)
    );
    check!(eq; snapshot(&store, phase)?, before);
    check!(eq; receipts(&store)?, current);
    check!(eq; snapshot(&store, super::super::CAPTURE_PHASE)?, raw);
    check!(eq; store.walk_table(Table::Performances)?.rows, 1);
    drop(store);
    let store = Store::open(dir.path().join("store"))?;
    check!(eq; snapshot(&store, phase)?, before);
    check!(eq; snapshot(&store, super::super::CAPTURE_PHASE)?, raw);
    let refused = collect(&context(&store, &fetcher)?, &alias).await?;
    check!(eq; (refused.errors, refused.rows), (1, 0));
    check!(joined(&refused).contains("ambiguous or incompatible LIVE projection receipt shape"));
    check!(eq; snapshot(&store, phase)?, before);
    check!(eq; receipts(&store)?, current);
    check!(eq; store.walk_table(Table::Performances)?.rows, 1);
    Ok(())
}

async fn corruptions(revision: Revision) -> TestResult {
    for case in [
        Damage::MissingDigest,
        Damage::MalformedDigest,
        Damage::UppercaseDigest,
        Damage::MissingBytes,
        Damage::NegativeBytes,
        Damage::TextBytes,
        Damage::ConflictingBytes,
        Damage::MissingMetadataDigest,
        Damage::ConflictingMetadataDigest,
        Damage::MissingMetadataBytes,
        Damage::ConflictingMetadataBytes,
    ] {
        refused_corruption(revision, case).await?;
    }
    Ok(())
}

#[test]
fn malformed_frozen_v2_capture_fields_are_refused_before_path_alias_owner_matching() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(corruptions(Revision::Historical))
}

#[test]
fn malformed_current_v3_capture_fields_are_refused_before_path_alias_owner_matching() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(corruptions(Revision::Current))
}

#[test]
fn valid_writer_capture_fields_keep_both_retained_revisions_owner_bound_on_path_aliases(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        for revision in [Revision::Historical, Revision::Current] {
            let (dir, store, fetcher) = scratch()?;
            let (original, alias) = prepare(&dir, &store)?;
            retain(&store, &fetcher, &original, revision, None).await?;
            let (phase, _, _) = revision.phases();
            let before = snapshot(&store, phase)?;
            let raw = snapshot(&store, super::super::CAPTURE_PHASE)?;
            let refused = collect(&context(&store, &fetcher)?, &alias).await?;
            check!(eq; (refused.errors, refused.rows), (1, 0), "{revision:?}: {}", joined(&refused));
            check!(joined(&refused).contains("capture belongs to another meet/provider; refusing to project"),
                "{revision:?}: {}", joined(&refused));
            check!(eq; snapshot(&store, phase)?, before);
            check!(eq; snapshot(&store, super::super::CAPTURE_PHASE)?, raw);
            check!(eq; store.walk_table(Table::Performances)?.rows, 1);
        }
        Ok(())
    })
}
