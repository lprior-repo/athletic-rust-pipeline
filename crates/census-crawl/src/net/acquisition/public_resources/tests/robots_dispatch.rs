use super::*;
use assertions::Witness;
use census_store::{AcquireDecision, AcquisitionReadBudget};
use std::time::Duration;

#[derive(Clone, Copy)]
enum Route {
    PublicWrapper,
    InternalPublicHttp,
}

#[tokio::test]
async fn historical_robots_work_refuses_without_attempts_before_and_after_reopen() -> TestResult {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let fixture = Fixture::new(&listener)?;
    fixture
        .context()
        .registered(PublicResourceRootKindV1::Robots)?;
    let before = Witness::capture(&fixture.store)?;
    refuse_routes(&fixture).await?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    let fixture = fixture.reopen()?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    refuse_routes(&fixture).await?;
    check!(eq; Witness::capture(&fixture.store)?, before);
    check!(
        tokio::time::timeout(Duration::from_millis(10), listener.accept())
            .await
            .is_err()
    );
    Ok(())
}

async fn refuse_routes(fixture: &Fixture) -> TestResult {
    for route in [Route::PublicWrapper, Route::InternalPublicHttp] {
        let owned = fixture
            .context()
            .registered(PublicResourceRootKindV1::Robots)?;
        let origin = url::Url::parse(owned.work().request().original_url())?.origin();
        let options = FetchOptions::default();
        let call = async {
            match route {
                Route::PublicWrapper => {
                    fixture
                        .fetcher
                        .get_acquisition(&fixture.store, owned, &options)
                        .await
                }
                Route::InternalPublicHttp => {
                    fixture
                        .fetcher
                        .get_acquisition_with_scope(
                            &fixture.store,
                            owned,
                            &options,
                            crate::net::public_http::TransportScope::PublicHttp(&origin),
                        )
                        .await
                }
            }
        };
        let error = tokio::time::timeout(Duration::from_secs(2), call)
            .await?
            .err()
            .ok_or("robots dispatch unexpectedly succeeded")?;
        check!(!error.retryable());
        check!(matches!(error, FetchError::Policy { detail }
            if detail == "robots.txt dispatch is disabled"));
        let work = fixture
            .context()
            .registered(PublicResourceRootKindV1::Robots)?;
        check!(matches!(
            fixture
                .store
                .inspect_acquisition_work(&work, &mut AcquisitionReadBudget::for_work())?,
            AcquireDecision::Ready
        ));
        assertions::no_directory_attempt(&fixture.store, work.work())?;
        check!(eq; fixture.fetcher.stats().await.physical_requests(), 0);
    }
    Ok(())
}
