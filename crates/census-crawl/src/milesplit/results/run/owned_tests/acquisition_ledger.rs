use super::*;
use crate::milesplit::{OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use crate::net::cache::{content_digest, write_cache, CacheMeta};
use std::collections::BTreeMap;

fn seed_acquisition(
    fetcher: &Fetcher,
    reference: &ResultSetRef,
    body: &[u8],
    fetched_at: &str,
    response_url: Option<&str>,
) -> TestResult {
    let url = crate::milesplit::fetch::owned_meet_url(reference)?;
    let (body_path, meta_path) = fetcher.cache_paths(&Fetcher::key_for("GET", &url, ""));
    std::fs::create_dir_all(fetcher.cache_dir())?;
    write_cache(
        &body_path,
        &meta_path,
        body,
        &CacheMeta {
        redirects: Vec::new(),
            representation: crate::net::RepresentationHeaders::default(),
            url,
            response_url: response_url.map(str::to_owned),
            method: "GET".into(),
            status: 200,
            content_digest: content_digest(body),
            bytes: body.len(),
            fetched_at: fetched_at.into(),
            ..CacheMeta::default()
        },
    )?;
    Ok(())
}

fn journal(store: &Store, phase: &str) -> TestResult<BTreeMap<String, serde_json::Value>> {
    store
        .journal_keys(phase)?
        .into_iter()
        .map(|key| {
            let payload = store.journal_payload(phase, &key)?.ok_or("existing key")?;
            Ok((key, payload))
        })
        .collect()
}

fn assert_retained(
    store: &Store,
    phase: &str,
    prior: &BTreeMap<String, serde_json::Value>,
) -> TestResult {
    for (key, payload) in prior {
        check!(eq;
            store.journal_payload(phase, key)?,
            Some(payload.clone()),
            "historical receipt changed: {phase}/{key}"
        );
    }
    Ok(())
}

fn partial_body() -> TestResult<Vec<u8>> {
    let mut source: serde_json::Value = serde_json::from_slice(TROY)?;
    source["data"][0]["athleteId"] = serde_json::Value::Null;
    Ok(serde_json::to_vec(&source)?)
}

#[test]
fn same_bytes_reacquisition_binds_new_owned_manifest_without_mutating_original_archive(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            seed_metadata(&fetcher, &reference)?;
            let first = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (first.rows, first.errors), (3, 0));
            let originals = journal(&store, OWNED_CAPTURE_PHASE)?;
            let original_receipts = journal(&store, super::super::super::RESULT_SET_PHASE)?;
            let old_key = format!("725218/{}/manifest", content_digest(TROY));
            check!(eq;
                originals[&old_key]["capture"]["fetched_at"],
                "2026-10-01T23:44:16Z"
            );
            let original_archive = original_receipts
                .values()
                .find(|row| row["disposition"] == "projection_applied")
                .ok_or("first projection")?["owned_capture_archive"]
                .clone();
            let final_url = crate::milesplit::fetch::owned_meet_url(&reference)?;
            seed_acquisition(
                &fetcher,
                &reference,
                TROY,
                "2026-10-02T00:00:00Z",
                Some(&final_url),
            )?;
            let second = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (second.rows, second.errors), (3, 0));
            let receipts = journal(&store, super::super::super::RESULT_SET_PHASE)?;
            let receipt = receipts
                .values()
                .find(|row| {
                    row["disposition"] == "projection_applied"
                        && row["capture"]["fetched_at"] == "2026-10-02T00:00:00Z"
                })
                .ok_or("new acquisition projection receipt")?;
            let archive_key = receipt["owned_capture_archive"]
                .as_str()
                .ok_or("archive locator")?;
            let manifest = store
                .journal_payload(OWNED_CAPTURE_PHASE, archive_key)?
                .ok_or("new acquisition retained")?;
            check!(eq;
                manifest["capture"]["fetched_at"],
                receipt["capture"]["fetched_at"]
            );
            check!(eq; manifest["capture"]["response_url"], final_url);
            check!(eq; manifest["capture"]["content_digest"], content_digest(TROY));
            check!(eq; manifest["content_manifest"], old_key);
            let prefix = manifest["chunk_key_prefix"]
                .as_str()
                .ok_or("shared byte locator")?;
            let chunk_count = manifest["chunks"].as_u64().ok_or("shared chunk count")?;
            let retained = (0..chunk_count).try_fold(
                Vec::new(),
                |mut bytes, index| -> TestResult<Vec<u8>> {
                    use base64::{engine::general_purpose::STANDARD, Engine};
                    let chunk = store
                        .journal_payload(OWNED_CAPTURE_PHASE, &format!("{prefix}/{index}"))?
                        .ok_or("original chunk retained")?;
                    bytes.extend(
                        STANDARD.decode(
                            chunk["raw_base64"]
                                .as_str()
                                .ok_or("encoded original bytes")?,
                        )?,
                    );
                    Ok(bytes)
                },
            )?;
            check!(eq; retained, TROY);
            check!(ne; receipt["owned_capture_archive"], original_archive);
            check!(archive_key.starts_with("acquisition/725218/"));
            assert_retained(&store, OWNED_CAPTURE_PHASE, &originals)?;
            assert_retained(
                &store,
                super::super::super::RESULT_SET_PHASE,
                &original_receipts,
            )?;
            let captures = journal(&store, OWNED_CAPTURE_PHASE)?;
            let chunks = |rows: &BTreeMap<String, serde_json::Value>| {
                rows.iter()
                    .filter(|(_, row)| {
                        row.get("raw_base64").is_some()
                            && row["capture"]["content_digest"] == content_digest(TROY)
                    })
                    .map(|(key, row)| (key.clone(), row.clone()))
                    .collect::<BTreeMap<_, _>>()
            };
            check!(eq; chunks(&captures), chunks(&originals));
            Ok(())
        })
}

