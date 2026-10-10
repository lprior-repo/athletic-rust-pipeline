use super::super::*;

pub(super) fn assert_archived_metadata(store: &Store, body: &[u8]) -> TestResult {
    use base64::{engine::general_purpose::STANDARD, Engine};
    let digest = crate::net::cache::content_digest(body);
    let captures = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
    let mut chunks: Vec<_> = captures
        .iter()
        .filter(|row| {
            row["role"] == "raw_metadata"
                && row["capture"]["content_digest"] == digest
                && row.get("raw_base64").is_some()
        })
        .collect();
    chunks.sort_by_key(|row| row["chunk_index"].as_u64());
    let retained =
        chunks
            .into_iter()
            .try_fold(Vec::new(), |mut bytes, row| -> TestResult<Vec<u8>> {
                bytes.extend(STANDARD.decode(row["raw_base64"].as_str().ok_or("captured bytes")?)?);
                Ok(bytes)
            })?;
    check!(eq; retained, body);
    check!(eq; crate::net::cache::content_digest(&retained), digest);
    Ok(())
}

#[test]
fn projected_raw_metadata_retains_its_own_acquisition_time_and_replays_physical_facts_exactly(
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            seed_owned(&fetcher, &reference, TROY)?;
            let (body_path, meta_path) =
                fetcher.cache_paths(&Fetcher::key_for("GET", &reference.url, ""));
            crate::net::cache::write_cache(
                &body_path,
                &meta_path,
                FEMALE_RAW,
                &crate::net::cache::CacheMeta {
                    redirects: Vec::new(),
                    representation: crate::net::RepresentationHeaders::default(),
                    url: reference.url.clone(),
                    method: "GET".into(),
                    status: 200,
                    content_digest: crate::net::cache::content_digest(FEMALE_RAW),
                    bytes: FEMALE_RAW.len(),
                    fetched_at: "2026-10-02T00:00:00Z".into(),
                    ..crate::net::cache::CacheMeta::default()
                },
            )?;
            let report = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (report.rows, report.errors), (3, 0));
            let marks: Vec<CanonicalPerformance> = store.scan(Table::Performances)?;
            let long = marks
                .iter()
                .find(|row| row.source_key == "milesplit_result:201782263")
                .ok_or("owned Spann result")?;
            let raw = long
                .evidence
                .iter()
                .find(|evidence| evidence.source.url.as_deref() == Some(reference.url.as_str()))
                .ok_or("captured raw metadata claim")?;
            check!(eq; raw.observed_on, "2026-10-02T00:00:00Z");
            check!(eq; long.evidence[0].observed_on, "2026-10-01T23:44:16Z");
            let note: serde_json::Value =
                serde_json::from_str(raw.note.as_deref().ok_or("bound provenance")?)?;
            check!(eq;
                note["raw_metadata_capture"]["content_digest"],
                crate::net::cache::content_digest(FEMALE_RAW)
            );
            check!(eq; note["raw_metadata_capture"]["fetched_at"], raw.observed_on);
            let manifest = store
                .journal_payload(
                    crate::milesplit::OWNED_CAPTURE_PHASE,
                    note["raw_metadata_capture"]["archive_manifest"]
                        .as_str()
                        .ok_or("archive locator")?,
                )?
                .ok_or("retained manifest")?;
            check!(eq;
                manifest["capture"]["content_digest"],
                note["raw_metadata_capture"]["content_digest"]
            );
            check!(eq; manifest["capture"]["fetched_at"], raw.observed_on);
            assert_archived_metadata(&store, FEMALE_RAW)?;
            let physical = super::super::replay::physical(&store)?;
            let entities = super::super::replay::entities(&store)?;
            let receipts = store.journal_payloads(super::super::super::super::RESULT_SET_PHASE)?;
            let captures = store.journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)?;
            let replay = crate::milesplit::collect_result_sets(
                &context(&store, &fetcher)?,
                &options(&reference),
            )
            .await?;
            check!(eq; (replay.rows, replay.errors), (3, 0));
            check!(eq; super::super::replay::physical(&store)?, physical);
            check!(eq; super::super::replay::entities(&store)?, entities);
            check!(eq;
                store
                    .journal_payloads(super::super::super::super::RESULT_SET_PHASE)
                    ?,
                receipts
            );
            check!(eq;
                store
                    .journal_payloads(crate::milesplit::OWNED_CAPTURE_PHASE)
                    ?,
                captures
            );
            Ok(())
        })
}
