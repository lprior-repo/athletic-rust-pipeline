use super::{admission, CrawlError, CrawlResult, ManifestOptions, ResultOptions};
use crate::{AdapterReport, UnresolvedCounters};
use std::collections::HashMap;

pub(super) fn select(
    input: &str,
    options: &ManifestOptions,
    entries: Vec<ResultOptions>,
) -> CrawlResult<(Vec<ResultOptions>, Vec<String>, String)> {
    let read = entries.len();
    let mut merged: Vec<ResultOptions> = Vec::new();
    let mut positions = HashMap::new();
    positions.try_reserve(read).map_err(|_| allocation(read))?;
    merged
        .try_reserve_exact(read)
        .map_err(|_| allocation(read))?;
    for entry in entries {
        let target = entry.meet.as_ref().ok_or_else(owner_error)?;
        if let Some(index) = positions.get(&target.athleticlive_meet_id).copied() {
            combine(merged.get_mut(index).ok_or_else(owner_error)?, entry)?;
        } else {
            positions.insert(target.athleticlive_meet_id, merged.len());
            merged.push(entry);
        }
    }
    let duplicates = read.checked_sub(merged.len()).ok_or_else(owner_error)?;
    merged.retain(|entry| {
        entry.meet.as_ref().is_some_and(|target| {
            options.states.is_empty() || options.states.contains(&target.state)
        })
    });
    let in_scope = merged.len();
    let unfinished = withheld(input, options.limit, &mut merged)?;
    let accounting = format!("manifest {input}: {read} entries, {duplicates} repeated owners merged, {in_scope} in scope, {} selected", merged.len());
    Ok((merged, unfinished, accounting))
}

fn combine(existing: &mut ResultOptions, incoming: ResultOptions) -> CrawlResult<()> {
    require_same_owner(existing, &incoming)?;
    if existing.summary.is_some()
        && incoming.summary.is_some()
        && existing.summary != incoming.summary
    {
        return Err(CrawlError::Invariant {
            detail: "manifest owner publishes conflicting summary captures".into(),
        });
    }
    if existing.summary.is_none() {
        existing.summary = incoming.summary;
    }
    existing
        .documents
        .try_reserve(incoming.documents.len())
        .map_err(|_| allocation(incoming.documents.len()))?;
    for path in incoming.documents {
        if !existing.documents.contains(&path) {
            existing.documents.push(path);
        }
    }
    existing
        .standings
        .try_reserve(incoming.standings.len())
        .map_err(|_| allocation(incoming.standings.len()))?;
    for row in incoming.standings {
        if !existing.standings.contains(&row) {
            existing.standings.push(row);
        }
    }
    for (path, metadata) in incoming.capture_metadata {
        if existing
            .capture_metadata
            .get(&path)
            .is_some_and(|known| known != &metadata)
        {
            return Err(CrawlError::Invariant {
                detail: format!("manifest path {path} has conflicting physical metadata"),
            });
        }
        existing.capture_metadata.insert(path, metadata);
    }
    Ok(())
}

fn require_same_owner(existing: &ResultOptions, incoming: &ResultOptions) -> CrawlResult<()> {
    let left = existing.meet.as_ref().ok_or_else(owner_error)?;
    let right = incoming.meet.as_ref().ok_or_else(owner_error)?;
    if (
        left.athleticlive_meet_id,
        &left.tenant,
        &left.name,
        left.state,
        &left.date,
    ) != (
        right.athleticlive_meet_id,
        &right.tenant,
        &right.name,
        right.state,
        &right.date,
    ) {
        return Err(owner_error());
    }
    Ok(())
}

fn withheld(
    input: &str,
    limit: Option<usize>,
    entries: &mut Vec<ResultOptions>,
) -> CrawlResult<Vec<String>> {
    let mut unfinished = Vec::new();
    let first = limit.map_or(entries.len(), |limit| limit.min(entries.len()));
    let tail = entries.get(first..).ok_or_else(owner_error)?;
    unfinished
        .try_reserve_exact(tail.len())
        .map_err(|_| allocation(tail.len()))?;
    tail.iter().try_for_each(|entry| {
        let target = entry.meet.as_ref().ok_or_else(owner_error)?;
        unfinished.push(format!(
            "{input}#meet={}&tenant={}",
            target.athleticlive_meet_id, target.tenant
        ));
        Ok::<_, CrawlError>(())
    })?;
    entries.truncate(first);
    Ok(unfinished)
}

pub(super) fn merge(report: &mut AdapterReport, one: AdapterReport, meet: u64) -> CrawlResult<()> {
    report.rows = report.rows.checked_add(one.rows).ok_or_else(owner_error)?;
    report.requests = report
        .requests
        .checked_add(one.requests)
        .ok_or_else(owner_error)?;
    report.from_cache = report
        .from_cache
        .checked_add(one.from_cache)
        .ok_or_else(owner_error)?;
    report.errors = report
        .errors
        .checked_add(one.errors)
        .ok_or_else(owner_error)?;
    report.rejections = report
        .rejections
        .checked_add(one.rejections)
        .ok_or_else(owner_error)?;
    if let Some(one) = one.unresolved {
        let total = report
            .unresolved
            .get_or_insert(UnresolvedCounters { rows: 0, labels: 0 });
        total.rows = total.rows.checked_add(one.rows).ok_or_else(owner_error)?;
        total.labels = total
            .labels
            .checked_add(one.labels)
            .ok_or_else(owner_error)?;
    }
    for locator in one.unfinished {
        admission::limit(
            "LIVE unfinished locators",
            report.unfinished.len().saturating_add(1),
            admission::MAX_RECORDS,
        )?;
        report
            .unfinished
            .try_reserve(1)
            .map_err(|_| allocation(1))?;
        report.unfinished.push(locator);
    }
    for note in one.notes.into_iter().take(16) {
        if report.notes.len() >= 128 {
            break;
        }
        report.notes.try_reserve(1).map_err(|_| allocation(1))?;
        report.note(format!(
            "meet {meet}: {}",
            note.chars().take(4096).collect::<String>()
        ));
    }
    Ok(())
}

fn owner_error() -> CrawlError {
    CrawlError::Invariant {
        detail: "manifest entries disagree on their published meet owner/context".into(),
    }
}

fn allocation(requested: usize) -> CrawlError {
    admission::resource(
        "LIVE manifest selection allocation",
        requested,
        admission::MAX_RECORDS,
    )
}
