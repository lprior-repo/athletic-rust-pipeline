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

use crate::sources::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::model::{CanonicalMeet, EvidenceMethod};

mod meets;
mod parse;

pub use meets::build_meets;
pub use parse::{implausible_year, infer_level, parse_meets_csv, state_code, MeetRow};

/// Adapter options (uniform across provider adapters plus `input`).
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// Path to `athleticlive-midwest-2026-meets-all.csv` (or the `-seeds` variant).
    pub input: Option<String>,
    pub limit: Option<usize>,
    pub refresh: bool,
    pub observed_on: String,
    /// Restrict to these state codes; empty = every state present in the file.
    pub states: Vec<String>,
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

    let wanted: BTreeSet<String> = options
        .states
        .iter()
        .filter_map(|s| state_code(s).map(str::to_string))
        .collect();
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
/// Returns how many were written and how many were refused for an impossible date.
fn write_meets(
    ctx: &AdapterContext<'_>,
    meets: Vec<CanonicalMeet>,
    done: &HashSet<String>,
) -> CrawlResult<(usize, usize)> {
    let mut written = 0usize;
    let mut skipped_corrupt = 0usize;
    for meet in meets {
        if done.contains(meet.id.as_str()) {
            continue;
        }
        // Placeholder rows in the tenant index carry impossible dates (`2222-08-23`). A meet is
        // the anchor for the school year a grade is read against, so an impossible date would
        // mint an impossible graduating class downstream: refuse it here instead.
        if implausible_year(&meet.date) {
            skipped_corrupt += 1;
            continue;
        }
        ctx.store.append(crate::store::Table::Meets, &meet)?;
        ctx.store.journal_done(
            "athleticlive_meets",
            meet.id.as_str(),
            &serde_json::json!({ "state": meet.state, "date": meet.date }),
        )?;
        written += 1;
    }
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
    report.rows = written as u64;
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
