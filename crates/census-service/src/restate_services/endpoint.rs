use std::sync::Arc;

use restate_sdk::prelude::*;
use tokio::sync::Semaphore;

use athleticnet_browser::clock::SystemClock;
use athleticnet_browser::BrowserSettings;
use census_crawl::net::bridge::BrowserLane;
use census_store::clock::Clock;
use census_store::Store;

use super::limits::census_service;
use super::{
    Bests, BrowserSession, Census, Consolidate, Ingest, Jobs, JurisdictionCensus, NationalCensus,
    Report, SchoolAddressJoin, Sweep, TeamsSource, Workbook,
};
use crate::spawn::Spawner;

pub const MAX_ENDPOINT_CONCURRENCY: usize = Semaphore::MAX_PERMITS;

#[derive(Debug, thiserror::Error)]
#[error("max-concurrent {value} is outside 1..={ceiling}")]
pub struct ConcurrencyTooLarge {
    pub value: usize,
    pub ceiling: usize,
}

pub fn validate_concurrency(max_concurrent: usize) -> Result<usize, ConcurrencyTooLarge> {
    if max_concurrent == 0 || max_concurrent > MAX_ENDPOINT_CONCURRENCY {
        return Err(ConcurrencyTooLarge {
            value: max_concurrent,
            ceiling: MAX_ENDPOINT_CONCURRENCY,
        });
    }
    Ok(max_concurrent)
}

#[tracing::instrument(skip_all, fields(max_concurrent))]
pub fn build_endpoint(
    store: Arc<Store>,
    max_concurrent: usize,
    region: Arc<Spawner>,
    serves_lane: Option<BrowserSettings>,
    uses_lane: Option<BrowserLane>,
) -> Result<Endpoint, ConcurrencyTooLarge> {
    let max_concurrent = validate_concurrency(max_concurrent)?;
    let clock: Arc<dyn Clock> = Arc::new(census_store::clock::SystemClock);
    let load = Arc::new(Semaphore::new(max_concurrent));
    let jobs = Jobs::new(Arc::clone(&store), load, Arc::clone(&region));
    let jurisdiction = JurisdictionCensus::new(Arc::clone(&store), Arc::clone(&clock), uses_lane);
    let mut builder = Endpoint::builder()
        .bind(census_service(Census::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            jobs.clone(),
        )))
        .bind(census_service(Consolidate::new(jobs.clone())))
        .bind(census_service(Report::new(jobs.clone())))
        .bind(census_service(Bests::new(jobs.clone())))
        .bind(census_service(Workbook::new(jobs.clone())))
        .bind(census_service(SchoolAddressJoin::new(jobs.clone())))
        .bind(census_service(Ingest::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            Arc::clone(&region),
        )))
        .bind(census_service(Sweep::new(
            Arc::clone(&store),
            Arc::clone(&clock),
            Arc::clone(&region),
        )))
        .bind(census_service(TeamsSource::new(jurisdiction.clone(), jobs)))
        .bind(census_service(jurisdiction))
        .bind(census_service(NationalCensus::new(clock)));
    if let Some(settings) = serves_lane {
        builder = builder.bind(BrowserSession::new(settings, Arc::new(SystemClock)));
    }
    let endpoint = builder.build();
    Ok(endpoint)
}
