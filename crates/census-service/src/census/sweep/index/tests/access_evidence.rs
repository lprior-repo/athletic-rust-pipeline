use super::*;
use crate::census::SourceAccessFailure;
use census_crawl::net::{FetchError, Fetcher};
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{StoreError, Table};

const URL: &str = "https://wi.milesplit.com/teams";

fn condition(at: &str) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "original-source",
        "wi.milesplit.com",
        AccessBlockKind::RateLimited,
        429,
        at,
        "original refusal detail",
    )
    .with_retry_after(Some(120))
    .with_cooldown_until(Some("2026-10-14T12:00:00Z".to_string()))
}

fn http(status: u16) -> CrawlError {
    FetchError::Http {
        status,
        url: URL.to_string(),
    }
    .into()
}

fn offline(root: &std::path::Path) -> Result<Fetcher, Box<dyn std::error::Error>> {
    Ok(Fetcher::new(
        root.join("cache"),
        None,
        std::time::Duration::ZERO,
        std::collections::HashMap::new(),
        Vec::new(),
    )?
    .with_offline(true))
}

#[test]
fn index_refusal_commits_exact_access_and_preserves_prior_observation_errors() -> TestResult {
    let root = tempfile::tempdir()?;
    let expected = condition("2026-10-14T11:00:00Z");
    {
        let store = Store::open(root.path())?;
        configure(
            &store,
            UsJurisdiction::Wisconsin,
            TeamIndexRead {
                teams: vec![team("52649")],
                disposition: Disposition::Partial,
                unfinished: vec!["legacy-inventory-unmeasured".to_string()],
                errors: 3,
            },
        )?;
        let error = super::super::failure::record(
            &store,
            UsJurisdiction::Wisconsin,
            http(429),
            std::slice::from_ref(&expected),
        );
        check!(
            matches!(error, SourceAccessFailure::Primary(CrawlError::Fetch(FetchError::Http { status: 429, ref url })) if url == URL)
        );
    }
    let store = Store::open(root.path())?;
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![expected]);
    let report = team_index_report(&store, UsJurisdiction::Wisconsin)?;
    check!(eq; (report.disposition, report.rows, report.errors), (Disposition::Partial, 1, 4));
    check!(eq; report.unfinished, vec!["legacy-inventory-unmeasured".to_string()]);
    Ok(())
}

#[test]
fn index_refusal_with_empty_access_keeps_the_legacy_failure_receipt() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let error = super::super::failure::record(&store, UsJurisdiction::Wisconsin, http(403), &[]);
    check!(
        matches!(error, SourceAccessFailure::Primary(CrawlError::Fetch(FetchError::Http { status: 403, ref url })) if url == URL)
    );
    let report = team_index_report(&store, UsJurisdiction::Wisconsin)?;
    check!(eq; (report.disposition, report.rows, report.errors), (Disposition::Partial, 0, 1));
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    Ok(())
}

#[test]
fn oversized_access_refuses_the_index_receipt_without_changing_primary_status() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let mut oversized = condition("2026-10-14T11:00:00Z");
    oversized.detail = "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES);
    let error =
        super::super::failure::record(&store, UsJurisdiction::Wisconsin, http(429), &[oversized]);
    check!(error.retryable());
    check!(matches!(&error, SourceAccessFailure::Retention {
        primary,
        secondary,
    } if matches!(primary.as_ref(), CrawlError::Fetch(FetchError::Http { status: 429, url }) if url == URL)
        && matches!(secondary.as_ref(),
        CrawlError::Store(StoreError::RowTooLarge { bytes, max })
            if bytes > max && *max == census_store::MAX_OBSERVATION_RECORD_BYTES)));
    check!(eq; store.journal_payload(&teams_phase(UsJurisdiction::Wisconsin), "WI")?, None);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    Ok(())
}

#[test]
fn index_counter_exhaustion_retains_original_failure_and_leaves_both_tables_unchanged() -> TestResult
{
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    save(
        &store,
        UsJurisdiction::Wisconsin,
        &Receipt {
            disposition: Disposition::Partial,
            errors: usize::MAX,
            ..Receipt::default()
        },
    )?;
    let before = store.journal_payload(&teams_phase(UsJurisdiction::Wisconsin), "WI")?;
    let error = super::super::failure::record(
        &store,
        UsJurisdiction::Wisconsin,
        http(429),
        &[condition("2026-10-14T11:00:00Z")],
    );
    check!(matches!(&error, SourceAccessFailure::Retention {
        primary,
        secondary,
    } if matches!(primary.as_ref(), CrawlError::Fetch(FetchError::Http { status: 429, url }) if url == URL)
        && matches!(secondary.as_ref(),
        CrawlError::Arithmetic { detail } if detail == "team index error count overflow")));
    check!(eq; store.journal_payload(&teams_phase(UsJurisdiction::Wisconsin), "WI")?, before);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    Ok(())
}

#[test]
fn older_index_refusal_cannot_regress_newer_access_evidence() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let newer = condition("2026-10-14T11:00:00Z");
    let first = super::super::failure::record(
        &store,
        UsJurisdiction::Wisconsin,
        http(429),
        std::slice::from_ref(&newer),
    );
    check!(
        matches!(first, SourceAccessFailure::Primary(CrawlError::Fetch(FetchError::Http { status: 429, ref url })) if url == URL)
    );
    let older = condition("2026-10-13T11:00:00Z").with_retry_after(Some(10));
    let second =
        super::super::failure::record(&store, UsJurisdiction::Wisconsin, http(403), &[older]);
    check!(matches!(
        second,
        SourceAccessFailure::Primary(CrawlError::Fetch(FetchError::Http { status: 403, .. }))
    ));
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![newer]);
    check!(eq; team_index_report(&store, UsJurisdiction::Wisconsin)?.errors, 2);
    Ok(())
}

