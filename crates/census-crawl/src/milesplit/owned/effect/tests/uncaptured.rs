use super::*;
#[test]
fn uncaptured_transport_failure_is_explicit_and_does_not_seal_the_meet() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let (_dir, store, fetcher, reference) = setup()?;
            let ctx = context(&store, &fetcher)?;
            let error = match read_owned_meet(&ctx, &reference).await {
                Err(error) => error,
                Ok(_) => return Err("transport propagates".into()),
            };
            check!(matches!(error, CrawlError::Fetch(_)));
            check!(eq;
                store
                    .journal_keys(OWNED_MEET_PHASE)
                    ?,
                std::collections::HashSet::from(["failed/725218".to_string()])
            );
            check!(eq;
                store
                    .journal_keys(OWNED_CAPTURE_PHASE)
                    ?,
                std::collections::HashSet::new()
            );
            let payload = store.journal_payloads(OWNED_MEET_PHASE)?;
            check!(eq; payload[0]["disposition"], "failed");
            check!(eq; payload[0]["capture_available"], false);
            check!(eq; payload[0]["error"], error.to_string());
            Ok(())
        })
}
#[test]
fn uncaptured_access_refusal_is_not_a_transport_failure_or_fabricated_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for status in [401, 403] {
                let (_dir, store, fetcher, reference) = setup()?;
                let ctx = context(&store, &fetcher)?;
                let url = owned_meet_url(&reference)?;
                let error = CrawlError::Fetch(crate::net::FetchError::Http {
                    status,
                    url: url.clone(),
                });
                record_failure(&ctx, &reference, &error)?;
                check!(eq;
                    store.journal_keys(OWNED_MEET_PHASE)?,
                    std::collections::HashSet::from(["refused/725218".to_string()])
                );
                check!(eq;
                    store
                        .journal_keys(OWNED_CAPTURE_PHASE)
                        ?,
                    std::collections::HashSet::new()
                );
                let payload = store.journal_payloads(OWNED_MEET_PHASE)?;
                check!(eq; payload[0]["disposition"], "refused");
                check!(eq; payload[0]["capture_available"], false);
                check!(eq; payload[0]["source_url"], url);
                check!(eq; payload[0]["error"], error.to_string());
            }
            Ok(())
        })
}
