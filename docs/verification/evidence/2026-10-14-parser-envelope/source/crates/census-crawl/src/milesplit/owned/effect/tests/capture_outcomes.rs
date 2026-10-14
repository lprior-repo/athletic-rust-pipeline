use super::*;

#[test]
fn recording_route_retains_all_capture_chunks_before_source_receipts_are_applied() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let mut body = crate::milesplit::owned::tests::TROY.to_vec();
            body.resize(CAPTURE_CHUNK_BYTES * 2 + 7, b' ');
            seed(&fetcher, &reference, &body)?;
            let recording = crate::recording::Recording::new();
            let ctx = AdapterContext {
                recording: Some(&recording),
                ..context(&store, &fetcher)?
            };
            let outcome = read_owned_meet(&ctx, &reference).await?;
            check!(eq; outcome.capture().body, body);
            check!(eq;
                store
                    .journal_keys(OWNED_MEET_PHASE)
                    ?,
                std::collections::HashSet::new()
            );
            let recorded = recording.drain();
            let mut chunks: Vec<_> = recorded
                .journal
                .iter()
                .filter(|entry| {
                    entry.phase == OWNED_CAPTURE_PHASE && entry.payload.get("raw_base64").is_some()
                })
                .map(|entry| {
                    Ok((
                        entry.payload["chunk_index"]
                            .as_u64()
                            .ok_or("chunk ordinal")?,
                        entry,
                    ))
                })
                .collect::<TestResult<Vec<_>>>()?;
            chunks.sort_by_key(|(ordinal, _)| *ordinal);
            check!(eq; chunks.len(), 3);
            let restored = chunks.iter().try_fold(
                Vec::new(),
                |mut bytes, (_, entry)| -> TestResult<Vec<u8>> {
                    bytes.extend(
                        STANDARD.decode(
                            entry.payload["raw_base64"]
                                .as_str()
                                .ok_or("encoded capture")?,
                        )?,
                    );
                    Ok(bytes)
                },
            )?;
            check!(eq; restored, body);
            check!(recorded
                .journal
                .iter()
                .any(|entry| entry.phase == OWNED_MEET_PHASE && entry.key.starts_with("parsed/")));
            Ok(())
        })
}

#[test]
fn a_recorded_cooldown_refuses_the_meet_without_closing_it() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let live = Fetcher::new(
                dir.path().join("live"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                vec!["milesplit.com".into()],
            )?;
            let cooldown = live
                .record_access_condition(
                    "al.milesplit.com",
                    census_domain::model::AccessBlockKind::RateLimited,
                    429,
                    Some(60),
                    "recorded while the host was cooled down",
                )
                .await;
            check!(cooldown.is_blocking(&crate::net::now_iso8601()));
            let error = match read_owned_meet(&context(&store, &live)?, &reference).await {
                Err(error) => error,
                Ok(_) => return Err("a host inside a cooldown cannot acquire the meet".into()),
            };
            check!(
                matches!(
                    &error,
                    CrawlError::Fetch(crate::net::FetchError::Cooldown { host })
                        if host == "al.milesplit.com"
                ),
                "the refusal is a typed cooldown: {error:?}"
            );
            check!(error.retryable(), "a cooldown is temporary, not terminal");
            check!(
                eq;
                store.journal_payloads(OWNED_MEET_PHASE)?.len(),
                0,
                "a cooldown writes no completion receipt, so the meet stays owed"
            );
            seed(&fetcher, &reference, crate::milesplit::owned::tests::TROY)?;
            let outcome = read_owned_meet(&context(&store, &fetcher)?, &reference).await?;
            parsed(&outcome)?;
            check!(
                !store.journal_payloads(OWNED_MEET_PHASE)?.is_empty(),
                "the retry journals the completed meet under the same store"
            );
            Ok(())
        })
}
