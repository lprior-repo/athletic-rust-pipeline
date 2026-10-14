use super::*;

#[test]
fn fetch_and_owned_reader_agree_on_resource_origin_and_preserve_the_acquired_body() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let body = serde_json::to_vec(
                &serde_json::json!({"data": [], "providerExtension": vec![0; 20_001]}),
            )?;
            seed(&fetcher, &reference, &body)?;
            let ctx = context(&store, &fetcher)?;
            let fetched =
                crate::milesplit::fetch_owned_meet(&ctx, &reference, &ctx.fetch_options()).await;
            let read = read_owned_meet(&ctx, &reference).await;
            for (outcome, receipt) in [
                (fetched, OwnedResourceReceipt::NotRequested),
                (read, OwnedResourceReceipt::Retained),
            ] {
                match outcome {
                    Err(CrawlError::OwnedParseResource(OwnedParseResourceFailure::Limit {
                        resource: crate::milesplit::OwnedParseResource::ArrayValues,
                        locator: crate::milesplit::OwnedParseLocator::Byte { offset },
                        requested,
                        limit,
                        retention:
                            crate::milesplit::OwnedCaptureRetention::Captured {
                                capture,
                                immutable_body,
                                receipt: retained_receipt,
                            },
                    })) => {
                        check!(eq; (requested, limit), (20_001, 20_000));
                        check!(eq; body.get(offset), Some(&b'0'));
                        check!(eq; retained_receipt, receipt);
                        check!(eq; capture.url, owned_meet_url(&reference)?);
                        check!(eq; capture.fetched_at, "2026-10-01T23:44:16Z");
                        check!(eq; capture.content_digest, content_digest(&body));
                        check!(eq; capture.bytes, body.len());
                        check!(eq; capture.body.len(), 0);
                        check!(eq; std::fs::read(immutable_body)?, body);
                    }
                    other => {
                        return Err(format!(
                            "resource failure must retain its origin and acquired bytes: {other:?}"
                        )
                        .into())
                    }
                }
            }
            check!(store
                .journal_keys(OWNED_MEET_PHASE)?
                .iter()
                .all(|key| key.starts_with("partial/resource-v2/")));
            check!(!store
                .journal_payloads(OWNED_MEET_PHASE)?
                .iter()
                .any(|row| row.get("verdict").is_some()));
            Ok(())
        })
}

#[test]
fn genuine_malformed_fetch_and_owned_read_keep_exact_historical_verdict_bytes() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let body = b"{\"data\":null}";
            seed(&fetcher, &reference, body)?;
            let ctx = context(&store, &fetcher)?;
            let expected = b"{\"disposition\":\"malformed\",\"detail\":\"expected a JSON object with a data array\"}";
            let fetched = crate::milesplit::fetch_owned_meet(&ctx, &reference, &ctx.fetch_options()).await?;
            let read = read_owned_meet(&ctx, &reference).await?;
            check!(eq; serde_json::to_vec(fetched.verdict())?, expected);
            check!(eq; serde_json::to_vec(read.verdict())?, expected);
            check!(eq; read.capture().body, body);
            let payloads = store.journal_payloads(OWNED_MEET_PHASE)?;
            let summary = payloads.first().ok_or("malformed capture receipt")?;
            check!(eq; summary["verdict"], serde_json::from_slice::<serde_json::Value>(expected)?);
            check!(store.journal_keys(OWNED_MEET_PHASE)?.iter().all(|key| key.starts_with("partial/acquisition/")));
            check!(store.journal_keys(OWNED_CAPTURE_PHASE)?.contains(&format!("725218/{}/manifest", content_digest(body))));
            Ok(())
        })
}

#[test]
fn resource_capture_network_metadata_to_cache_replay_preserves_every_original_payload() -> TestResult
{
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (dir, store, fetcher, reference) = setup()?;
            let body = serde_json::to_vec(
                &serde_json::json!({"data": [], "providerExtension": vec![0; 20_001]}),
            )?;
            seed(&fetcher, &reference, &body)?;
            let ctx = context(&store, &fetcher)?;
            let mut capture = fetch_owned_capture(&ctx, &reference, &ctx.fetch_options())
                .await?
                .into_capture();
            capture.from_cache = false;
            let failure = match parse_owned_meet(&body, 725218) {
                Err(OwnedMeetParseError::Resource(failure)) => failure,
                outcome => return Err(format!("resource fixture: {outcome:?}").into()),
            };
            match retain_limited_capture(&ctx, &reference, capture, failure.into()) {
                Err(CrawlError::OwnedParseResource(failure)) => match failure.retention() {
                    crate::milesplit::OwnedCaptureRetention::Captured {
                        capture, receipt, ..
                    } => {
                        check!(eq; capture.from_cache, false);
                        check!(eq; *receipt, OwnedResourceReceipt::Retained);
                    }
                    other => return Err(format!("first capture retention: {other:?}").into()),
                },
                outcome => {
                    return Err(format!("resource cannot become a verdict: {outcome:?}").into())
                }
            }
            let phases = [OWNED_MEET_PHASE, OWNED_CAPTURE_PHASE];
            let snapshot = |store: &Store| -> TestResult<
                Vec<std::collections::BTreeMap<String, serde_json::Value>>,
            > {
                phases
                    .iter()
                    .map(|phase| {
                        store
                            .journal_keys(phase)?
                            .into_iter()
                            .map(|key| {
                                Ok((
                                    key.clone(),
                                    store
                                        .journal_payload(phase, &key)?
                                        .ok_or("original receipt")?,
                                ))
                            })
                            .collect()
                    })
                    .collect()
            };
            let original = snapshot(&store)?;
            drop(store);
            let store = Store::open(dir.path().join("store"))?;
            match read_owned_meet(&context(&store, &fetcher)?, &reference).await {
                Err(CrawlError::OwnedParseResource(failure)) => match failure.retention() {
                    crate::milesplit::OwnedCaptureRetention::Captured {
                        capture,
                        immutable_body,
                        receipt,
                    } => {
                        check!(eq; capture.from_cache, true);
                        check!(eq; *receipt, OwnedResourceReceipt::Retained);
                        check!(eq; std::fs::read(immutable_body)?, body);
                    }
                    other => return Err(format!("cache replay retention: {other:?}").into()),
                },
                outcome => {
                    return Err(
                        format!("replay must preserve resource outcome: {outcome:?}").into(),
                    )
                }
            }
            check!(eq; snapshot(&store)?, original);
            Ok(())
        })
}
