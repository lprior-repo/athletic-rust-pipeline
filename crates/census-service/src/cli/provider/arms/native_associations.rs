use anyhow::Result;
use census_crawl::{self as providers, AdapterContext, AdapterReport};

use super::super::ProviderArgs;

pub(crate) async fn chsaa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::chsaa::collect(
        context,
        &providers::chsaa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn tssaa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::tssaa::collect(
        context,
        &providers::tssaa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}
