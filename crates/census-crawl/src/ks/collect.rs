use super::parse::{parse_ad_coach, parse_school};
use super::wire::KshsaaRecord;
use crate::directory::acquisition::{
    fail, owe, publish as persist, publish_school as persist_school, text,
};
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::SourceNamespace;
use census_domain::UsJurisdiction;
use census_store::Table;
use serde_json::Value;

const KSHSAA_API: &str = "https://kshsaa-api.kshsaa.org/directory/search/name/";

#[derive(Debug, Clone, Default)]
pub struct Options {
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new("ks", "schools");
    if !options.states.is_empty() && !options.states.contains(&UsJurisdiction::Kansas) {
        return Ok(report);
    }
    let before = ctx.fetcher.stats().await;
    let url = format!("{KSHSAA_API}a/");
    let fetch = crate::net::FetchOptions {
        refresh: ctx.refresh || options.refresh,
        ..ctx.fetch_options()
    };
    match ctx.fetcher.get(&url, &fetch).await {
        Ok(capture) => match text(&capture).and_then(|body| {
            serde_json::from_str::<Value>(body).map_err(|source| CrawlError::Decode {
                url: url.clone(),
                source,
            })
        }) {
            Ok(Value::Array(records)) => {
                records
                    .into_iter()
                    .enumerate()
                    .try_for_each(|(ordinal, value)| {
                        let record = match serde_json::from_value::<KshsaaRecord>(value) {
                            Ok(record) => record,
                            Err(error) => {
                                return fail(&mut report, &format!("{url}#row={ordinal}"), error)
                            }
                        };
                        if options.limit.is_some_and(|limit| ordinal >= limit) {
                            return owe(&mut report, format!("{url}#school={}", record.identifier));
                        }
                        collect_record(ctx, &record, &capture, &mut report)
                    })?
            }
            Ok(_) => fail(&mut report, &url, "missing school array")?,
            Err(error) => fail(&mut report, &url, error)?,
        },
        Err(error) => fail(&mut report, &url, error)?,
    }
    owe(&mut report, KSHSAA_API)?;
    report.rejections = report
        .rejections
        .checked_add(1)
        .ok_or_else(|| CrawlError::Arithmetic {
            detail: "KSHSAA refusal counter overflow".to_string(),
        })?;
    report.note("full inventory refused: the qualified capture and collector cover only letter a; the remaining KSHSAA directory is not qualified");
    let after = ctx.fetcher.stats().await;
    report.requests = after
        .physical_requests()
        .saturating_sub(before.physical_requests());
    report.from_cache = after.cache_hits.saturating_sub(before.cache_hits);
    Ok(report)
}

fn collect_record(
    ctx: &AdapterContext<'_>,
    record: &KshsaaRecord,
    capture: &crate::net::FetchOutcome,
    report: &mut AdapterReport,
) -> CrawlResult<()> {
    let provenance = (capture.url.as_str(), capture.fetched_at.as_str());
    let Some((school, school_id)) = parse_school(record, provenance.0, provenance.1) else {
        report.rejections = report.rejections.saturating_add(1);
        return owe(
            report,
            format!("{}#school={}", provenance.0, record.identifier),
        );
    };
    let mut coach = parse_ad_coach(record, &school_id, provenance.0, provenance.1);
    for coach in coach.as_mut_slice() {
        crate::coach_directories::record_appointment(
            coach,
            Some(crate::coach_directories::PublishedListing {
                role: "AD",
                program: &record.school_name,
                source: "ks",
                school_year: ctx.school_year,
            }),
            crate::coach_directories::StaffCapture {
                url: &capture.url,
                observed_on: &capture.fetched_at,
                sha256: &capture.content_digest,
            },
        )?;
    }
    let locator = format!("{}#school={}", provenance.0, record.identifier);
    let written = persist_school(
        ctx,
        ("ks", &locator),
        (
            &SourceNamespace::association_school(super::ASSOCIATION),
            &school,
            provenance.1,
        ),
        report,
    )?;
    persist(
        ctx,
        ("ks", &locator),
        Table::Coaches,
        coach.as_slice(),
        report,
    )?;
    report.rows = report
        .rows
        .saturating_add(u64::try_from(written).map_or(u64::MAX, |value| value));
    report.with_email = report.with_email.saturating_add(u64::from(
        coach.is_some_and(|coach| coach.has_published_email()),
    ));
    Ok(())
}
