use super::*;

async fn http_same_school_recovery(first: FirstPage, recording_route: bool) -> TestResult {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let host = format!("http://{}", listener.local_addr()?);
    let url = local_url(&host, 450, 1);
    let bad = match first {
        FirstPage::Malformed | FirstPage::LaterMalformed => b"not JSON".to_vec(),
        FirstPage::EmptyShort => serde_json::to_vec(&json!({"data": {"total": 1, "rows": []}}))?,
        FirstPage::SamePageMalformed | FirstPage::MalformedField => {
            let invalid = if matches!(first, FirstPage::MalformedField) {
                json!({"firstName": 42})
            } else {
                json!(42)
            };
            serde_json::to_vec(
                &json!({"data": {"total": 3, "rows": [coach("Ada", "Lane"), invalid, coach("Beau", "Pine")]}}),
            )?
        }
        _ => serde_json::to_vec(&json!({"data": {"total": 2, "rows": [coach("Ada", "Lane")]}}))?,
    };
    let mut replies = vec![robots()];
    let later = matches!(first, FirstPage::LaterMalformed);
    let bad_url = if later {
        let full: Vec<_> = std::iter::once(coach("Ada", "Lane"))
            .chain(std::iter::repeat_n(json!({}), 199))
            .collect();
        let body = serde_json::to_vec(&json!({"data": {"total": 201, "rows": full}}))?;
        replies.push(reply(&host, &url, 200, &body)?);
        local_url(&host, 450, 2)
    } else {
        url.clone()
    };
    replies.push(reply(&host, &bad_url, 200, &bad)?);
    let good = if later {
        json!({"data": {"total": 201, "rows": [coach("Beau", "Pine")]}})
    } else {
        json!({"data": {"total": 2, "rows": [coach("Ada", "Lane"), coach("Beau", "Pine")]}})
    };
    replies.push(reply(&host, &bad_url, 200, &serde_json::to_vec(&good)?)?);
    let dir = tempfile::tempdir()?;
    let cache = dir.path().join("http");
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = local_fetcher(&cache)?;
    let recording = Recording::new();
    let ctx = context(
        &fetcher,
        &store,
        if recording_route {
            Some(&recording)
        } else {
            None
        },
    )?;
    let options = options();
    let row = school();
    let client = async {
        let mut failed = run(&ctx, &options)?;
        failed
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await?;
        check!(eq; (failed.tally.errors, failed.done.len()), (1, 0));
        if recording_route {
            apply_recorded(&store, &recording.drain())?;
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
        assert_retained_facts(&store, &retained)?;
        let marker = pending(&store, &row)?;
        check!(eq; marker["owner_key"], json!("NH:2132:450"));
        check!(eq; marker["recovery"]["responses"][0]["url"], json!(bad_url));
        check!(eq;
            marker["recovery"]["responses"][0]["content_digest"],
            json!(crate::net::cache::content_digest(&bad))
        );
        check!(eq; marker["recovery"]["responses"][0]["status"], json!(200));
        check!(eq; marker["recovery"]["responses"][0]["method"], json!("GET"));
        let original_meta = capture_meta(&cache, &bad_url)?;
        check!(eq; marker["recovery"]["responses"][0]["fetched_at"], original_meta["fetched_at"]);
        let mut recovered = run(&ctx, &options)?;
        recovered
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await?;
        check!(eq; (recovered.tally.schools, recovered.tally.coaches, recovered.tally.errors), (1, 3, 0));
        assert_archived_capture(&cache, &bad, &original_meta)?;
        if recording_route {
            apply_recorded(&store, &recording.drain())?;
        }
        let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        check!(eq; coaches.len(), 3);
        check!(eq;
            coaches.iter().map(|coach| coach.name.as_str()).collect::<BTreeSet<_>>(),
            BTreeSet::from(["Ada Lane", "Beau Pine", "Casey Reed"])
        );
        check!(eq;
            store.journal_keys(JOURNAL)?,
            std::collections::HashSet::from(["NH:2132:450".to_string()])
        );
        let before = store.walk_table(Table::Coaches)?;
        let requests = fetcher.stats().await.requests;
        let mut replay = run(&ctx, &options)?;
        replay
            .process_school_at(UsJurisdiction::NewHampshire, "2132", &row, BASE, &host)
            .await?;
        check!(eq; (replay.tally.schools, replay.tally.skipped), (0, 1));
        check!(eq; store.walk_table(Table::Coaches)?, before);
        check!(eq; fetcher.stats().await.requests, requests);
        assert_archived_capture(&cache, &bad, &original_meta)?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    tokio::time::timeout(Duration::from_secs(15), async {
        let (served, acquired) = tokio::join!(serve_responses(listener, replies), client);
        served?;
        acquired
    })
    .await?
}

#[test]
fn malformed_200_cache_reacquires_recovered_source_on_ordinary_same_school_call() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::Malformed, false).await })
}

#[test]
fn short_200_cache_reacquires_recovered_source_on_ordinary_same_school_call() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::Short, false).await })
}

#[test]
fn empty_short_200_cache_reacquires_without_claiming_no_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::EmptyShort, false).await })
}

#[test]
fn later_bad_page_reacquires_only_that_page_and_preserves_earlier_facts() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::LaterMalformed, false).await })
}

#[test]
fn recorded_incomplete_acquisition_reacquires_after_atomic_store_application() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::Short, true).await })
}

#[test]
fn mixed_valid_and_malformed_rows_reacquire_without_losing_valid_coaches() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::SamePageMalformed, false).await })
}

#[test]
fn malformed_coach_field_reacquires_corrected_body_without_cache_rewrite() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async { http_same_school_recovery(FirstPage::MalformedField, false).await })
}

#[test]
fn zero_coach_total_completes_only_with_an_admitted_response_schema() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
    for body in [
        "{}",
        "{\"data\":{\"rows\":[]}}",
        "{\"data\":{\"total\":0}}",
        "{\"data\":{\"total\":0,\"rows\":[]}}",
    ] {
        let dir = tempfile::tempdir()?;
        let cache = dir.path().join("http");
        seed(&cache, 1, body)?;
        let store = Store::open(dir.path().join("store"))?;
        let fetcher = fetcher(&cache)?;
        let ctx = context(&fetcher, &store, None)?;
        let options = options();
        let row = school();
        let mut acquired = run(&ctx, &options)?;
        acquired.process_school(UsJurisdiction::NewHampshire, "2132", &row, BASE).await?;
        let complete = body == "{\"data\":{\"total\":0,\"rows\":[]}}";
        check!(eq; acquired.tally.errors, usize::from(!complete), "{body}");
        check!(eq; store.journal_keys(JOURNAL)?.contains("NH:2132:450"), complete, "{body}");
        let coaches: Vec<CanonicalCoach> = store.scan(Table::Coaches)?;
        check!(eq; coaches.into_iter().map(|coach| coach.name).collect::<Vec<_>>(), vec!["Casey Reed"]);
        if !complete {
            check!(eq; pending(&store, &row)?["recovery"]["responses"][0]["url"], json!(coach_url(1)));
        }
    }
    Ok(())
    })
}
