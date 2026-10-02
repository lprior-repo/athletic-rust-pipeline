use super::*;

async fn http_same_school_recovery(first: FirstPage, recording_route: bool) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("local fixture");
    let host = format!("http://{}", listener.local_addr().expect("fixture address"));
    let url = local_url(&host, 450, 1);
    let bad = match first {
        FirstPage::Malformed | FirstPage::LaterMalformed => b"not JSON".to_vec(),
        FirstPage::EmptyShort => {
            serde_json::to_vec(&json!({"data": {"total": 1, "rows": []}})).expect("empty short")
        }
        FirstPage::SamePageMalformed | FirstPage::MalformedField => {
            let invalid = if matches!(first, FirstPage::MalformedField) {
                json!({"firstName": 42})
            } else {
                json!(42)
            };
            serde_json::to_vec(&json!({"data": {"total": 3, "rows": [coach("Ada", "Lane"), invalid, coach("Beau", "Pine")]}})).expect("partial rows")
        }
        _ => serde_json::to_vec(&json!({"data": {"total": 2, "rows": [coach("Ada", "Lane")]}}))
            .expect("short page"),
    };
    let mut replies = vec![robots()];
    let later = matches!(first, FirstPage::LaterMalformed);
    let bad_url = if later {
        let full: Vec<_> = std::iter::once(coach("Ada", "Lane"))
            .chain(std::iter::repeat_n(json!({}), 199))
            .collect();
        let body = serde_json::to_vec(&json!({"data": {"total": 201, "rows": full}}))
            .expect("earlier full page");
        replies.push(reply(&host, &url, 200, &body));
        local_url(&host, 450, 2)
    } else {
        url.clone()
    };
    replies.push(reply(&host, &bad_url, 200, &bad));
    let good = if later {
        json!({"data": {"total": 201, "rows": [coach("Beau", "Pine")]}})
    } else {
        json!({"data": {"total": 2, "rows": [coach("Ada", "Lane"), coach("Beau", "Pine")]}})
    };
    replies.push(reply(
        &host,
        &bad_url,
        200,
        &serde_json::to_vec(&good).expect("corrected page"),
    ));
    let dir = tempfile::tempdir().expect("isolated recovery");
    let cache = dir.path().join("http");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = local_fetcher(&cache);
    let recording = Recording::new();
    let ctx = context(
        &fetcher,
        &store,
        if recording_route {
            Some(&recording)
        } else {
            None
        },
    );
    let options = options();
    let row = school();
    let client = async {
        let mut failed = run(&ctx, &options);
        failed
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await
            .expect("incomplete HTTP acquisition");
        assert_eq!((failed.tally.errors, failed.done.len()), (1, 0));
        if recording_route {
            apply_recorded(&store, &recording.drain());
        }
        let retained = match first {
            FirstPage::Malformed | FirstPage::EmptyShort => {
                BTreeSet::from(["Casey Reed".to_string()])
            }
            FirstPage::SamePageMalformed | FirstPage::MalformedField => BTreeSet::from([
                "Ada Lane".to_string(),
                "Beau Pine".to_string(),
                "Casey Reed".to_string(),
            ]),
            _ => BTreeSet::from(["Ada Lane".to_string(), "Casey Reed".to_string()]),
        };
        assert_retained_facts(&store, &retained);
        let marker = pending(&store, &row);
        assert_eq!(marker["owner_key"], json!("NH:2132:450"));
        assert_eq!(marker["recovery"]["responses"][0]["url"], json!(bad_url));
        assert_eq!(
            marker["recovery"]["responses"][0]["content_digest"],
            json!(crate::net::cache::content_digest(&bad))
        );
        assert_eq!(marker["recovery"]["responses"][0]["status"], json!(200));
        assert_eq!(marker["recovery"]["responses"][0]["method"], json!("GET"));
        let original_meta = capture_meta(&cache, &bad_url);
        assert_eq!(
            marker["recovery"]["responses"][0]["fetched_at"],
            original_meta["fetched_at"]
        );
        let mut recovered = run(&ctx, &options);
        recovered
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await
            .expect("ordinary reacquisition");
        assert_eq!(
            (
                recovered.tally.schools,
                recovered.tally.coaches,
                recovered.tally.errors
            ),
            (1, 3, 0)
        );
        assert_archived_capture(&cache, &bad, &original_meta);
        if recording_route {
            apply_recorded(&store, &recording.drain());
        }
        let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches).expect("retained coaches");
        assert_eq!(coaches.len(), 3);
        assert_eq!(
            coaches
                .iter()
                .map(|coach| coach.name.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
        );
        assert_eq!(
            store.journal_keys(JOURNAL).expect("completed owner"),
            std::collections::HashSet::from(["NH:2132:450".to_string()])
        );
        let before = store.walk_table(Table::Coaches).expect("completed facts");
        let requests = fetcher.stats().await.requests;
        let mut replay = run(&ctx, &options);
        replay
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await
            .expect("completed replay");
        assert_eq!((replay.tally.schools, replay.tally.skipped), (0, 1));
        assert_eq!(
            store.walk_table(Table::Coaches).expect("replayed facts"),
            before
        );
        assert_eq!(fetcher.stats().await.requests, requests);
        assert_archived_capture(&cache, &bad, &original_meta);
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        tokio::join!(serve_responses(listener, replies), client);
    })
    .await
    .expect("bounded recovery scenario");
}

