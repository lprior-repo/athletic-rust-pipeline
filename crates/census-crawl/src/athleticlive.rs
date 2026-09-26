//! AthleticLIVE meet import (research-artifact adapter).
//!
//! Consumes the AthleticLIVE tenant/meet harvest produced by the source-research phase
//! (`athleticlive-midwest-2026-meets-all.csv`, one row per tenant x meet) and turns it into
//! canonical meets.
//!
//! Why this matters: 84-89% of AthleticLIVE meet documents carry an Athletic.net meet id. That
//! makes this artifact a *bridge*: it names specific Athletic.net meets (date, venue, id) without
//! issuing any Athletic.net request, so later targeted acquisition can address one meet directly
//! instead of enumerating Athletic.net's meet universe.
//!
//! This adapter performs no HTTP. `Options::input` MUST point at the CSV; the research corpus is
//! the source of record and is never edited here.
//!
//! # Layout
//!
//! `parse` reads the CSV and carries the harvest's vocabulary, `meets` mints the canonical meets,
//! and this module drives the run and narrates it.

use std::collections::{BTreeSet, HashSet};
use std::path::PathBuf;

use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult, FLUSH_UNITS};
use census_domain::model::{CanonicalMeet, EvidenceMethod};
use census_domain::UsJurisdiction;

mod docs;
mod map;
mod map_rows;
mod meets;
mod parse;
mod results;
mod standings;
mod wire;

pub use docs::parse_event_document;
pub use meets::build_meets;
pub use parse::{implausible_year, infer_level, parse_meets_csv, MeetRow};
pub use results::{
    collect as collect_results, collect_manifest, ManifestOptions, ResultOptions, StandingsCapture,
};
pub use wire::event_doc_url;

/// Adapter options (uniform across provider adapters plus `input`).
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Path to `athleticlive-midwest-2026-meets-all.csv` (or the `-seeds` variant).
    pub input: Option<String>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to these jurisdictions; empty = every state present in the file.
    pub states: Vec<UsJurisdiction>,
    pub school_names: Vec<String>,
}

impl Options {
    /// Options for the research-corpus artifact, stamped with an observation date.
    pub fn for_input(input: impl Into<String>, observed_on: impl Into<String>) -> Self {
        Self {
            input: Some(input.into()),
            observed_on: observed_on.into(),
            ..Default::default()
        }
    }
}

/// Import AthleticLIVE meets into the canonical store.
pub async fn collect(ctx: &AdapterContext<'_>, options: &Options) -> CrawlResult<AdapterReport> {
    let Some(input) = options.input.as_deref() else {
        return Err(CrawlError::Invariant {
            detail: "athleticlive adapter requires --input <path to athleticlive-midwest-2026-meets-all.csv>"
                .to_string(),
        });
    };
    let body = std::fs::read_to_string(input).map_err(|source| CrawlError::Io {
        path: PathBuf::from(input),
        source,
    })?;
    let mut report = AdapterReport::new("athleticlive", "meets");
    let parsed = parse_meets_csv(&body)?;
    report.note(format!("rows read: {}", parsed.len()));

    let wanted: BTreeSet<UsJurisdiction> = options.states.iter().copied().collect();
    let filtered: Vec<MeetRow> = parsed
        .into_iter()
        .filter(|row| wanted.is_empty() || wanted.contains(&row.state_code))
        .collect();

    let mut meets = build_meets(&filtered, &options.observed_on, "athleticlive_meets_csv");
    if let Some(limit) = options.limit {
        meets.truncate(limit);
    }

    let done = ctx.store.journal_keys("athleticlive_meets")?;
    let (written, skipped_corrupt) = write_meets(ctx, meets, &done)?;
    note_coverage(&mut report, &filtered, &done, written, skipped_corrupt);
    Ok(report)
}

/// Write the harvest meets that are neither already journaled nor impossibly dated.
///
/// Each meet's row and the entry that journals it reach the store in one commit, flushed every
/// [`FLUSH_UNITS`] meets, so a resume can neither skip a meet whose row is missing nor re-read one
/// the store already holds.
///
/// Returns how many were written and how many were refused for an impossible date.
fn write_meets(
    ctx: &AdapterContext<'_>,
    meets: Vec<CanonicalMeet>,
    done: &HashSet<String>,
) -> CrawlResult<(usize, usize)> {
    let mut written = 0usize;
    let mut skipped_corrupt = 0usize;
    let mut page = ctx.store.write_batch();
    let mut units = 0usize;
    for meet in meets {
        if done.contains(meet.id.as_str()) {
            continue;
        }
        if implausible_year(&meet.date) {
            skipped_corrupt = skipped_corrupt.saturating_add(1);
            continue;
        }
        page.append_many(census_store::Table::Meets, std::slice::from_ref(&meet))?;
        page.journal_done(
            "athleticlive_meets",
            meet.id.as_str(),
            &serde_json::json!({ "state": meet.state, "date": meet.date }),
        )?;
        written = written.saturating_add(1);
        units = units.saturating_add(1);
        if units >= FLUSH_UNITS {
            page.commit()?;
            page = ctx.store.write_batch();
            units = 0;
        }
    }
    page.commit()?;
    Ok((written, skipped_corrupt))
}

/// Record what the run read, kept and refused on the supplied report.
fn note_coverage(
    report: &mut AdapterReport,
    filtered: &[MeetRow],
    done: &HashSet<String>,
    written: usize,
    skipped_corrupt: usize,
) {
    let tenants: BTreeSet<&str> = filtered.iter().map(|r| r.tenant.as_str()).collect();
    let with_an = filtered
        .iter()
        .filter(|r| r.athleticnet_meet_id.is_some())
        .count();
    let corrupt = filtered
        .iter()
        .filter(|r| implausible_year(&r.start))
        .count();
    report.note(format!(
        "rows with implausible years (outside 2015-2030): {corrupt} (refused at write; skipped this run: {skipped_corrupt})"
    ));
    let min_date = filtered.iter().map(|r| r.start.as_str()).min();
    let max_date = filtered.iter().map(|r| r.start.as_str()).max();
    report.rows = u64::try_from(written).unwrap_or(u64::MAX);
    report.note(format!("distinct tenants: {}", tenants.len()));
    report.note(format!(
        "rows with an Athletic.net meet id: {with_an}/{}",
        filtered.len()
    ));
    report.note(format!(
        "meets written: {written} (journal had {})",
        done.len()
    ));
    if let (Some(min), Some(max)) = (min_date, max_date) {
        report.note(format!("date span: {min}..{max}"));
    }
    let observed = EvidenceMethod::Parsed;
    report.note(format!("evidence method: {observed:?}"));
}

#[cfg(test)]
mod tests;
