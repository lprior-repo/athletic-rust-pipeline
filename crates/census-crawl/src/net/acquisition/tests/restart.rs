use super::*;
use crate::net::{Fetcher, PacingState};
use census_domain::model::{AccessBlockKind, SourceAccessCondition};
use census_store::{ReadBudget, Table};
use std::{collections::HashMap, time::Duration};

fn budget() -> ReadBudget {
    ReadBudget {
        rows: 16,
        bytes: 65536,
        record_bytes: 8192,
    }
}

#[tokio::test]
async fn persisted_429_cold_pacing_blocks_new_work_before_http() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let headers = format!("Retry-After: 300\r\nETag: {}\r\n", "x".repeat(1025));
    let fetch = async {
        check!(fixture.acquire(2025, true).await?.is_err());
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let replies = [Reply {
        status: 429,
        body: b"slow down",
        headers: &headers,
        before: Mutation::None,
    }];
    tokio::try_join!(
        serve(listener, replies, Arc::clone(&fixture.requests)),
        fetch
    )?;
    let rows = fixture.fetcher.access_conditions().await?;
    fixture.store.replace_many(Table::SourceAccess, &rows)?;
    let mut fixture = fixture.reopen()?;
    let pacing = PacingState::restore(&fixture.store, budget())?;
    fixture.fetcher = Fetcher::new(
        fixture.store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec![HOST.into()],
    )?
    .with_shared_pacing(Arc::new(pacing));
    let outcome = fixture.acquire(2026, true).await?;
    check!(matches!(
        outcome,
        Err(crate::CrawlError::Fetch(
            crate::net::FetchError::Cooldown { .. }
        ))
    ));
    check!(eq; fixture.fetcher.access_conditions().await?, rows);
    check!(eq; fixture.count(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

fn restore(fixture: &mut Fixture, row: SourceAccessCondition) -> TestResult {
    fixture.store.replace(Table::SourceAccess, &row)?;
    fixture.fetcher.pacing = Arc::new(PacingState::restore(&fixture.store, budget())?);
    Ok(())
}

#[tokio::test]
async fn restored_human_required_refuses_new_work_before_http() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    restore(
        &mut fixture,
        SourceAccessCondition::new(
            "retained",
            HOST,
            AccessBlockKind::HumanRequired,
            403,
            CAPTURED,
            "operator required",
        ),
    )?;
    let outcome = fixture.acquire(2026, true).await?;
    check!(matches!(
        outcome,
        Err(crate::CrawlError::Fetch(
            crate::net::FetchError::BrowserLane {
                retryable: false,
                ..
            }
        ))
    ));
    check!(eq; fixture.count(), 0);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn restored_expired_rate_limit_admits_new_work() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    restore(
        &mut fixture,
        SourceAccessCondition::new(
            "retained",
            HOST,
            AccessBlockKind::RateLimited,
            429,
            CAPTURED,
            "expired",
        )
        .with_cooldown_until(Some("2020-01-01T00:00:00Z".into())),
    )?;
    let fetch = async {
        fixture.acquire(2026, true).await??;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn restored_human_hold_leaves_other_host_admitted() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    restore(
        &mut fixture,
        SourceAccessCondition::new(
            "retained",
            "other.example.org",
            AccessBlockKind::HumanRequired,
            403,
            CAPTURED,
            "operator required",
        ),
    )?;
    let fetch = async {
        fixture.acquire(2026, true).await??;
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 1);
    Ok(())
}