#[tokio::test]
async fn malformed_200_cache_reacquires_recovered_source_on_ordinary_same_school_call() {
    http_same_school_recovery(FirstPage::Malformed, false).await;
}

#[tokio::test]
async fn short_200_cache_reacquires_recovered_source_on_ordinary_same_school_call() {
    http_same_school_recovery(FirstPage::Short, false).await;
}

#[tokio::test]
async fn empty_short_200_cache_reacquires_without_claiming_no_coaches() {
    http_same_school_recovery(FirstPage::EmptyShort, false).await;
}

#[tokio::test]
async fn later_bad_page_reacquires_only_that_page_and_preserves_earlier_facts() {
    http_same_school_recovery(FirstPage::LaterMalformed, false).await;
}

#[tokio::test]
async fn recorded_incomplete_acquisition_reacquires_after_atomic_store_application() {
    http_same_school_recovery(FirstPage::Short, true).await;
}

#[tokio::test]
async fn mixed_valid_and_malformed_rows_reacquire_without_losing_valid_coaches() {
    http_same_school_recovery(FirstPage::SamePageMalformed, false).await;
}

#[tokio::test]
async fn malformed_coach_field_reacquires_corrected_body_without_cache_rewrite() {
    http_same_school_recovery(FirstPage::MalformedField, false).await;
}

#[tokio::test]
async fn zero_coach_total_completes_only_with_an_admitted_response_schema() {
    for body in [
        "{}",
        "{\"data\":{\"rows\":[]}}",
        "{\"data\":{\"total\":0}}",
        "{\"data\":{\"total\":0,\"rows\":[]}}",
    ] {
        let dir = tempfile::tempdir().expect("isolated schema outcome");
        let cache = dir.path().join("http");
        seed(&cache, 1, body);
        let store = Store::open(dir.path().join("store")).expect("store");
        let fetcher = fetcher(&cache);
        let ctx = context(&fetcher, &store, None);
        let options = options();
        let row = school();
        let mut acquired = run(&ctx, &options);
        acquired
            .process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE)
            .await
            .expect("schema outcome");
        let complete = body == "{\"data\":{\"total\":0,\"rows\":[]}}";
        assert_eq!(acquired.tally.errors, usize::from(!complete), "{body}");
        assert_eq!(
            store
                .journal_keys(JOURNAL)
                .expect("completion state")
                .contains("NH:2132:450"),
            complete,
            "{body}"
        );
        let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches).expect("primary contact");
        assert_eq!(
            coaches
                .into_iter()
                .map(|coach| coach.name)
                .collect::<Vec<_>>(),
            vec!["Casey Reed"]
        );
        if !complete {
            assert_eq!(
                pending(&store, &row)["recovery"]["responses"][0]["url"],
                json!(coach_url(1))
            );
        }
    }
}
