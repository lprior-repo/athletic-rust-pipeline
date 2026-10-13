use super::report_census;
use crate::census::MeetCensus;
use census_crawl::{AdapterContext, AdapterReport, CollectionDisposition};
use restate_sdk::prelude::HandlerError;

pub(super) fn athleticnet() -> Result<MeetCensus, HandlerError> {
    blocked(
        "athleticnet",
        "athleticnet:historical-discovery-producer-capture",
        "prerequisite missing: no qualified Athletic.net historical-discovery frontier or producer capture is bound to this run; existing whole-meet and athlete collectors require capture-derived meet IDs or an athlete registry, not invented IDs; registry presence and zero selected inputs do not complete discovery",
    )
}

pub(super) async fn unarmed(
    context: &AdapterContext<'_>,
    slug: &str,
    jurisdiction: census_domain::UsJurisdiction,
) -> Result<MeetCensus, HandlerError> {
    match slug {
        "athleticlive" => athleticlive(context).await,
        "athleticlive_athletes" => athletes(context, jurisdiction).await,
        "tfrrs" => tfrrs(context).await,
        _ => blocked(
            slug,
            &format!("{slug}:historical-discovery-arm"),
            "prerequisite missing: the selected meet-capable source has no durable historical-discovery arm; another stage's registration does not complete this discovery obligation",
        ),
    }
}

async fn athleticlive(context: &AdapterContext<'_>) -> Result<MeetCensus, HandlerError> {
    let result = census_crawl::athleticlive::collect(context, &Default::default()).await;
    let reason = match result {
        Err(error) => format!("prerequisite missing: qualified published AthleticLIVE meet CSV and physical producer metadata are not bound to this run: {error}"),
        Ok(_) => return Err(super::jobs::invariant("missing AthleticLIVE input unexpectedly collected")),
    };
    blocked(
        "athleticlive",
        "athleticlive:meet-csv-producer-capture",
        &reason,
    )
}

async fn athletes(
    context: &AdapterContext<'_>,
    jurisdiction: census_domain::UsJurisdiction,
) -> Result<MeetCensus, HandlerError> {
    let options = census_crawl::athleticlive_athletes::Options {
        limit: Some(0),
        states: vec![jurisdiction],
        observed_on: context.observed_on.clone(),
        ..Default::default()
    };
    let mut report = census_crawl::athleticlive_athletes::collect(context, &options)
        .await
        .map_err(|error| super::super::job_error(super::jobs::collect_error(error)))?;
    report.disposition = CollectionDisposition::Blocked;
    report.note("prerequisite missing: no qualified bounded AthleticLIVE athlete-discovery manifest with physical producer captures is bound to this run; capture-derived meet targets remain unfinished without acquiring invented seeds; zero selected targets do not complete discovery");
    report_census("athleticlive_athletes", report)
}

async fn tfrrs(_context: &AdapterContext<'_>) -> Result<MeetCensus, HandlerError> {
    blocked(
        "tfrrs",
        "tfrrs:terms-refusal",
        "DirectAthletics terms refuse automated acquisition for any purpose without written permission",
    )
}

fn blocked(slug: &str, prerequisite: &str, reason: &str) -> Result<MeetCensus, HandlerError> {
    let mut report = AdapterReport::new(slug, "meets");
    report.disposition = CollectionDisposition::Blocked;
    report.unfinished.push(prerequisite.into());
    report.note(reason);
    report_census(slug, report)
}
