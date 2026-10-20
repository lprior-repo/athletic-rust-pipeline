use super::*;
use std::collections::HashSet;

#[test]
fn explicit_refresh_reopens_journaled_profiles_and_keeps_failed_refresh_owed() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let ctx = context(&store, &fetcher, &capture)?;
            let urls = source_urls(28872883);
            for url in &urls {
                store.journal_done(
                    "athleticnet",
                    url,
                    &serde_json::json!({
                        "url": url, "parser": PROFILE_PARSE_VERSION, "parsed": true,
                    }),
                )?;
                store.journal_done(
                    PROFILE_ATTEMPT_PHASE,
                    url,
                    &serde_json::json!({
                        "url": url, "parser": PROFILE_PARSE_VERSION, "parsed": true,
                    }),
                )?;
            }
            check!(eq;
                crate::athleticnet::collect::journaled_urls(&ctx)
                    ?
                    .len(),
                2
            );
            let mut options = options(dir.path(), 28872883)?;
            options.refresh = true;
            let report = crate::athleticnet::collect::collect(&ctx, &options).await?;
            check!(eq; report.errors, 2);
            check!(eq; report.unfinished, urls);
            check!(eq; report.disposition, crate::CollectionDisposition::Partial);
            check!(crate::athleticnet::collect::journaled_urls(&ctx)?.is_empty());
            check!(eq;
                store
                    .journal_payloads("athleticnet")
                    ?
                    .len(),
                2
            );
            let attempt_keys = store.journal_keys(PROFILE_ATTEMPT_PHASE)?;
            for url in &urls {
                check!(attempt_keys.contains(url));
            }
            let reviews: Vec<ReviewCase> = store.snapshot().scan(Table::ReviewCases)?;
            for url in urls {
                check!(reviews.iter().any(|review| review.subject_id == url));
            }
            Ok(())
        })
}

#[test]
fn pre_admission_profile_receipts_do_not_hide_owed_owner_validation() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let store = Store::open(dir.path().join("store"))?;
            let fetcher = crate::net::Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let ctx = context(&store, &fetcher, &capture)?;
            for url in source_urls(28127170) {
                store.journal_done(
                    "athleticnet",
                    &url,
                    &serde_json::json!({
                        "url": url,
                        "parser": crate::athleticnet::PARSE_VERSION,
                        "parsed": true,
                    }),
                )?;
            }
            let meet = crate::athleticnet::meet_requests(634313)
                .into_iter()
                .next()
                .ok_or("meet URL")?;
            store.journal_done(
                crate::athleticnet::MEET_COMPLETION_PHASE,
                &meet,
                &serde_json::json!({
                    "url": meet,
                    "parser": crate::athleticnet::PARSE_VERSION,
                    "parsed": true,
                }),
            )?;
            check!(eq;
                crate::athleticnet::collect::journaled_urls(&ctx)?,
                std::collections::HashSet::from([meet]),
            );
            Ok(())
        })
}

#[test]
fn previous_profile_parser_receipts_remain_retained_without_current_completion_credit() -> TestResult
{
    let dir = tempfile::tempdir()?;
    let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
    let store = Store::open(dir.path().join("store"))?;
    let fetcher = crate::net::Fetcher::new(
        dir.path().join("http"),
        None,
        std::time::Duration::from_millis(1),
        HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true);
    let ctx = context(&store, &fetcher, &capture)?;
    let url = source_urls(28872883)
        .into_iter()
        .next()
        .ok_or("profile URL")?;
    let legacy = serde_json::json!({"url":url,"parser":4,"parsed":true,"rows":7});
    store.journal_done("athleticnet_profile_attempts_v4", &url, &legacy)?;
    store.journal_done("athleticnet", &url, &legacy)?;
    check!(eq; PROFILE_PARSE_VERSION, 5);
    check!(eq; PROFILE_ATTEMPT_PHASE, "athleticnet_profile_attempts_v5");
    check!(eq; crate::athleticnet::collect::journaled_urls(&ctx)?, HashSet::new());
    let mismatched = serde_json::json!({"url":url,"parser":4,"parsed":true});
    store.journal_done(PROFILE_ATTEMPT_PHASE, &url, &mismatched)?;
    check!(eq; crate::athleticnet::collect::journaled_urls(&ctx)?, HashSet::new());
    let incomplete = serde_json::json!({"url":url,"parser":5,"parsed":false});
    store.journal_done(PROFILE_ATTEMPT_PHASE, &url, &incomplete)?;
    check!(eq; crate::athleticnet::collect::journaled_urls(&ctx)?, HashSet::new());
    let current = serde_json::json!({"url":url,"parser":5,"parsed":true});
    store.journal_done(PROFILE_ATTEMPT_PHASE, &url, &current)?;
    check!(eq;
        crate::athleticnet::collect::journaled_urls(&ctx)?,
        HashSet::from([url.clone()])
    );
    check!(eq; store.journal_payload("athleticnet_profile_attempts_v4", &url)?, Some(legacy.clone()));
    check!(eq; store.journal_payload("athleticnet", &url)?, Some(legacy));
    Ok(())
}

#[test]
fn withheld_profile_commits_owner_grade_locators_without_successful_full_keys() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let store = Store::open(dir.path().join("store"))?;
            let cache = dir.path().join("http");
            let mut profile: serde_json::Value = serde_json::from_str(CAPTURE)?;
            profile["athlete"]["Gender"] = serde_json::json!("");
            let profile = serde_json::to_string(&profile)?;
            let urls = source_urls(28872883);
            for url in &urls {
                seed_cache(&cache, url, &profile)?;
            }
            let fetcher = crate::net::Fetcher::new(
                &cache,
                None,
                std::time::Duration::from_millis(1),
                HashMap::new(),
                Vec::new(),
            )?
            .with_offline(true);
            let ctx = context(&store, &fetcher, &capture)?;
            let report =
                crate::athleticnet::collect::collect(&ctx, &options(dir.path(), 28872883)?).await?;
            check!(eq; report.errors, 2);
            check!(store.journal_keys("athleticnet")?.is_empty());
            check!(crate::athleticnet::collect::journaled_urls(&ctx)?.is_empty());
            let mut retained = std::collections::HashSet::new();
            let mut provider_facts = Vec::new();
            store
                .snapshot()
                .for_each_observation(Table::SourceObservations, |observation| {
                    if let census_domain::model::SourceObservation::Athlete(row) = observation {
                        if let Some(grade) = row.observed_grade {
                            provider_facts.push((row.source_athlete_id, row.observed_school));
                            retained.insert((
                                grade.grade.get(),
                                grade.school_year.get(),
                                row.source_row_key,
                            ));
                        }
                    }
                    Ok(())
                })?;
            for (source_athlete_id, observed_school) in provider_facts {
                check!(eq; source_athlete_id, "28872883");
                check!(eq; observed_school.as_deref(), Some("Middleton"));
            }
            let expected: std::collections::HashSet<_> = urls
                .iter()
                .flat_map(|url| {
                    [
                        (10, 2024, format!("{url}#grades/3204_2025")),
                        (11, 2025, format!("{url}#grades/3204_2026")),
                    ]
                })
                .collect();
            check!(eq; retained, expected);
            check!(store.snapshot().athletes()?.is_empty());
            let payloads = store.journal_payloads(PROFILE_ATTEMPT_PHASE)?;
            for url in urls {
                check!(payloads.iter().any(|payload| payload
                    .get("url")
                    .and_then(serde_json::Value::as_str)
                    == Some(url.as_str())
                    && payload.get("parsed") == Some(&serde_json::json!(false))));
            }
            Ok(())
        })
}
