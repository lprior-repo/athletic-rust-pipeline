//! The arms whose payload is a roster, a coach directory or a state association's own pages.
//! One function per registry slug, each marshalling [`ProviderArgs`] into its adapter's `Options`.

use anyhow::{bail, Result};
use census_crawl::{self as providers, AdapterContext, AdapterReport};
use census_store::Store;

use std::path::Path;

use super::super::ProviderArgs;

/// The researched coach-contact artifact, addressed by its registry slug.
///
/// `--input` names the CSV; the dedicated `import-coaches` subcommand takes the same file. The
/// import is an artifact read, so it issues no HTTP request.
pub(crate) fn coach_contacts_report(
    store: &Store,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    let Some(csv) = args.input.as_deref() else {
        bail!("adapter coach_contacts needs --input <path/to/coach-contacts.csv>");
    };
    Ok(providers::coach_contacts::import_csv(
        store,
        Path::new(csv),
        &observed_on,
    )?)
}

pub(crate) async fn ks_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ks::collect(
        context,
        &providers::ks::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn wiaa_results_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::wiaa_results::collect(
        context,
        &providers::wiaa_results::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            seasons: args.seasons.clone(),
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn wiaa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::wiaa::collect(
        context,
        &providers::wiaa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn ihsa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ihsa::collect(
        context,
        &providers::ihsa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn ihsa_tournament_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ihsa::tournament::collect(
        context,
        &providers::ihsa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn ohsaa_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::ohsaa::collect(
        context,
        &providers::ohsaa::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn mshsl_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::mshsl::collect(
        context,
        &providers::mshsl::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}

pub(crate) async fn plain_names_report(
    context: &AdapterContext<'_>,
    args: &ProviderArgs,
    observed_on: String,
) -> Result<AdapterReport> {
    Ok(providers::plain_names::collect(
        context,
        &providers::plain_names::Options {
            limit: args.limit,
            refresh: args.refresh,
            observed_on,
            states: args.jurisdictions()?,
            school_names: args.school_names.clone(),
        },
    )
    .await?)
}