#[tokio::test]
async fn public_team_index_failure_retains_snapshot_before_any_sweep_flush() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = offline(root.path())?;
    let expected = fetcher
        .record_access_condition(
            "wi.milesplit.com",
            AccessBlockKind::RateLimited,
            429,
            Some(120),
            "seeded original refusal",
        )
        .await;
    let error =
        crate::census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            .err()
            .ok_or("offline index unexpectedly succeeded")?;
    check!(
        matches!(error, SourceAccessFailure::Primary(CrawlError::Fetch(FetchError::Offline { ref url })) if url == URL)
    );
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![expected]);
    check!(eq; team_index_report(&store, UsJurisdiction::Wisconsin)?.errors, 1);
    check!(eq; fetcher.stats().await.requests, 0);
    Ok(())
}

#[tokio::test]
async fn snapshot_failure_keeps_primary_and_does_not_commit_the_index_receipt() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = offline(root.path())?;
    fetcher
        .record_access_condition(
            "wi.milesplit.com",
            AccessBlockKind::RateLimited,
            429,
            Some(120),
            "x".repeat(census_store::MAX_OBSERVATION_RECORD_BYTES),
        )
        .await;
    let error =
        crate::census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            .err()
            .ok_or("offline index unexpectedly succeeded")?;
    check!(matches!(&error, SourceAccessFailure::Retention {
        primary,
        secondary,
    } if matches!(primary.as_ref(), CrawlError::Fetch(FetchError::Offline { url }) if url == URL)
        && matches!(secondary.as_ref(),
        CrawlError::Store(StoreError::RowTooLarge { bytes, max })
            if bytes > max && *max == census_store::MAX_OBSERVATION_RECORD_BYTES)));
    check!(eq; store.journal_payload(&teams_phase(UsJurisdiction::Wisconsin), "WI")?, None);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 0);
    check!(eq; fetcher.stats().await.requests, 0);
    Ok(())
}

#[test]
fn retained_access_decode_failure_refuses_the_index_receipt_atomically() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let row = condition("2026-10-14T11:00:00Z");
    store.replace_many(
        Table::SourceAccess,
        &[serde_json::json!({"id": row.id, "detail": "missing required fields"})],
    )?;
    let generation = store.evidence_generation();
    let sequence = store.snapshot().sequence();
    let error = super::super::failure::record(&store, UsJurisdiction::Wisconsin, http(429), &[row]);
    check!(matches!(&error, SourceAccessFailure::Retention {
        primary,
        secondary,
    } if matches!(primary.as_ref(), CrawlError::Fetch(FetchError::Http { status: 429, url }) if url == URL)
        && matches!(secondary.as_ref(),
        CrawlError::Store(StoreError::Json { detail, source })
            if detail == "decoding retained source access condition" && source.is_data())));
    check!(eq; store.journal_payload(&teams_phase(UsJurisdiction::Wisconsin), "WI")?, None);
    check!(eq; store.evidence_generation(), generation);
    check!(eq; store.snapshot().sequence(), sequence);
    check!(eq; store.walk_table(Table::SourceAccess)?.rows, 1);
    Ok(())
}

#[tokio::test]
async fn configured_index_failure_retains_access_and_legacy_inventory_obligation() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path().join("store"))?;
    let fetcher = offline(root.path())?;
    crate::capture_cache::seed(
        fetcher.cache_dir(),
        URL,
        br#"<meta name="application-name" content="MileSplit">"#,
        "2026-10-14T11:00:00Z",
        &[],
    )?;
    save(
        &store,
        UsJurisdiction::Wisconsin,
        &Receipt {
            roster_teams: vec!["52649".to_string()],
            unfinished: vec!["legacy-inventory-unmeasured".to_string()],
            disposition: Disposition::Partial,
            ..Receipt::default()
        },
    )?;
    store.journal_done(
        "milesplit_roster_inputs_v1",
        "WI:52649",
        &serde_json::json!({"id": "52649"}),
    )?;
    let expected = fetcher
        .record_access_condition(
            "wi.milesplit.com",
            AccessBlockKind::RateLimited,
            429,
            Some(120),
            "seeded original refusal",
        )
        .await;
    let error =
        crate::census::collect_state_teams(&fetcher, &store, UsJurisdiction::Wisconsin, false)
            .await
            .err()
            .ok_or("corrupt roster input unexpectedly succeeded")?;
    check!(
        matches!(error, SourceAccessFailure::Primary(CrawlError::Canonical { ref table, .. }) if table == "milesplit_roster_inputs_v1")
    );
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, vec![expected]);
    let receipt = load(&store, UsJurisdiction::Wisconsin)?.ok_or("missing failed index")?;
    check!(eq; (receipt.disposition, receipt.errors), (Disposition::Partial, 2));
    check!(receipt
        .unfinished
        .contains(&"legacy-inventory-unmeasured".to_string()));
    check!(eq; receipt.roster_teams, vec!["52649".to_string()]);
    check!(eq; fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
