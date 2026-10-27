use super::super::participation_evidence::provider_target::profile_pr;
use crate::net::FetchOptions;
use crate::{AdapterContext, AdapterReport, CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
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
    let width = ctx.fetcher.family_parallelism().max(1);
    let outcomes = std::thread::scope(|scope| {
        let handle = tokio::runtime::Handle::current();
        let chunk = options.targets.len().div_ceil(width).max(1);
        let mut workers = Vec::new();
        for targets in options.targets.chunks(chunk) {
            let handle = handle.clone();
            workers.push(scope.spawn(move || {
                let mut driver = BatchDriver::new(ctx, &handle, targets.len());
                for target in targets {
                    driver.stage(target);
                }
                driver.finish()
            }));
        }
        let mut outcomes = Vec::with_capacity(options.targets.len());
        for worker in workers {
            outcomes.extend(
                worker
                    .join()
                    .map_err(|_| resource("profile harvest thread"))?,
            );
        }
        Ok::<Vec<CrawlResult<Harvest>>, CrawlError>(outcomes)
    })?;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Harvest {
    Captured,
    Current,
    NotFound,
    NoTable,
}

const JOURNAL_BATCH_ROWS: usize = 128;

struct BatchDriver<'ctx> {
    ctx: &'ctx AdapterContext<'ctx>,
    handle: tokio::runtime::Handle,
    batch: crate::recording::RowBatch<'ctx>,
    pending: Vec<Harvest>,
    outcomes: Vec<CrawlResult<Harvest>>,
}

impl<'ctx> BatchDriver<'ctx> {
    fn new(
        ctx: &'ctx AdapterContext<'ctx>,
        handle: &tokio::runtime::Handle,
        capacity: usize,
    ) -> Self {
        Self {
            ctx,
            handle: handle.clone(),
            batch: ctx.write_batch(),
            pending: Vec::new(),
            outcomes: Vec::with_capacity(capacity),
        }
    }

    fn stage(&mut self, target: &ProfilePrTarget) {
        let staged = self
            .handle
            .block_on(stage_table(self.ctx, &mut self.batch, target));
        match staged {
            Ok((harvest, true)) => {
                self.outcomes.push(Ok(harvest));
                self.pending.push(harvest);
                if self.pending.len() >= JOURNAL_BATCH_ROWS {
                    self.flush();
                }
            }
            Ok((harvest, false)) => self.outcomes.push(Ok(harvest)),
            Err(error) => self.outcomes.push(Err(error)),
        }
    }

    fn flush(&mut self) {
        if self.pending.is_empty() {
            return;
        }
        let pending = std::mem::take(&mut self.pending);
        let committed = std::mem::replace(&mut self.batch, self.ctx.write_batch())
            .commit()
            .is_ok();
        if !committed {
            for _ in pending {
                self.outcomes.push(Err(resource("profile journal batch")));
            }
        }
    }

    fn finish(mut self) -> Vec<CrawlResult<Harvest>> {
        self.flush();
        self.outcomes
    }
}

#[tracing::instrument(skip(ctx, batch), fields(athlete = target.athlete_id.as_str()))]
async fn stage_table(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    target: &ProfilePrTarget,
) -> CrawlResult<(Harvest, bool)> {
    if target.profile_url.len() > MAX_URL_BYTES {
        return Err(resource("profile URL length"));
    }
    let key = journal_key(target);
    if !ctx.refresh && ctx.store.journal_contains(PROFILE_PR_PHASE, &key)? {
        return Ok((Harvest::Current, false));
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
        404 => Ok((Harvest::NotFound, false)),
        200 => stage_row(ctx, batch, target, &key, &outcome).await,
        status => Err(crate::CrawlError::Fetch(crate::net::FetchError::Http {
            status,
            url: target.profile_url.clone(),
        })),
    }
}

async fn stage_row(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    target: &ProfilePrTarget,
    key: &str,
    outcome: &crate::net::FetchOutcome,
) -> CrawlResult<(Harvest, bool)> {
    let body = std::str::from_utf8(&outcome.body).map_err(|_| resource("profile body encoding"))?;
    let parsed =
        profile_pr::read_source_rows(body, &target.profile_url, target.site, &target.athlete_id)?;
    let Some(rows) = parsed else {
        return Ok((Harvest::NoTable, false));
    };
    let row = journal_row(target, &outcome.fetched_at, &outcome.content_digest, rows)?;
    let Some(row) = row else {
        return Ok((Harvest::NoTable, false));
    };
    let existed = ctx.store.journal_payload(PROFILE_PR_PHASE, key)?.is_some();
    batch.journal_done(PROFILE_PR_PHASE, key, &row)?;
    if existed {
        return Ok((Harvest::Current, true));
    }
    Ok((Harvest::Captured, true))
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
