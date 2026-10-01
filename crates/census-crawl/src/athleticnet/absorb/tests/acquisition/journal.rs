use super::*;

#[tokio::test]
async fn explicit_refresh_reopens_journaled_profiles_and_keeps_failed_refresh_owed() {
    let dir = tempfile::tempdir().expect("isolated run");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let ctx = context(&store, &fetcher);
    let urls = source_urls(28872883);
    for url in &urls {
        store
            .journal_done(
                "athleticnet",
                url,
                &serde_json::json!({
                    "url": url, "parser": PROFILE_PARSE_VERSION, "parsed": true,
                }),
            )
            .expect("previous completion");
        store
            .journal_done(
                PROFILE_ATTEMPT_PHASE,
                &format!("attempt:{url}"),
                &serde_json::json!({
                    "url": url, "parser": PROFILE_PARSE_VERSION, "parsed": true,
                }),
            )
            .expect("previous complete attempt");
    }
    assert_eq!(
        crate::athleticnet::collect::journaled_urls(&ctx)
            .expect("previous completion")
            .len(),
        2
    );
    let mut options = options(dir.path(), 28872883);
    options.refresh = true;
    let report = crate::athleticnet::collect::collect(&ctx, &options)
        .await
        .expect("refresh report");
    assert_eq!(report.errors, 2);
    assert!(crate::athleticnet::collect::journaled_urls(&ctx)
        .expect("refresh still owed")
        .is_empty());
    assert_eq!(
        store
            .journal_payloads("athleticnet")
            .expect("retained successes")
            .len(),
        2
    );
    let attempt_keys = store
        .journal_keys(PROFILE_ATTEMPT_PHASE)
        .expect("distinct attempt keys");
    for url in &urls {
        assert!(attempt_keys.contains(&format!("attempt:{url}")));
    }
    let reviews: Vec<ReviewCase> = store
        .snapshot()
        .scan(Table::ReviewCases)
        .expect("refresh failures");
    for url in urls {
        assert!(reviews.iter().any(|review| review.subject_id == url));
    }
}

#[tokio::test]
async fn pre_admission_profile_receipts_do_not_hide_owed_owner_validation() {
    let dir = tempfile::tempdir().expect("isolated run");
    let store = Store::open(dir.path().join("store")).expect("store");
    let fetcher = crate::net::Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let ctx = context(&store, &fetcher);
    for url in source_urls(28127170) {
        store
            .journal_done(
                "athleticnet",
                &url,
                &serde_json::json!({
                    "url": url,
                    "parser": crate::athleticnet::PARSE_VERSION,
                    "parsed": true,
                }),
            )
            .expect("historical profile receipt");
    }
    let meet = crate::athleticnet::meet_requests(634313)
        .into_iter()
        .next()
        .expect("meet URL");
    store
        .journal_done(
            "athleticnet",
            &meet,
            &serde_json::json!({
                "url": meet,
                "parser": crate::athleticnet::PARSE_VERSION,
                "parsed": true,
            }),
        )
        .expect("unchanged meet receipt");
    assert_eq!(
        crate::athleticnet::collect::journaled_urls(&ctx).expect("scoped completion"),
        std::collections::HashSet::from([meet]),
    );
}

#[tokio::test]
async fn withheld_profile_commits_owner_grade_locators_without_successful_full_keys() {
    let dir = tempfile::tempdir().expect("isolated run");
    let store = Store::open(dir.path().join("store")).expect("store");
    let cache = dir.path().join("http");
    let mut profile: serde_json::Value = serde_json::from_str(CAPTURE).expect("public capture");
    profile["athlete"]["Gender"] = serde_json::json!("");
    let profile = serde_json::to_string(&profile).expect("derived unknown gender");
    let urls = source_urls(28872883);
    for url in &urls {
        seed_cache(&cache, url, &profile);
    }
    let fetcher = crate::net::Fetcher::new(
        &cache,
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )
    .expect("fetcher")
    .with_offline(true);
    let ctx = context(&store, &fetcher);
    let report = crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28872883))
        .await
        .expect("withheld report");
    assert_eq!(report.errors, 2);
    assert!(store
        .journal_keys("athleticnet")
        .expect("success keys")
        .is_empty());
    assert!(crate::athleticnet::collect::journaled_urls(&ctx)
        .expect("owed profiles")
        .is_empty());
    let mut retained = std::collections::HashSet::new();
    store
        .snapshot()
        .for_each_observation(Table::SourceObservations, |observation| {
            if let census_domain::model::SourceObservation::Athlete(row) = observation {
                if let Some(grade) = row.observed_grade {
                    assert_eq!(row.source_athlete_id, "28872883");
                    assert_eq!(row.observed_school.as_deref(), Some("Middleton"));
                    retained.insert((
                        grade.grade.get(),
                        grade.school_year.get(),
                        row.source_row_key,
                    ));
                }
            }
            Ok(())
        })
        .expect("durable grade facts");
    let expected: std::collections::HashSet<_> = urls
        .iter()
        .flat_map(|url| {
            [
                (10, 2024, format!("{url}#grades/3204_2025")),
                (11, 2025, format!("{url}#grades/3204_2026")),
            ]
        })
        .collect();
    assert_eq!(retained, expected);
    assert!(store
        .snapshot()
        .athletes()
        .expect("canonical withheld")
        .is_empty());
    let payloads = store
        .journal_payloads(PROFILE_ATTEMPT_PHASE)
        .expect("partial keys");
    for url in urls {
        assert!(payloads.iter().any(|payload| payload
            .get("url")
            .and_then(serde_json::Value::as_str)
            == Some(url.as_str())
            && payload.get("parsed") == Some(&serde_json::json!(false))));
    }
}
