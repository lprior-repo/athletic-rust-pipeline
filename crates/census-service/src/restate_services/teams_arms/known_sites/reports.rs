use crate::restate_services::JobError;
use census_crawl::{AdapterReport, CollectionDisposition, UnresolvedCounters};

const MAX_LOCATORS: usize = 65_536;
const MAX_FIELD_BYTES: usize = 4096;

pub(super) fn merge(report: &mut AdapterReport, page: AdapterReport) -> Result<(), JobError> {
    if !page.disposition.is_complete() && page.unfinished.is_empty() {
        owe(
            report,
            format!("{}:unmeasured-contact-frontier", page.adapter),
        )?;
    }
    for (target, incoming) in [
        (&mut report.rows, page.rows),
        (&mut report.requests, page.requests),
        (&mut report.from_cache, page.from_cache),
        (&mut report.errors, page.errors),
        (&mut report.with_email, page.with_email),
        (&mut report.rejections, page.rejections),
    ] {
        *target = target.checked_add(incoming).ok_or_else(capacity)?;
    }
    if let Some(value) = page.unresolved {
        let old = report
            .unresolved
            .get_or_insert(UnresolvedCounters { rows: 0, labels: 0 });
        old.rows = old.rows.checked_add(value.rows).ok_or_else(capacity)?;
        old.labels = old.labels.checked_add(value.labels).ok_or_else(capacity)?;
    }
    page.notes
        .into_iter()
        .try_for_each(|note| append(&mut report.notes, note))?;
    page.unfinished
        .into_iter()
        .try_for_each(|locator| owe(report, locator))?;
    report.finish_frontier();
    Ok(())
}

pub(super) fn owe(report: &mut AdapterReport, locator: String) -> Result<(), JobError> {
    append(&mut report.unfinished, locator)?;
    report.disposition = CollectionDisposition::Partial;
    Ok(())
}

fn append(rows: &mut Vec<String>, value: String) -> Result<(), JobError> {
    if rows.len() >= MAX_LOCATORS || value.len() > MAX_FIELD_BYTES {
        return Err(capacity());
    }
    rows.try_reserve(1).map_err(|_| capacity())?;
    rows.push(value);
    Ok(())
}

fn capacity() -> JobError {
    JobError::Terminal {
        message: "contact source report capacity or counter exhausted; work remains owed"
            .to_string(),
    }
}
