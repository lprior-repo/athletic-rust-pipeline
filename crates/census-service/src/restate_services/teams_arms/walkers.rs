use std::sync::Arc;

use census_domain::model::SchoolYear;
use census_domain::UsJurisdiction;

use crate::census;
use census_crawl::{net::Fetcher, AdapterReport};
use census_store::Store;

use super::super::jobs::{adapter_context, collect_error, AdapterScope};

#[derive(Clone, Copy)]
pub(super) struct Crawl<'a> {
    pub(super) store: &'a Arc<Store>,
    pub(super) fetcher: &'a Arc<Fetcher>,
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) season: SchoolYear,
    pub(super) refresh: bool,
    pub(super) at: &'a str,
    pub(super) as_of: chrono::NaiveDate,
}

pub(super) fn crawl<'a>(
    store: &'a Arc<Store>,
    fetcher: &'a Arc<Fetcher>,
    jurisdiction: UsJurisdiction,
    scope: AdapterScope<'a>,
) -> Crawl<'a> {
    Crawl {
        store,
        fetcher,
        jurisdiction,
        season: scope.season,
        refresh: scope.refresh,
        at: scope.at,
        as_of: scope.as_of,
    }
}

pub(super) async fn walk_milesplit(
    crawl: Crawl<'_>,
) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        refresh,
        ..
    } = crawl;
    let teams = census::collect_state_teams(fetcher, store, jurisdiction, refresh)
        .await
        .map_err(collect_error)?;
    let mut report = AdapterReport::new(census::SOURCE, "teams");
    report.rows = u64::try_from(teams.len()).map_err(|_| super::super::JobError::Terminal {
        message: format!("milesplit team count {} exceeds u64", teams.len()),
    })?;
    Ok(report)
}

pub(super) async fn walk_wiaa(crawl: Crawl<'_>) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::wiaa::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::wiaa::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}

pub(super) async fn walk_mshsl(crawl: Crawl<'_>) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::mshsl::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::mshsl::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}

pub(super) async fn walk_plain_names(
    crawl: Crawl<'_>,
) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::plain_names::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::plain_names::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}

pub(super) async fn walk_ihsa(crawl: Crawl<'_>) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::ihsa::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::ihsa::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}

pub(super) async fn walk_ks(crawl: Crawl<'_>) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::ks::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::ks::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}

pub(super) async fn walk_coach_directories(
    crawl: Crawl<'_>,
) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::coach_directories::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
        school_names: Vec::new(),
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::coach_directories::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}

pub(super) async fn walk_arbiter_orgs(
    crawl: Crawl<'_>,
) -> Result<AdapterReport, super::super::JobError> {
    let Crawl {
        store,
        fetcher,
        jurisdiction,
        season,
        refresh,
        at,
        as_of,
    } = crawl;
    let options = census_crawl::arbiter::Options {
        limit: None,
        refresh,
        observed_on: at.to_string(),
        states: vec![jurisdiction],
    };
    let context = adapter_context(
        store,
        fetcher,
        AdapterScope {
            season,
            refresh,
            at,
            as_of,
        },
        None,
    );
    let report = census_crawl::arbiter::collect(&context, &options)
        .await
        .map_err(collect_error)?;
    Ok(report)
}
