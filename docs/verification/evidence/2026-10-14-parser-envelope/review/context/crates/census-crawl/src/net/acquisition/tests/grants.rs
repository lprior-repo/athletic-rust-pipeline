use super::*;
use crate::net::CaptureTransportError;
use crate::{CaptureContext, CrawlError};

#[tokio::test]
async fn target_bound_linear_grant_refuses_foreign_meet_then_consumed_reuse() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let owner = fixture.context_owner(2026, false)?;
    let ctx = fixture.context(&owner, false)?;
    let foreign = crate::milesplit::ResultSetRef::parse(
        "https://mi.milesplit.com/meets/725219/results/1266814/raw",
    )
    .ok_or("reference")?;
    let result = crate::milesplit::fetch_owned_meet(&ctx, &foreign, &ctx.fetch_options()).await;
    check!(matches!(
        result,
        Err(CrawlError::Capture(CaptureTransportError::TargetMismatch))
    ));
    check!(eq; fixture.count(), 0);
    let fetch = async {
        let captured =
            crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &ctx.fetch_options())
                .await?;
        let repeat =
            crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &ctx.fetch_options())
                .await;
        check!(matches!(
            repeat,
            Err(CrawlError::Capture(CaptureTransportError::GrantConsumed))
        ));
        Ok::<_, Box<dyn std::error::Error + Send + Sync>>(captured)
    };
    tokio::try_join!(
        serve(listener, [Reply::ok(BODY)], Arc::clone(&fixture.requests)),
        fetch
    )?;
    check!(eq; fixture.count(), 1);
    Ok(())
}

#[tokio::test]
async fn lifecycle_four_legacy_context_refuses_owned_role_before_http() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    let owner = fixture.context_owner(2026, false)?;
    let mut ctx = fixture.context(&owner, false)?;
    ctx.capture = CaptureContext::LegacyMetadata;
    let result =
        crate::milesplit::fetch_owned_meet(&ctx, &fixture.reference()?, &ctx.fetch_options()).await;
    check!(matches!(
        result,
        Err(CrawlError::Store(census_store::StoreError::Capture(_)))
    ));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}

#[tokio::test]
async fn adopted_foreign_cache_root_refuses_before_grant_or_http() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let mut fixture = Fixture::new(&listener)?;
    fixture.fetcher.cache_dir = fixture.root.path().join("foreign-http");
    std::fs::create_dir(&fixture.fetcher.cache_dir)?;
    let result = fixture.acquire(2026, false).await?;
    check!(matches!(
        result,
        Err(CrawlError::Store(census_store::StoreError::Capture(
            census_store::CaptureError::CacheRootMismatch
        )))
    ));
    check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    Ok(())
}
