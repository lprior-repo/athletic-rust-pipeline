use super::*;
use crate::net::Fetcher;
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use std::{collections::HashMap, sync::Arc, time::Duration};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 16,
        bytes: 65536,
        record_bytes: 8192,
    }
}

fn condition(host: &str, kind: AccessBlockKind) -> SourceAccessCondition {
    SourceAccessCondition::new(
        "retained-source",
        host,
        kind,
        429,
        "2026-10-18T10:00:00Z",
        "original evidence",
    )
}

#[tokio::test]
async fn cold_pacing_preserves_fields_expiry_human_hold_and_other_hosts() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let limited = condition("limited.example.org", AccessBlockKind::RateLimited)
        .with_retry_after(Some(300))
        .with_cooldown_until(Some("2026-10-18T10:05:00Z".into()));
    let human = condition("human.example.org", AccessBlockKind::HumanRequired);
    let expired = condition("expired.example.org", AccessBlockKind::Unavailable)
        .with_cooldown_until(Some("2020-01-01T00:00:00Z".into()));
    let rows = vec![human.clone(), limited.clone(), expired];
    store.replace_many(Table::SourceAccess, &rows)?;
    drop(store);
    let store = Store::open(root.path())?;
    let restored = PacingState::restore(&store, budget())?;
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_shared_pacing(Arc::new(restored));
    check!(eq; fetcher.access_conditions().await?, rows);
    check!(
        fetcher
            .host_blocked(&limited.host, "2026-10-18T10:04:59Z")
            .await
    );
    check!(
        !fetcher
            .host_blocked(&limited.host, "2026-10-18T10:05:00Z")
            .await
    );
    check!(
        fetcher
            .host_blocked(&human.host, "2099-01-01T00:00:00Z")
            .await
    );
    check!(
        !fetcher
            .host_blocked("www.human.example.org", "2099-01-01T00:00:00Z")
            .await
    );
    check!(
        !fetcher
            .host_blocked("other.example.org", "2026-10-18T10:00:00Z")
            .await
    );
    check!(eq; store.scan::<SourceAccessCondition>(Table::SourceAccess)?, rows);
    Ok(())
}

#[test]
fn restoration_refuses_row_byte_record_and_malformed_boundaries() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let row = condition("example.org", AccessBlockKind::HumanRequired);
    store.replace(Table::SourceAccess, &row)?;
    for limits in [
        ReadBudget {
            rows: 0,
            ..budget()
        },
        ReadBudget {
            bytes: 0,
            ..budget()
        },
        ReadBudget {
            record_bytes: 1,
            ..budget()
        },
    ] {
        check!(PacingState::restore(&store, limits).is_err());
    }
    let mut malformed = row.clone();
    malformed.cooldown_until = Some("not-a-deadline".into());
    let mut bad_id = row.clone();
    bad_id.id = "wrong-identity".into();
    let mut bad_host = row.clone();
    bad_host.host = " example.org ".into();
    let mut url_host = row.clone();
    url_host.host = "https://example.org/path".into();
    url_host.id = format!("{}:{}", url_host.kind.slug(), url_host.host);
    let mut bad_time = row.clone();
    bad_time.observed_at = "2026-10-18".into();
    for invalid in [malformed, bad_id, bad_host, bad_time, url_host] {
        let root = tempfile::tempdir()?;
        let store = Store::open(root.path())?;
        store.replace_many(
            Table::SourceAccess,
            &[
                condition("aaa.example.org", AccessBlockKind::HumanRequired),
                invalid,
            ],
        )?;
        check!(PacingState::restore(&store, budget()).is_err());
    }
    Ok(())
}

#[test]
fn restoration_accepts_producer_maximum_deadline_without_rewriting() -> TestResult {
    let root = tempfile::tempdir()?;
    let store = Store::open(root.path())?;
    let row = condition("example.org", AccessBlockKind::RateLimited)
        .with_cooldown_until(Some(crate::net::cooldown_until_iso8601(u64::MAX)));
    store.replace(Table::SourceAccess, &row)?;
    let mut restored = PacingState::restore(&store, budget())?;
    check!(eq; restored.blocks.get_mut().get(&row.id), Some(&row));
    check!(row.is_blocking("2026-10-18T13:00:00Z"));
    Ok(())
}
