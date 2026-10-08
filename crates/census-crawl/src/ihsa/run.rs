use super::parse::{parse_staff, SchoolRecord, StaffPerson};
use super::Options;
use crate::net::{now_iso8601, FetchError};
use crate::{AdapterContext, AdapterReport, CrawlResult};
use census_domain::model::CanonicalCoach;
use census_store::Table;
use std::collections::HashMap;

pub(super) const IHSA_HOST: &str = "api.ihsa.org";

pub(super) struct IhsaRun<'a> {
    pub(super) options: &'a Options,
    pub(super) revealed_emails: HashMap<i64, Option<String>>,
    pub(super) processed: usize,
    pub(super) skipped: usize,
    pub(super) deferred: usize,
    pub(super) blocked: usize,
}

pub(super) async fn retry_later(ctx: &AdapterContext<'_>, error: &FetchError) -> bool {
    if error.retryable() || matches!(error, FetchError::Offline { .. }) {
        return true;
    }
    match error {
        FetchError::Policy { .. } => ctx.fetcher.host_blocked(IHSA_HOST, &now_iso8601()).await,
        _ => false,
    }
}

pub(super) fn journal_school(
    ctx: &AdapterContext<'_>,
    journal_key: &str,
    details: &serde_json::Value,
    coaches: &[CanonicalCoach],
) -> CrawlResult<()> {
    let mut batch = ctx.store.write_batch();
    batch.append_many(Table::Coaches, coaches)?;
    batch.journal_done("ihsa_schools", journal_key, details)?;
    batch.commit()?;
    Ok(())
}

fn close_school(
    ctx: &AdapterContext<'_>,
    journal_key: &str,
    run: &mut IhsaRun<'_>,
    details: serde_json::Value,
) -> CrawlResult<()> {
    journal_school(ctx, journal_key, &details, &[])?;
    run.processed = run.processed.saturating_add(1);
    Ok(())
}

struct JournalFailure {
    kind: &'static str,
    message: String,
    detail: String,
}

fn close_school_failure(
    ctx: &AdapterContext<'_>,
    journal_key: &str,
    record: &SchoolRecord,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
    failure: JournalFailure,
) -> CrawlResult<()> {
    report.errors = report.errors.saturating_add(1);
    report.note(failure.message);
    let mut details = serde_json::Map::new();
    details.insert("school_id".to_string(), serde_json::json!(record.school_id));
    details.insert("name".to_string(), serde_json::json!(record.name_formal));
    details.insert(failure.kind.to_string(), serde_json::json!(failure.detail));
    close_school(ctx, journal_key, run, serde_json::Value::Object(details))
}

pub(super) fn note_fetch(report: &mut AdapterReport, from_cache: bool) {
    if from_cache {
        report.from_cache = report.from_cache.saturating_add(1);
    } else {
        report.requests = report.requests.saturating_add(1);
    }
}

fn staff_fetch_unreachable(
    record: &SchoolRecord,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
    error: &FetchError,
) {
    report.errors = report.errors.saturating_add(1);
    report.note(format!(
        "failed to fetch staff for school {}: {error}",
        record.school_id
    ));
    run.deferred = run.deferred.saturating_add(1);
    report.note(format!(
        "school {} left open: the staff fetch never reached the source",
        record.school_id
    ));
}

pub(super) async fn fetch_staff(
    ctx: &AdapterContext<'_>,
    staff_url: &str,
    record: &SchoolRecord,
    journal_key: &str,
    run: &mut IhsaRun<'_>,
    report: &mut AdapterReport,
) -> CrawlResult<Option<Vec<StaffPerson>>> {
    let staff_outcome = match ctx.fetcher.get(staff_url, &ctx.fetch_options()).await {
        Ok(outcome) => outcome,
        Err(error) => {
            if retry_later(ctx, &error).await {
                staff_fetch_unreachable(record, run, report, &error);
                return Ok(None);
            }
            close_school_failure(
                ctx,
                journal_key,
                record,
                run,
                report,
                JournalFailure {
                    kind: "error",
                    message: format!(
                        "failed to fetch staff for school {}: {error}",
                        record.school_id
                    ),
                    detail: error.to_string(),
                },
            )?;
            return Ok(None);
        }
    };
    note_fetch(report, staff_outcome.from_cache);

    let staff = match parse_staff(&staff_outcome.text()) {
        Ok(env) => env,
        Err(error) => {
            close_school_failure(
                ctx,
                journal_key,
                record,
                run,
                report,
                JournalFailure {
                    kind: "parse_error",
                    message: format!(
                        "failed to parse staff for school {}: {error}",
                        record.school_id
                    ),
                    detail: error.to_string(),
                },
            )?;
            return Ok(None);
        }
    };
    Ok(Some(staff))
}