#[test]
fn identical_partial_owned_replay_stages_no_rows_rejections_or_summary_replacements() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, &partial_body()?)?;
            seed_metadata(&fetcher, &reference)?;
            let first = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; first.errors, 1);
            let originals = journal(&store, OWNED_MEET_PHASE)?;
            let captures = journal(&store, OWNED_CAPTURE_PHASE)?;
            let physical = super::replay::physical(&store)?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let replay = crate::milesplit::collect_result_sets(&ctx, &options(&reference)).await?;
            check!(eq; replay.errors, 1);
            let recorded = recording.drain();
            let owned: Vec<_> = recorded
                .journal
                .iter()
                .filter(|entry| {
                    entry.phase == OWNED_MEET_PHASE || entry.phase == OWNED_CAPTURE_PHASE
                })
                .collect();
            check!(eq; owned, Vec::<&crate::recording::RecordedJournal>::new());
            apply(&store, &recorded)?;
            check!(eq; super::replay::physical(&store)?, physical);
            check!(eq; journal(&store, OWNED_MEET_PHASE)?, originals);
            check!(eq; journal(&store, OWNED_CAPTURE_PHASE)?, captures);
            Ok(())
        })
}

#[test]
fn distinct_partial_owned_acquisitions_keep_both_receipts_and_original_valid_rows() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let body = partial_body()?;
            seed_owned(&fetcher, &reference, &body)?;
            seed_metadata(&fetcher, &reference)?;
            let first = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; first.errors, 1);
            let originals = journal(&store, OWNED_MEET_PHASE)?;
            let original_captures = journal(&store, OWNED_CAPTURE_PHASE)?;
            let final_url = crate::milesplit::fetch::owned_meet_url(&reference)?;
            seed_acquisition(
                &fetcher,
                &reference,
                &body,
                "2026-10-02T00:00:00Z",
                Some(&final_url),
            )?;
            let second = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; second.errors, 1);
            assert_retained(&store, OWNED_MEET_PHASE, &originals)?;
            assert_retained(&store, OWNED_CAPTURE_PHASE, &original_captures)?;
            let retained = journal(&store, OWNED_MEET_PHASE)?;
            let times = retained
                .values()
                .filter(|row| row["disposition"] == "partial")
                .map(|row| {
                    Ok(row["capture"]["fetched_at"]
                        .as_str()
                        .ok_or("acquisition time")?)
                })
                .collect::<TestResult<std::collections::BTreeSet<_>>>()?;
            check!(eq;
                times,
                std::collections::BTreeSet::from(["2026-10-01T23:44:16Z", "2026-10-02T00:00:00Z"])
            );
            let original_rows: BTreeMap<_, _> = originals
                .iter()
                .filter(|(_, row)| row.get("result_id").is_some() || row["locator"] == "data[0]")
                .map(|(key, row)| (key.clone(), row.clone()))
                .collect();
            assert_retained(&store, OWNED_MEET_PHASE, &original_rows)?;
            let valid = retained
                .values()
                .filter_map(|row| row["result_id"].as_u64())
                .collect::<std::collections::BTreeSet<_>>();
            check!(eq;
                valid,
                std::collections::BTreeSet::from([201782277, 201782806])
            );
            let rejection = retained
                .values()
                .find(|row| row["locator"] == "data[0]")
                .ok_or("original rejected locator retained")?;
            check!(eq; rejection["kind"], "missing_owner");
            Ok(())
        })
}
