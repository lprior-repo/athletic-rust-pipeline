use super::*;
use census_crawl::net::{FetchError, FetchOptions};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub(super) fn scope() -> TestResult<HistoricalStageScope> {
    Ok(HistoricalStageScope {
        jurisdiction: census_domain::UsJurisdiction::Wisconsin,
        year: 2024,
        refresh: false,
        window: HistoryWindow::new(2023, 2026, "2026-10-07")?,
        observed_on: chrono::NaiveDate::parse_from_str("2026-10-14", "%Y-%m-%d")?,
    })
}

pub(super) fn fetcher(root: &Path) -> TestResult<Fetcher> {
    Ok(Fetcher::new(
        root.join("http"),
        None,
        Duration::ZERO,
        HashMap::new(),
        vec!["127.0.0.1".into()],
    )?
    .with_source("original-fixture-source"))
}

pub(super) fn condition(host: &str, at: &str, status: u16) -> SourceAccessCondition {
    let kind = if status == 429 {
        AccessBlockKind::RateLimited
    } else {
        AccessBlockKind::Forbidden
    };
    SourceAccessCondition::new(
        "original-fixture-source",
        host,
        kind,
        status,
        at,
        "original observed detail",
    )
    .with_retry_after(Some(120))
    .with_cooldown_until(Some("2026-10-14T12:00:00Z".into()))
}

pub(super) fn outcome(store: &Store, identity: &str) -> TestResult<MeetsStageOutcome> {
    let original = MeetCensus {
        sources: vec![MeetSourceRows {
            slug: "wiaa_results".into(),
            disposition: CollectionDisposition::Failed,
            errors: vec!["retained original acquisition cause".into()],
            unfinished: vec!["https://source.example/unfinished".into()],
            notes: vec!["original source note".into()],
            ..MeetSourceRows::default()
        }],
        ..MeetCensus::default()
    };
    let census = diagnostics::bounded_census(store, original, identity, 0).map_err(sdk_error)?;
    let recording = Recording::spooled(store.clone(), identity.into())?;
    Ok(MeetsStageOutcome {
        census,
        identity: identity.into(),
        spool: recording.finish_fragments()?,
        next: None,
    })
}

pub(super) async fn observed_refusal(
    root: &Path,
    status: u16,
) -> TestResult<(Fetcher, SourceAccessCondition)> {
    let fetcher = fetcher(root)?;
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let url = format!("http://{}/refused", listener.local_addr()?);
    let response = format!(
        "HTTP/1.1 {status} Fixture Refusal\r\nRetry-After: 120\r\nContent-Length: 0\r\nConnection: close\r\n\r\n"
    );
    let server = async {
        let (mut socket, _) = listener.accept().await?;
        let mut request = [0_u8; 4096];
        check!(socket.read(&mut request).await? > 0);
        socket.write_all(response.as_bytes()).await?;
        Ok::<_, Box<dyn std::error::Error>>(())
    };
    let options = FetchOptions::default();
    let (served, result) = tokio::time::timeout(Duration::from_secs(5), async {
        tokio::join!(server, fetcher.get(&url, &options))
    })
    .await?;
    served?;
    match status {
        403 => check!(matches!(result, Err(FetchError::Http { status: 403, .. }))),
        429 => check!(matches!(result, Err(FetchError::Http { status: 429, .. }))),
        _ => return Err("fixture permits only observed 403/429".into()),
    }
    let conditions = fetcher.access_conditions().await?;
    check!(eq; conditions.len(), 1);
    let observed = conditions
        .into_iter()
        .next()
        .ok_or("observed access condition")?;
    check!(eq; observed.source, "original-fixture-source");
    check!(eq; observed.host, "127.0.0.1");
    check!(eq; observed.status, status);
    check!(eq; observed.kind, if status == 429 { AccessBlockKind::RateLimited } else { AccessBlockKind::Forbidden });
    check!(eq; observed.detail, url);
    check!(eq; observed.retry_after_seconds, Some(120));
    check!(observed.cooldown_until.is_some());
    Ok((fetcher, observed))
}
