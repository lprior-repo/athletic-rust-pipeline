//! The operator's capture manifest: one entry per meet whose documents were captured.
//!
//! `provider athleticlive_results --input <manifest.json>` reads this file. Each entry names one
//! meet exactly as the harvest publishes it — the same facts the meet-index route journals — plus
//! the captures staged for it. The meet is minted from those facts, which is why the results route
//! lands on the canonical meet id the meet-index route already minted instead of minting a second
//! one for the same meet.
//!
//! The route issues no request: every capture is a path the operator staged. A capture the harvest
//! never took is simply absent from the entry, and the run reports what it could not fold.

use std::collections::HashSet;
use std::path::PathBuf;

use census_domain::model::CanonicalMeet;
use census_domain::UsJurisdiction;
use serde::Deserialize;

use super::{collect, ResultOptions, StandingsCapture, SOURCE_ID};
use crate::athleticlive_athletes::MeetTarget;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};

/// The manifest file: the meets whose captures are staged.
#[derive(Debug, Clone, Deserialize)]
struct ManifestFile {
    meets: Vec<CaptureEntry>,
}

/// One meet's captures, as the manifest publishes them.
#[derive(Debug, Clone, Deserialize)]
pub struct CaptureEntry {
    /// AthleticLIVE's own meet id: the key the harvest and the athlete route both use.
    pub athleticlive_meet_id: u64,
    /// The tenant whose pages published the captures.
    pub tenant: String,
    /// The meet name, exactly as the harvest publishes it.
    pub name: String,
    /// The meet's jurisdiction: a USPS code (`MI`) or a name (`Michigan`).
    pub state: String,
    /// The meet date, `YYYY-MM-DD`.
    pub date: String,
    /// The captured `meet_<meetId>/event_summary.json`, when the harvest took one.
    #[serde(default)]
    pub summary: Option<String>,
    /// Captured event documents (`ind_res_list/_doc/<eventId>`), in the order to read them.
    #[serde(default)]
    pub documents: Vec<String>,
    /// Captured live standings, each paired with the run key whose race it publishes.
    #[serde(default)]
    pub standings: Vec<StandingsEntry>,
}

/// One captured live-standings payload and the run key it answers for.
#[derive(Debug, Clone, Deserialize)]
pub struct StandingsEntry {
    /// The run key the event published (`rui`: `4-1`, `19-1`).
    pub run_id: String,
    /// Path to the captured body of `liveRunStandings/<runId>.json`.
    pub path: String,
}

/// What a manifest import is asked for.
#[derive(Debug, Clone, Default)]
pub struct ManifestOptions {
    /// Path to the manifest JSON.
    pub input: Option<String>,
    /// Read at most this many meets, in manifest order.
    pub limit: Option<usize>,
    /// Observation date stamped on every evidence row.
    pub observed_on: String,
    /// Restrict to these jurisdictions; empty = every entry the file names.
    pub states: Vec<UsJurisdiction>,
}

/// Read a manifest body into one [`ResultOptions`] per meet, in manifest order.
///
/// A malformed manifest is an operator artifact, not a data row: it fails the run by name rather
/// than being skipped, so a harvest that dropped a field is never silently imported halfway.
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

/// The manifest's meets, deduplicated by AthleticLIVE id, narrowed to the asked-for jurisdictions
/// and capped — with the line that accounts for everything the selection dropped.
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

/// Fold one meet's report into the import's, tagging each note with the meet it came from so a
/// failure names the meet rather than the import.
fn merge(report: &mut AdapterReport, mut one: AdapterReport, meet: u64) {
    report.rows = report.rows.saturating_add(one.rows);
    report.requests = report.requests.saturating_add(one.requests);
    report.from_cache = report.from_cache.saturating_add(one.from_cache);
    report.errors = report.errors.saturating_add(one.errors);
    for note in one.notes.drain(..) {
        report.note(format!("meet {meet}: {note}"));
    }
}

/// Import every meet's captures the manifest names.
///
/// Meets are deduplicated by AthleticLIVE meet id: several tenants publishing one meet collapse to
/// the first entry, because duplicate ids would read the same captures twice.
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
