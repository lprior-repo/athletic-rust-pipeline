use super::*;
use crate::net::Fetcher;
use census_store::Store;
use std::collections::HashMap;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

mod binding;
mod durable_capture;
mod fixture;
mod large_partial_replay;
use fixture::*;

#[test]
fn a_signed_meet_identifier_cannot_rebind_a_public_owned_capture() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let dir = tempfile::tempdir()?;
            let fetcher = Fetcher::new(
                dir.path().join("http"),
                None,
                std::time::Duration::ZERO,
                HashMap::new(),
                vec!["milesplit.com".into()],
            )?
            .with_offline(true);
            let mut reference =
                ResultSetRef::parse("https://al.milesplit.com/meets/725218/results/1266814/raw")
                    .ok_or("reference")?;
            seed_owned(&fetcher, &reference, crate::milesplit::troy_fixture::BODY)?;
            reference.meet_id = "+725218".to_string();
            let store = Store::open(dir.path().join("store"))?;
            let capture = crate::net::durable_capture_context(&dir.path().join("capture"))?;
            let ctx = context(&store, &fetcher, &capture)?;
            match crate::milesplit::fetch_owned_meet(
                &ctx,
                &reference,
                &crate::net::FetchOptions::default(),
            )
            .await
            {
                Err(crate::CrawlError::Schema { detail, .. }) => {
                    check!(eq; detail, "invalid structured meet ID");
                }
                other => {
                    return Err(format!(
            "noncanonical source identity must not acquire another meet's capture: {other:?}"
        )
                    .into())
                }
            }
            check!(eq; fetcher.stats().await.cache_hits, 0);
            Ok(())
        })
}
