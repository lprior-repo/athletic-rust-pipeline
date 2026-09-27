use std::collections::HashSet;
use std::path::PathBuf;

use census_domain::model::CanonicalMeet;
use census_domain::UsJurisdiction;
use serde::Deserialize;

use super::{collect, ResultOptions, StandingsCapture, SOURCE_ID};
use crate::athleticlive_athletes::MeetTarget;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};

#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    meets: Vec<CaptureEntry>,
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
    let file: ManifestFile = serde_json::from_str(body).map_err(|error| CrawlError::Invariant {
        detail: format!("the capture manifest does not parse: {error}"),
    })?;
    let mut options: Vec<ResultOptions> = Vec::with_capacity(file.meets.len());
    for (index, entry) in file.meets.into_iter().enumerate() {
        let position = index.saturating_add(1);
        let Some(state) = UsJurisdiction::parse(&entry.state) else {
            return Err(CrawlError::Invariant {
                detail: format!(
                    "manifest meet {position}: {:?} places no jurisdiction",
                    entry.state
                ),
            });
        };
        let meet = CanonicalMeet::new(
            Some(state),
            &entry.name,
            &entry.date,
            super::super::infer_level(&entry.name),
        );
        options.push(ResultOptions {
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
            standings: entry
                .standings
                .into_iter()
                .map(|capture| StandingsCapture {
                    run_id: capture.run_id,
                    path: capture.path,
                })
                .collect(),
            observed_on: observed_on.to_string(),
            limit: None,
        });
    }
    Ok(options)
}

fn select(
    input: &str,
    options: &ManifestOptions,
    mut entries: Vec<ResultOptions>,
) -> (Vec<ResultOptions>, String) {
    let read = entries.len();
    let mut seen: HashSet<u64> = HashSet::new();
    entries.retain(|entry| match entry.meet.as_ref() {
        Some(meet) => seen.insert(meet.athleticlive_meet_id),
        None => false,
    });
    let duplicates = read.saturating_sub(entries.len());

    let wanted: HashSet<UsJurisdiction> = options.states.iter().copied().collect();
    if !wanted.is_empty() {
        entries.retain(|entry| {
            entry
                .meet
                .as_ref()
                .is_some_and(|meet| wanted.contains(&meet.state))
        });
    }
    let in_scope = entries.len();
    if let Some(limit) = options.limit {
        entries.truncate(limit);
    }
    let accounting = format!(
        "manifest {input}: {read} meets read, {duplicates} repeated ids dropped, {in_scope} in \
         scope, {} read",
        entries.len()
    );
    (entries, accounting)
}

fn merge(report: &mut AdapterReport, mut one: AdapterReport, meet: u64) {
    report.rows = report.rows.saturating_add(one.rows);
    report.requests = report.requests.saturating_add(one.requests);
    report.from_cache = report.from_cache.saturating_add(one.from_cache);
    report.errors = report.errors.saturating_add(one.errors);
    for note in one.notes.drain(..) {
        report.note(format!("meet {meet}: {note}"));
    }
}

pub async fn collect_manifest(
    ctx: &AdapterContext<'_>,
    options: &ManifestOptions,
) -> CrawlResult<AdapterReport> {
    let Some(input) = options.input.as_deref() else {
        return Err(CrawlError::Invariant {
            detail: "athleticlive_results requires --input <manifest.json>: this route reads the \
                     captures an operator staged and issues no request"
                .to_string(),
        });
    };
    let body = std::fs::read_to_string(input).map_err(|source| CrawlError::Io {
        path: PathBuf::from(input),
        source,
    })?;
    let entries = parse_manifest(&body, &options.observed_on)?;
    let (entries, accounting) = select(input, options, entries);

    let mut report = AdapterReport::new(SOURCE_ID, "result rows");
    report.note(accounting);
    for entry in &entries {
        let Some(meet) = entry.meet.as_ref() else {
            continue;
        };
        let id = meet.athleticlive_meet_id;
        merge(&mut report, collect(ctx, entry).await?, id);
    }
    report.note(format!(
        "imported {} meets from the manifest; requests: 0 by construction",
        entries.len()
    ));
    Ok(report)
}
