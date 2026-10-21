use super::*;
use assertions::{retained, Witness};
use server::{serve, Reply};
use std::sync::Arc;

#[tokio::test]
async fn expired_dispatch_clock_refuses_before_arm_and_keeps_owned_policy() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    let robots = seed_robots(&fixture, listener).await?;
    let original = retained(&fixture.store, &robots)?;
    check!(crate::net::robots::policy::capture_is_current(
        robots.capture(),
        chrono::Utc::now(),
    )?);
    let expired = chrono::DateTime::parse_from_rfc3339(&robots.capture().fetched_at)?
        .with_timezone(&chrono::Utc)
        .checked_add_signed(chrono::Duration::seconds(86_401))
        .ok_or("controlled expired dispatch time")?;
    fixture.fetcher.capture_clock =
        crate::net::time::CaptureClock::Injected(Arc::new(fixture::FixedClock(expired)));
    check!(
        matches!(fixture.acquire().await, Err(FetchError::Policy { detail })
        if detail == "robots policy is no longer current at dispatch")
    );
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    check!(eq; retained(&fixture.store, &robots)?, original);
    refusal_without_arm(&fixture)?;
    let before = Witness::capture(&fixture.store)?;
    check!(matches!(
        fixture.acquire().await,
        Err(FetchError::Capture(CaptureTransportError::Settled))
    ));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 1);
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(eq; retained(&fixture.store, &robots)?, original);
    Ok(())
}

async fn seed_robots(
    fixture: &Fixture,
    listener: TcpListener,
) -> TestResult<crate::net::CapturedFetch> {
    let fetch = async {
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(
            fixture
                .fetcher
                .get_public_resource_root(
                    &fixture.context(),
                    PublicResourceRootKindV1::Robots,
                    &FetchOptions::default(),
                    crate::net::robots::Admission::Unrestricted,
                )
                .await?,
        )
    };
    let (requests, robots) = tokio::try_join!(serve(listener, [Reply::robots(ALLOW)]), fetch)?;
    check!(eq; requests.len(), 1);
    Ok(robots)
}

fn refusal_without_arm(fixture: &Fixture) -> TestResult {
    let work = fixture.work(PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let prefix = format!("work/{}/", work.id()?);
    let mut keys: Vec<_> = fixture
        .store
        .journal_keys(CaptureProtocolVersion::V2.phase())?
        .into_iter()
        .collect();
    keys.retain(|key| key.starts_with(&prefix));
    keys.sort();
    let terminal_key = format!("{prefix}attempt/1/terminal");
    let mut expected = vec![
        format!("{prefix}identity"),
        format!("{prefix}attempt/1/reserved"),
        terminal_key.clone(),
    ];
    expected.sort();
    check!(eq; keys, expected);
    let terminal: census_store::ImmutableAttemptResult = serde_json::from_value(
        fixture
            .store
            .journal_payload(CaptureProtocolVersion::V2.phase(), &terminal_key)?
            .ok_or("unarmed policy refusal terminal")?,
    )?;
    check!(matches!(
        terminal.terminal(),
        census_store::AcquisitionTerminal::NotDispatched {
            failure: AcquisitionFailureV1::AdmissionRefused,
        }
    ));
    let operation = format!("{}/{terminal_key}", CaptureProtocolVersion::V2.phase());
    let receipt = fixture
        .store
        .receipt(&operation)?
        .ok_or("unarmed refusal receipt")?;
    check!(eq; receipt.digest, serialized_digest(&terminal)?);
    Ok(())
}
