use census_domain::model::CanonicalMeet;
use census_domain::UsJurisdiction;
use serde::Deserialize;

use super::{collect, ResultOptions, StandingsCapture, SOURCE_ID};
use crate::athleticlive_athletes::MeetTarget;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};

mod admission;
mod selection;

#[derive(Debug, Deserialize)]
struct ManifestEnvelope<'a> {
    #[serde(borrow)]
    meets: Vec<&'a serde_json::value::RawValue>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CaptureEntry {
    pub athleticlive_meet_id: u64,
    pub tenant: String,
    pub name: String,
    pub state: String,
    pub date: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub documents: Vec<String>,
    #[serde(default)]
    pub standings: Vec<StandingsEntry>,
    #[serde(default)]
    pub captures: std::collections::BTreeMap<String, crate::net::cache::CacheMeta>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StandingsEntry {
    pub run_id: String,
    pub path: String,
}

#[derive(Debug, Clone, Default)]
pub struct ManifestOptions {
    pub input: Option<String>,
    pub limit: Option<usize>,
    pub observed_on: String,
    pub states: Vec<UsJurisdiction>,
}

pub fn parse_manifest(body: &str, observed_on: &str) -> CrawlResult<Vec<ResultOptions>> {
    admission::limit("LIVE manifest bytes", body.len(), admission::MAX_BYTES)?;
    let envelope: ManifestEnvelope<'_> =
        serde_json::from_str(body).map_err(|error| CrawlError::Invariant {
            detail: format!("the capture manifest does not parse: {error}"),
        })?;
    admission::limit(
        "LIVE manifest meets",
        envelope.meets.len(),
        admission::MAX_RECORDS,
    )?;
    let mut options = Vec::new();
    options
        .try_reserve_exact(envelope.meets.len())
        .map_err(|_| {
            admission::resource(
                "LIVE manifest allocation",
                envelope.meets.len(),
                admission::MAX_RECORDS,
            )
        })?;
    let mut records = 0usize;
    for (index, raw) in envelope.meets.into_iter().enumerate() {
        let entry: CaptureEntry =
            serde_json::from_str(raw.get()).map_err(|error| CrawlError::Invariant {
                detail: format!("the capture manifest does not parse: {error}"),
            })?;
        records = admission::admit(&entry, records)?;
        options.push(entry_options(entry, index, observed_on)?);
    }
    Ok(options)
}

fn entry_options(
    entry: CaptureEntry,
    index: usize,
    observed_on: &str,
) -> CrawlResult<ResultOptions> {
    let state = UsJurisdiction::parse(&entry.state).ok_or_else(|| CrawlError::Invariant {
        detail: format!(
            "manifest meet {}: {:?} places no jurisdiction",
            index.saturating_add(1),
            entry.state
        ),
    })?;
    let meet = CanonicalMeet::new(
        Some(state),
        &entry.name,
        &entry.date,
        super::super::infer_level(&entry.name),
    );
    let mut standings = Vec::new();
    standings
        .try_reserve_exact(entry.standings.len())
        .map_err(|_| {
            admission::resource(
                "LIVE manifest standings",
                entry.standings.len(),
                admission::MAX_RECORDS,
            )
        })?;
    standings.extend(entry.standings.into_iter().map(|capture| StandingsCapture {
        run_id: capture.run_id,
        path: capture.path,
    }));
    Ok(ResultOptions {
        meet: Some(MeetTarget {
            athleticlive_meet_id: entry.athleticlive_meet_id,
            meet_id: meet.id.as_str().to_string(),
            tenant: entry.tenant,
            name: entry.name,
            state,
            date: entry.date,
        }),
        summary: entry.summary,
        documents: entry.documents,
        standings,
        observed_on: observed_on.to_string(),
        limit: None,
        capture_metadata: entry.captures,
    })
}

pub async fn collect_manifest(
    ctx: &AdapterContext<'_>,
    options: &ManifestOptions,
) -> CrawlResult<AdapterReport> {
    let input = options.input.as_deref().ok_or_else(|| CrawlError::Invariant {
        detail: "athleticlive_results requires --input <manifest.json>: this route reads operator captures and issues no request".into(),
    })?;
    let body = admission::read(input)?;
    let entries = parse_manifest(&body, &options.observed_on)?;
    let (entries, unfinished, accounting) = selection::select(input, options, entries)?;
    let mut report = AdapterReport::new(SOURCE_ID, "result rows");
    report.unfinished = unfinished;
    report.note(accounting);
    for entry in &entries {
        let target = entry.meet.as_ref().ok_or_else(|| CrawlError::Invariant {
            detail: "selected manifest entry has no meet owner".into(),
        })?;
        match collect(ctx, entry).await {
            Ok(one) => selection::merge(&mut report, one, target.athleticlive_meet_id)?,
            Err(error) => crate::directory::acquisition::fail(
                &mut report,
                &format!("{input}#meet={}", target.athleticlive_meet_id),
                error,
            )?,
        }
    }
    report.note(format!(
        "imported {} selected meets from the manifest; requests: 0 by construction",
        entries.len()
    ));
    if !entries.is_empty() || !report.unfinished.is_empty() {
        report.finish_frontier();
    }
    Ok(report)
}
