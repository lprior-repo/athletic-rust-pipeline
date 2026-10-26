use super::super::participation_evidence::provider_target::profile_pr;
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const PROFILE_PR_PHASE: &str = "milesplit_profile_prs_v1";
const ADAPTER: &str = "milesplit_profiles";
const MAX_ERROR_NOTES: usize = 50;
const MAX_URL_BYTES: usize = 4096;

#[derive(Debug, Clone)]
pub struct ProfilePrTarget {
    pub athlete_id: String,
    pub profile_url: String,
    pub site: UsJurisdiction,
}

#[derive(Debug)]
pub struct ProfilePrOptions {
    pub targets: Vec<ProfilePrTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfilePrJournalRowV1 {
    version: u8,
    athlete_id: String,
    site: String,
    profile_url: String,
    acquired_at: String,
    body_sha256: String,
    rows: Vec<ProfilePrJournalMarkV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProfilePrJournalMarkV1 {
    event: String,
    mark: String,
    date: String,
    mode: String,
    level: String,
    ordinal: u32,
    byte_offset: usize,
    byte_length: usize,
}

impl ProfilePrJournalRowV1 {
    pub fn version(&self) -> u8 {
        self.version
    }

    pub fn athlete_id(&self) -> &str {
        &self.athlete_id
    }

    pub fn site(&self) -> &str {
        &self.site
    }

    pub fn profile_url(&self) -> &str {
        &self.profile_url
    }

    pub fn acquired_at(&self) -> &str {
        &self.acquired_at
    }

    pub fn body_sha256(&self) -> &str {
        &self.body_sha256
    }

    pub fn rows(&self) -> &[ProfilePrJournalMarkV1] {
        &self.rows
    }
}

impl ProfilePrJournalMarkV1 {
    pub fn event(&self) -> &str {
        &self.event
    }

    pub fn mark(&self) -> &str {
        &self.mark
    }

    pub fn date(&self) -> &str {
        &self.date
    }

    pub fn mode(&self) -> &str {
        &self.mode
    }

    pub fn level(&self) -> &str {
        &self.level
    }

    pub fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub fn byte_offset(&self) -> usize {
        self.byte_offset
    }

    pub fn byte_length(&self) -> usize {
        self.byte_length
    }
}

#[derive(Debug, Default)]
struct Counts {
    captured: u64,
    current: u64,
    not_found: u64,
    no_table: u64,
    failed: u64,
}

#[tracing::instrument(skip(ctx, options))]
pub async fn collect(
    ctx: &AdapterContext<'_>,
    options: &ProfilePrOptions,
) -> CrawlResult<AdapterReport> {
    let mut report = AdapterReport::new(ADAPTER, "profile PR rows");
    if options.targets.is_empty() {
        report.note("no profile targets supplied; nothing was requested".to_string());
        return Ok(report);
    }
    let lanes = ctx.fetcher.family_parallelism().max(1);
    let outcomes = futures::stream::iter(options.targets.iter())
        .map(|target| harvest(ctx, target))
        .buffer_unordered(lanes)
        .collect::<Vec<_>>()
        .await;
    let counts = outcomes.iter().fold(Counts::default(), |counts, outcome| {
        accumulate(counts, outcome)
    });
    let notes = outcomes.iter().filter_map(outcome_note);
    let overflow = counts.failed.saturating_sub(MAX_ERROR_NOTES as u64) > 0;
    report.rows = u64::try_from(options.targets.len()).map_err(|_| resource("target count"))?;
    let stats = ctx.fetcher.stats().await;
    report.requests = stats.physical_requests();
    report.from_cache = stats.cache_hits;
    report.errors = counts.failed;
    report.note(format!(
        "captured={} current={} not_found={} no_table={} failed={}",
        counts.captured, counts.current, counts.not_found, counts.no_table, counts.failed
    ));
    for note in notes.take(MAX_ERROR_NOTES) {
        report.note(note);
    }
    if overflow {
        report.note("additional target failures omitted".to_string());
    }
    report.finish_frontier();
    Ok(report)
}

fn accumulate(mut counts: Counts, outcome: &CrawlResult<Harvest>) -> Counts {
    match outcome {
        Ok(Harvest::Captured) => counts.captured = counts.captured.saturating_add(1),
        Ok(Harvest::Current) => counts.current = counts.current.saturating_add(1),
        Ok(Harvest::NotFound) => counts.not_found = counts.not_found.saturating_add(1),
        Ok(Harvest::NoTable) => counts.no_table = counts.no_table.saturating_add(1),
        Err(_) => counts.failed = counts.failed.saturating_add(1),
    }
    counts
}

fn outcome_note(outcome: &CrawlResult<Harvest>) -> Option<String> {
    match outcome {
        Err(error) => Some(truncate(format!("profile target failed: {error}"))),
        _ => None,
    }
}

fn truncate(note: String) -> String {
    const LIMIT: usize = 512;
    if note.len() <= LIMIT {
        return note;
    }
    match note.char_indices().nth(LIMIT) {
        Some((index, _)) => format!("{}…", &note[..index]),
        None => note,
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Harvest {
    Captured,
    Current,
    NotFound,
    NoTable,
}

#[tracing::instrument(skip(ctx), fields(athlete = target.athlete_id.as_str()))]
async fn harvest(ctx: &AdapterContext<'_>, target: &ProfilePrTarget) -> CrawlResult<Harvest> {
    if target.profile_url.len() > MAX_URL_BYTES {
        return Err(resource("profile URL length"));
    }
    let key = journal_key(target);
    if !ctx.refresh && ctx.store.journal_contains(PROFILE_PR_PHASE, &key)? {
        return Ok(Harvest::Current);
    }
    let outcome = ctx
        .fetcher
        .get(
            &target.profile_url,
            &FetchOptions {
                refresh: ctx.refresh,
                allow_not_found: true,
                headers: Vec::new(),
            },
        )
        .await?;
    match outcome.status {
        404 => Ok(Harvest::NotFound),
        200 => commit_table(ctx, target, &key, &outcome).await,
        status => Err(crate::CrawlError::Fetch(crate::net::FetchError::Http {
            status,
            url: target.profile_url.clone(),
        })),
    }
}

async fn commit_table(
    ctx: &AdapterContext<'_>,
    target: &ProfilePrTarget,
    key: &str,
    outcome: &crate::net::FetchOutcome,
) -> CrawlResult<Harvest> {
    let body = std::str::from_utf8(&outcome.body).map_err(|_| resource("profile body encoding"))?;
    let parsed =
        profile_pr::read_source_rows(body, &target.profile_url, target.site, &target.athlete_id)?;
    let Some(rows) = parsed else {
        return Ok(Harvest::NoTable);
    };
    let row = journal_row(target, &outcome.fetched_at, &outcome.content_digest, rows)?;
    let Some(row) = row else {
        return Ok(Harvest::NoTable);
    };
    let existed = ctx.store.journal_payload(PROFILE_PR_PHASE, key)?.is_some();
    let mut batch = ctx.write_batch();
    batch.journal_done(PROFILE_PR_PHASE, key, &row)?;
    batch.commit()?;
    if existed {
        return Ok(Harvest::Current);
    }
    Ok(Harvest::Captured)
}

fn journal_row(
    target: &ProfilePrTarget,
    fetched_at: &str,
    content_digest: &str,
    parsed: Vec<profile_pr::SourceProfilePrV1>,
) -> CrawlResult<Option<ProfilePrJournalRowV1>> {
    if parsed.is_empty() {
        return Ok(None);
    }
    let row = ProfilePrJournalRowV1 {
        version: 1,
        athlete_id: target.athlete_id.clone(),
        site: target.site.code().to_string(),
        profile_url: target.profile_url.clone(),
        acquired_at: fetched_at.to_owned(),
        body_sha256: content_digest.to_owned(),
        rows: parsed
            .iter()
            .map(|mark| {
                let locator = mark.locator();
                ProfilePrJournalMarkV1 {
                    event: mark.event().to_owned(),
                    mark: mark.published_mark().to_owned(),
                    date: mark.published_date().to_owned(),
                    mode: mark.source_mode().to_owned(),
                    level: mark.source_level().to_owned(),
                    ordinal: locator.ordinal(),
                    byte_offset: locator.byte_offset(),
                    byte_length: locator.byte_length(),
                }
            })
            .collect(),
    };
    validate(&row)?;
    Ok(Some(row))
}

fn validate(row: &ProfilePrJournalRowV1) -> CrawlResult<()> {
    if row.version != 1
        || row.athlete_id.is_empty()
        || row.site.is_empty()
        || row.profile_url.is_empty()
        || row.body_sha256.is_empty()
        || row.rows.is_empty()
    {
        return Err(resource("profile PR journal shape"));
    }
    chrono::DateTime::parse_from_rfc3339(&row.acquired_at)
        .map_err(|_| resource("profile PR acquired timestamp"))?;
    if row
        .rows
        .iter()
        .any(|mark| mark.event.is_empty() || mark.mark.is_empty() || mark.date.is_empty())
    {
        return Err(resource("profile PR journal mark"));
    }
    Ok(())
}

fn journal_key(target: &ProfilePrTarget) -> String {
    let seed = format!("{}:{}", target.site.code(), target.athlete_id);
    let digest = format!("{:x}", Sha256::digest(seed.as_bytes()));
    let shard: String = digest.chars().take(2).collect();
    format!("{shard}:{}:{}", target.site.code(), target.athlete_id)
}

fn resource(detail: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: detail.to_string(),
    }
}

#[cfg(test)]
mod tests;
