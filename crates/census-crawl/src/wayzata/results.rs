use super::budget;
use super::projection::Page;
use super::{MeetRow, Options, PROVIDER};
use crate::athleticlive::collect_results;
use crate::net::FetchOutcome;
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::CanonicalMeet;
use serde_json::{json, Value};

mod captures;
mod tenant;
use captures::CapturedResults;
use tenant::ResolvedMeet;

pub(super) const RESULTS_HOST: &str = "results.wayzatatiming.com";

pub(super) const LANE_PHASE: &str = "wayzata_results_ledger_v1";

const SEARCH_ORIGIN: &str = "https://search.athletic.live";

const SEARCH_INDEX: &str = "wayzata_meet_list";

const TENANT: &str = PROVIDER;

const REVISION: u32 = 1;

pub(super) enum Lane {
    Off(String),
    On,
}

impl Lane {
    pub(super) fn of(ctx: &AdapterContext<'_>) -> Self {
        if ctx.fetcher.is_authorized_host(RESULTS_HOST) {
            Self::On
        } else {
            Self::Off(format!(
                "results lane off: {RESULTS_HOST} carries no operator grant"
            ))
        }
    }

    pub(super) fn dispatch(&self) -> bool {
        matches!(self, Self::On)
    }

    pub(super) fn off_reason(&self) -> Option<&str> {
        match self {
            Self::Off(reason) => Some(reason.as_str()),
            Self::On => None,
        }
    }
}

#[derive(Default)]
pub(super) struct Totals {
    meets: u64,
    documents: u64,
    performances: u64,
    owed: u64,
    errors: u64,
}

impl Totals {
    pub(super) fn record(&mut self, harvest: &Harvest) -> CrawlResult<()> {
        let owed = u64::try_from(harvest.owed.len())
            .map_err(|_| budget::arithmetic("Wayzata owed result documents"))?;
        self.meets = budget::add(self.meets, 1)?;
        self.documents = budget::add(self.documents, harvest.documents)?;
        self.performances = budget::add(self.performances, harvest.performances)?;
        self.owed = budget::add(self.owed, owed)?;
        self.errors = budget::add(self.errors, harvest.errors)?;
        Ok(())
    }

    pub(super) fn note(&self) -> String {
        format!(
            "results lane: {} meets, {} event documents, {} performances projected, {} owed, {} refused",
            self.meets, self.documents, self.performances, self.owed, self.errors
        )
    }
}

pub(super) struct Harvest {
    pub(super) documents: u64,
    pub(super) performances: u64,
    pub(super) errors: u64,
    pub(super) owed: Vec<String>,
    pub(super) refusal: Option<String>,
}

pub(super) fn search_url() -> String {
    format!("{SEARCH_ORIGIN}/{SEARCH_INDEX}/_search")
}

pub(super) fn search_query(meet_id: u64) -> Value {
    json!({"query": {"term": {"i": meet_id}}})
}

pub(super) async fn harvest(
    ctx: &AdapterContext<'_>,
    options: &Options,
    page: &Page<'_>,
    row: &MeetRow,
    meet: &CanonicalMeet,
) -> CrawlResult<Harvest> {
    let slug = row.slug.as_deref().ok_or_else(|| {
        schema(
            &page.fetched.url,
            "published meet row carries no provider link",
        )
    })?;
    let state = meet
        .state
        .ok_or_else(|| schema(&page.fetched.url, "projected meet carries no jurisdiction"))?;
    let mut fetch_options = ctx.fetch_options();
    fetch_options.refresh = ctx.refresh || options.refresh;
    let resolved = ResolvedMeet::fetch(ctx, &fetch_options, slug).await?;
    resolved.validate_date(meet)?;
    let mut captures = CapturedResults::fetch(ctx, &fetch_options, resolved.id).await?;
    let (result_options, documents) = captures.prepare(ctx, options, meet, state, resolved.id)?;
    let report = collect_results(ctx, &result_options).await?;
    let refusal = report
        .notes
        .iter()
        .find(|note| note.starts_with("capture refused:"))
        .cloned();
    let ledger = captures::LedgerInputs {
        slug,
        meet,
        state,
        resolved: &resolved,
        documents: &documents,
    };
    let identity = captures.identity(&ledger);
    let payload = captures.payload(page, &ledger, &report);
    retain(ctx, &identity, &payload)?;
    Ok(Harvest {
        documents: u64::try_from(captures.cached.len())
            .map_err(|_| budget::arithmetic("Wayzata result documents"))?,
        performances: report.rows,
        errors: report.errors,
        owed: report.unfinished,
        refusal,
    })
}

fn retain(ctx: &AdapterContext<'_>, identity: &Value, payload: &Value) -> CrawlResult<()> {
    let digest = census_domain::model::serialized_digest(identity).map_err(|source| {
        CrawlError::Canonical {
            table: LANE_PHASE.to_string(),
            source,
        }
    })?;
    let operation = format!("{LANE_PHASE}/{digest}");
    if ctx.effect_is_committed(&operation, &digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.journal_done(LANE_PHASE, &digest, payload)?;
    batch.commit_once(&operation, &digest)?;
    Ok(())
}

fn text_of<'a>(fetched: &'a FetchOutcome, url: &str) -> CrawlResult<&'a str> {
    std::str::from_utf8(&fetched.body).map_err(|error| {
        schema(
            url,
            &format!("the published document is not UTF-8: {error}"),
        )
    })
}

fn schema(url: &str, detail: &str) -> CrawlError {
    CrawlError::Schema {
        url: url.to_string(),
        detail: detail.to_string(),
    }
}
