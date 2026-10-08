use super::{adapter_context, collect_error, AdapterScope, SourceRuntime};
use crate::restate_services::{blocking, JobError};
use census_crawl::{AdapterContext, AdapterReport, CollectionDisposition};
use census_domain::{model::CanonicalSchool, UsJurisdiction};
use futures::{stream, StreamExt, TryStreamExt};
use std::sync::Arc;
use tokio::sync::mpsc;

mod reader;
mod reports;

#[tracing::instrument(skip_all)]
pub(super) async fn collect(
    runtime: SourceRuntime<'_>,
    jurisdiction: UsJurisdiction,
    scope: AdapterScope<'_>,
) -> Result<AdapterReport, JobError> {
    let (sender, receiver) = mpsc::channel(2);
    let store = Arc::clone(runtime.store);
    let producer = blocking(Arc::clone(runtime.region), move || {
        reader::produce(&store, jurisdiction, sender)
    });
    let ctx = adapter_context(runtime.store, runtime.fetcher, scope, None);
    let consumer = windows(&ctx, receiver);
    let (_, mut report) = tokio::try_join!(producer, consumer)?;
    if report.unit == "unmeasured school inventory" {
        reports::owe(
            &mut report,
            format!("{}/school-sites/inventory", jurisdiction.code()),
        )?;
    }
    report.finish_frontier();
    Ok(report)
}

#[tracing::instrument(skip_all)]
async fn windows(
    ctx: &AdapterContext<'_>,
    receiver: mpsc::Receiver<Vec<CanonicalSchool>>,
) -> Result<AdapterReport, JobError> {
    stream::unfold(receiver, |mut receiver| async move {
        receiver.recv().await.map(|rows| (rows, receiver))
    })
    .map(Ok::<_, JobError>)
    .try_fold(
        AdapterReport::new("sidearm_staff", "unmeasured school inventory"),
        |mut report, schools| async move {
            report.unit = "school contacts".to_string();
            reports::merge(&mut report, contacts(ctx, schools).await?)?;
            Ok(report)
        },
    )
    .await
}

#[tracing::instrument(skip_all)]
async fn contacts(
    ctx: &AdapterContext<'_>,
    mut schools: Vec<CanonicalSchool>,
) -> Result<AdapterReport, JobError> {
    let options = census_crawl::sidearm_staff::Options {
        limit: None,
        refresh: ctx.refresh,
        observed_on: ctx.observed_on.clone(),
        states: Vec::new(),
        school_names: Vec::new(),
    };
    let mut report = census_crawl::sidearm_staff::collect_discovered(ctx, &options, &schools)
        .await
        .map_err(collect_error)?;
    let office = stream::iter(schools.iter_mut())
        .map(Ok::<_, JobError>)
        .try_fold(
            AdapterReport::new("school_sites", "school contacts"),
            |mut report, school| async move {
                let page = census_crawl::school_sites::collect_contacts(ctx, school)
                    .await
                    .map_err(collect_error)?;
                reports::merge(&mut report, page)?;
                Ok(report)
            },
        )
        .await?;
    reports::merge(&mut report, office)?;
    if report.disposition == CollectionDisposition::Unknown {
        report.disposition = CollectionDisposition::Partial;
    }
    Ok(report)
}
