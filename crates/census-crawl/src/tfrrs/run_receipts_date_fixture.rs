use super::*;

pub(super) async fn synthetic_v4_date_projection(
    ctx: &AdapterContext<'_>,
    bindings: &[CanonicalSchool],
    historical_body: &[u8],
) -> TestResult<(String, census_store::Receipt)> {
    let recording = crate::recording::Recording::new();
    let staged = AdapterContext {
        recording: Some(&recording),
        ..context(ctx.store, ctx.fetcher)?
    };
    let drafted = collect(&staged).await?;
    check!(eq; (drafted.errors, drafted.rows, drafted.requests), (0, 20, 0), "{:?}", drafted.notes);
    let mut recorded = recording.drain();
    let expected_witnesses = assert_v2_witnesses(&recorded)?;
    let completion = recorded
        .journal
        .iter_mut()
        .find(|entry| entry.phase == PHASE)
        .ok_or("staged projection completion")?;
    let old_context = digest(&(
        DATE_PREDECESSOR_PHASE,
        ctx.performance_as_of,
        ctx.school_year,
        bindings,
    ))?;
    let body_digest = content_digest(historical_body);
    let old_key = key(&old_context, URL, &body_digest)?;
    completion.phase = DATE_PREDECESSOR_PHASE.into();
    completion.key = old_key.clone();
    completion.payload["body_sha256"] = serde_json::json!(body_digest);
    completion.payload["projection_context"] = serde_json::json!(old_context);
    seed(ctx.fetcher, historical_body)?;
    let applied = recorded.apply(ctx.store, "tfrrs-synthetic-date-v4-fixture")?;
    check!(applied.written());
    check!(eq; snapshot(ctx.store, "crawl_effect_receipts_v1")?, expected_witnesses);
    check!(snapshot(ctx.store, PHASE)?.is_empty());
    Ok((old_key, applied.receipt().clone()))
}

pub(super) fn physical_rows(store: &Store) -> TestResult<Vec<(Table, census_store::TableWalk)>> {
    [
        Table::Schools,
        Table::Meets,
        Table::Teams,
        Table::Athletes,
        Table::SourceObservations,
        Table::Events,
        Table::Performances,
    ]
    .into_iter()
    .map(|table| Ok((table, store.walk_table(table)?)))
    .collect()
}

pub(super) fn entity_digest(store: &Store) -> TestResult<String> {
    Ok(store.snapshot().tables_digest(&[
        Table::Schools, Table::Meets, Table::Teams, Table::Athletes,
        Table::SourceObservations, Table::Events, Table::Performances,
    ])?)
}

pub(super) fn assert_v2_witnesses(recorded: &crate::recording::Recorded) -> TestResult<BTreeMap<String, serde_json::Value>> {
    use census_domain::model::{CanonicalAthlete, CanonicalMeet, CanonicalTeam, SourceObservation};
    let mut expected = BTreeMap::new();
    for batch in &recorded.rows {
        let entries = match batch.table {
            Table::Schools => row_witnesses::<CanonicalSchool>(batch)?,
            Table::Meets => row_witnesses::<CanonicalMeet>(batch)?,
            Table::Teams => row_witnesses::<CanonicalTeam>(batch)?,
            Table::Athletes => row_witnesses::<CanonicalAthlete>(batch)?,
            Table::SourceObservations => row_witnesses::<SourceObservation>(batch)?,
            Table::Events => row_witnesses::<CanonicalEvent>(batch)?,
            Table::Performances => row_witnesses::<CanonicalPerformance>(batch)?,
            _ => return Err("unexpected table in TFRRS fixture".into()),
        };
        expected.extend(entries);
    }
    let actual: BTreeMap<_, _> = recorded.journal.iter()
        .filter(|entry| entry.phase == "crawl_effect_receipts_v1")
        .map(|entry| (entry.key.clone(), entry.payload.clone())).collect();
    check!(eq; actual, expected);
    check!(eq; recorded.effects.len(), expected.len());
    check!(eq; recorded.journal.len(), expected.len() + 1);
    Ok(expected)
}

pub(super) fn row_witnesses<T: serde::de::DeserializeOwned + serde::Serialize>(
    batch: &crate::recording::RecordedBatch,
) -> TestResult<Vec<(String, serde_json::Value)>> {
    batch.rows.iter().map(|value| {
        let row: T = serde_json::from_value(value.clone())?;
        let bytes = serde_json::to_vec(&("tfrrs_pages_v2", batch.table.file(), row))?;
        let digest = content_digest(&bytes);
        Ok((format!("tfrrs_pages_v2:{digest}"), serde_json::json!({"digest": digest})))
    }).collect()
}

pub(super) fn capture_bytes(
    fetcher: &crate::net::Fetcher,
    body: &[u8],
) -> TestResult<BTreeMap<std::path::PathBuf, Vec<u8>>> {
    let digest = content_digest(body);
    let root = fetcher.cache_dir().join("archive");
    let body_path = root.join("bodies").join(format!("{digest}.body"));
    check!(eq; std::fs::read(&body_path)?, body);
    let mut captured = BTreeMap::from([(body_path, body.to_vec())]);
    for entry in std::fs::read_dir(root.join("captures").join(&digest))? {
        let path = entry?.path();
        let bytes = std::fs::read(&path)?;
        let meta: CacheMeta = serde_json::from_slice(&bytes)?;
        check!(eq; meta.url.as_str(), URL);
        check!(eq; meta.content_digest.as_str(), digest.as_str());
        check!(eq; meta.bytes, body.len());
        check!(eq; meta.fetched_at.as_str(), "2026-09-22T03:55:31Z");
        captured.insert(path, bytes);
    }
    check!(eq; captured.len(), 2);
    Ok(captured)
}
