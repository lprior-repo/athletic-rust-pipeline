use super::*;

async fn http_refusal_preserves_owed_state(status: u16) -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let host = format!("http://{}", listener.local_addr()?);
    let url = local_url(&host, 450, 1);
    let bad = serde_json::to_vec(&json!({"data": {"total": 2, "rows": [coach("Ada", "Lane")]}}))?;
    let replies = vec![
        robots(),
        reply(&host, &url, 200, &bad)?,
        reply(&host, &url, status, b"source refused")?,
    ];
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = local_fetcher(&cache)?;
    let ctx = context(&fetcher, &store, None)?;
    let options = options();
    let row = school();
    let client = async {
        run(&ctx, &options)?
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, (BASE, &host))
            .await?;
        let original = pending(&store, &row)?["recovery"].clone();
        let mut refused = run(&ctx, &options)?;
        refused
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, (BASE, &host))
            .await?;
        check!(eq; (refused.tally.errors, store.journal_keys(JOURNAL)?.len()), (1, 0));
        check!(eq;
            pending(&store, &row)?["kind"],
            json!(if status == 429 { "retryable" } else { "source_refused" })
        );
        check!(eq; pending(&store, &row)?["recovery"], original);
        assert_retained_facts(
            &store,
            &BTreeSet::from(["Ada Lane".to_string(), "Casey Reed".to_string()]),
        )?;
        let conditions = fetcher.access_conditions().await;
        check!(eq; conditions.len(), 1);
        check!(eq; conditions.first().ok_or("source condition")?.status, status);
        let before = fetcher.stats().await.physical_requests();
        let mut cooldown = run(&ctx, &options)?;
        cooldown
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, (BASE, &host))
            .await?;
        check!(eq; (cooldown.tally.errors, store.journal_keys(JOURNAL)?.len()), (1, 0));
        check!(eq; fetcher.stats().await.physical_requests(), before);
        check!(
            eq;
            pending(&store, &row)?["kind"],
            json!("retryable"),
            "a recorded cooldown keeps the school retryable instead of refusing it"
        );
        check!(eq; pending(&store, &row)?["recovery"], original);
        assert_retained_facts(
            &store,
            &BTreeSet::from(["Ada Lane".to_string(), "Casey Reed".to_string()]),
        )?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let (served, acquired) = tokio::join!(serve_responses(listener, replies), client);
        let accepted = served?;
        acquired?;
        assert_request_conservation(&fetcher, &accepted).await
    })
    .await?
}

#[test]
fn known_incomplete_cache_reacquisition_does_not_bypass_source_refusal() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_refusal_preserves_owed_state(403).await })
}

#[test]
fn known_incomplete_cache_reacquisition_does_not_bypass_source_cooldown_budget() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_refusal_preserves_owed_state(429).await })
}
