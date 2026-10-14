use super::*;
use census_domain::model::{
    AcquisitionFailureV1, ResponseCheckpointRefusalV1, SOURCE_ACQUISITION_PHASE,
};
use census_store::{AcquisitionCheckpointV1, AcquisitionTerminal, ImmutableAttemptResult};

fn terminals(fixture: &Fixture) -> TestResult<Vec<ImmutableAttemptResult>> {
    let rows = fixture.store.journal_payloads(SOURCE_ACQUISITION_PHASE)?;
    Ok(rows
        .into_iter()
        .filter_map(|row| serde_json::from_value(row).ok())
        .collect())
}

#[tokio::test]
async fn actual_overbound_response_header_keeps_typed_status_and_url_checkpoint_refusal(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let headers = format!("ETag: {}\r\n", "x".repeat(1025));
    let fetch = async {
        let outcome = fixture.acquire(2026, false).await?;
        check!(matches!(
            outcome,
            Err(crate::CrawlError::Fetch(
                crate::net::FetchError::CaptureDomain(_)
            ))
        ));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(())
    };
    let replies = [Reply {
        status: 200,
        body: BODY,
        headers: &headers,
        before: Mutation::None,
    }];
    tokio::try_join!(
        serve(listener, replies, Arc::clone(&fixture.requests)),
        fetch
    )?;
    let fixture = fixture.reopen()?;
    let rows = terminals(&fixture)?;
    check!(eq; rows.len(), 1);
    let expected = format!(
        "https://{HOST}/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
        crate::milesplit::OWNED_FIELDS
    );
    check!(rows.iter().any(|row| matches!(row.terminal(),
        AcquisitionTerminal::FailedUncaptured {
            checkpoint: AcquisitionCheckpointV1::Armed { .. },
            failure: AcquisitionFailureV1::ResponseCheckpointRefused {
                status, response_url, reason: ResponseCheckpointRefusalV1::UnrepresentableResponse,
            },
        } if status.get() == 200 && response_url.as_str() == expected)));
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn actual_malformed_429_preserves_source_cooldown_and_blocks_new_work_before_http(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let headers = format!("Retry-After: 300\r\nETag: {}\r\n", "x".repeat(1025));
    let fetch = async {
        let outcome = fixture.acquire(2025, true).await?;
        check!(matches!(
            outcome,
            Err(crate::CrawlError::Fetch(
                crate::net::FetchError::CaptureDomain(_)
            ))
        ));
        let blocked = fixture.acquire(2026, true).await?;
        check!(matches!(
            blocked,
            Err(crate::CrawlError::Fetch(
                crate::net::FetchError::Cooldown { .. }
            ))
        ));
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
    let conditions = fixture.fetcher.access_conditions().await?;
    check!(eq; conditions.len(), 1);
    check!(
        fixture
            .fetcher
            .host_blocked(HOST, &crate::net::now_iso8601())
            .await
    );
    check!(eq; fixture.count(), 1);
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    Ok(())
}

#[tokio::test]
async fn actual_truncated_bodies_reopen_with_observed_failures_and_three_attempt_limit(
) -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let requests = Arc::clone(&fixture.requests);
    let fetch = async move {
        let mut fixture = fixture;
        for ordinal in 1..=3 {
            let outcome = fixture.acquire(2026, true).await?;
            check!(matches!(
                outcome,
                Err(crate::CrawlError::Fetch(
                    crate::net::FetchError::Transport { .. }
                ))
            ));
            fixture = fixture.reopen()?;
            let rows = terminals(&fixture)?;
            check!(rows
                .iter()
                .any(|row| row.attempt().ordinal().get() == ordinal
                    && matches!(
                        row.terminal(),
                        AcquisitionTerminal::FailedUncaptured {
                            checkpoint: AcquisitionCheckpointV1::ResponseObserved { .. },
                            failure: AcquisitionFailureV1::Transport,
                        }
                    )));
        }
        let fourth = fixture.acquire(2026, true).await?;
        check!(matches!(
            fourth,
            Err(crate::CrawlError::Fetch(crate::net::FetchError::CaptureStore(error)))
                if matches!(error.as_ref(), census_store::StoreError::Capture(
                    census_store::CaptureError::AcquisitionBudgetExhausted
                ))
        ));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(fixture)
    };
    let replies = std::array::from_fn::<_, 3, _>(|_| Reply {
        status: 200,
        body: b"{",
        headers: "",
        before: Mutation::TruncateBody,
    });
    let ((), fixture) = tokio::try_join!(serve(listener, replies, requests), fetch)?;
    check!(eq; fixture.count(), 3);
    let keys = fixture.store.journal_keys(SOURCE_ACQUISITION_PHASE)?;
    check!(eq; keys.iter().filter(|key| key.starts_with("slot/")).count(), 1);
    check!(eq; keys.iter().filter(|key| key.ends_with("/response")).count(), 3);
    check!(eq; keys.iter().filter(|key| key.ends_with("/terminal")).count(), 3);
    Ok(())
}
