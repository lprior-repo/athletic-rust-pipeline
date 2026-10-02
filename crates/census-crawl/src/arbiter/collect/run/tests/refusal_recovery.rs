use super::*;

async fn http_refusal_preserves_owed_state(status: u16) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("local fixture");
    let host = format!("http://{}", listener.local_addr().expect("fixture address"));
    let url = local_url(&host, 450, 1);
    let bad = serde_json::to_vec(&json!({"data": {"total": 2, "rows": [coach("Ada", "Lane")]}}))
        .expect("partial body");
    let replies = vec![
        robots(),
        reply(&host, &url, 200, &bad),
        reply(&host, &url, status, b"source refused"),
    ];
    let dir = tempfile::tempdir().expect("isolated refusal");
    let cache = dir.path().join("http");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = local_fetcher(&cache);
    let ctx = context(&fetcher, &store, None);
    let options = options();
    let row = school();
    let client = async {
        run(&ctx, &options)
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await
            .expect("incomplete facts");
        let original = pending(&store, &row)["recovery"].clone();
        let mut refused = run(&ctx, &options);
        refused
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await
            .expect("retained refusal");
        assert_eq!((refused.tally.errors, refused.done.len()), (1, 0));
        assert_eq!(
            pending(&store, &row)["kind"],
            json!(if status == 429 {
                "retryable"
            } else {
                "source_refused"
            })
        );
        assert_eq!(pending(&store, &row)["recovery"], original);
        assert_retained_facts(
            &store,
            &BTreeSet::from(["Ada Lane".to_string(), "Casey Reed".to_string()]),
        );
        let conditions = fetcher.access_conditions().await;
        assert_eq!(conditions.len(), 1);
        assert_eq!(conditions.first().expect("source condition").status, status);
        let before = fetcher.stats().await.requests;
        let mut cooldown = run(&ctx, &options);
        cooldown
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await
            .expect("retained cooldown");
        assert_eq!((cooldown.tally.errors, cooldown.done.len()), (1, 0));
        assert_eq!(fetcher.stats().await.requests, before);
        assert_eq!(pending(&store, &row)["kind"], json!("source_refused"));
        assert_eq!(pending(&store, &row)["recovery"], original);
        assert_retained_facts(
            &store,
            &BTreeSet::from(["Ada Lane".to_string(), "Casey Reed".to_string()]),
        );
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        tokio::join!(serve_responses(listener, replies), client);
    })
    .await
    .expect("bounded refusal scenario");
}

#[tokio::test]
async fn known_incomplete_cache_reacquisition_does_not_bypass_source_refusal() {
    http_refusal_preserves_owed_state(403).await;
}

#[tokio::test]
async fn known_incomplete_cache_reacquisition_does_not_bypass_source_cooldown_budget() {
    http_refusal_preserves_owed_state(429).await;
}
