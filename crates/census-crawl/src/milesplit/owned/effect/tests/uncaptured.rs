use super::*;
#[test]
fn uncaptured_access_refusal_is_not_a_transport_failure_or_fabricated_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for status in [401, 403] {
                let (_dir, store, fetcher, reference) = setup()?;
                let durable = crate::net::durable_capture_context(&_dir.path().join("capture"))?;
                let ctx = context(&store, &fetcher, &durable)?;
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
