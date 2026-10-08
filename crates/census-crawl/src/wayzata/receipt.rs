use super::budget;
use super::projection::Page;
use super::{MeetRow, Options, PARSE_VERSION};
use crate::{AdapterContext, CrawlError, CrawlResult};
use census_domain::model::{serialized_digest, CanonicalMeet};
use census_domain::UsJurisdiction;
use census_store::Table;
use serde::Serialize;

pub(super) const RAW_PHASE: &str = "wayzata_schedule_capture_v4";
const PROJECTION_PHASE: &str = "wayzata_schedule_projection_v4";

#[derive(Serialize)]
struct Capture<'a> {
    url: &'a str,
    sha256: &'a str,
    fetched_at: &'a str,
    sport: &'a str,
    year: i16,
    ordinal: usize,
    producer_revision: u32,
}

impl<'a> Capture<'a> {
    fn new(page: &'a Page<'_>, ordinal: usize) -> Self {
        Self {
            url: &page.fetched.url,
            sha256: &page.fetched.content_digest,
            fetched_at: &page.fetched.fetched_at,
            sport: page.sport.as_str(),
            year: page.year,
            ordinal,
            producer_revision: PARSE_VERSION,
        }
    }
}

#[derive(Serialize)]
struct Projection<'a> {
    capture: Capture<'a>,
    as_of: chrono::NaiveDate,
    jurisdictions: Vec<UsJurisdiction>,
}

pub(super) struct Effect {
    pub operation: String,
    pub digest: String,
}

fn effect(phase: &str, payload: &impl Serialize) -> CrawlResult<Effect> {
    let digest = serialized_digest(payload).map_err(|source| CrawlError::Canonical {
        table: phase.to_string(),
        source,
    })?;
    Ok(Effect {
        operation: format!("{phase}/{digest}"),
        digest,
    })
}

pub(super) fn projection_effect(
    ctx: &AdapterContext<'_>,
    options: &Options,
    page: &Page<'_>,
    ordinal: usize,
) -> CrawlResult<Effect> {
    let mut jurisdictions = Vec::new();
    budget::reserve(
        &mut jurisdictions,
        options.jurisdictions.len(),
        UsJurisdiction::ALL.len(),
    )?;
    jurisdictions.extend(
        UsJurisdiction::ALL
            .into_iter()
            .filter(|state| options.jurisdictions.contains(state)),
    );
    effect(
        PROJECTION_PHASE,
        &Projection {
            capture: Capture::new(page, ordinal),
            as_of: ctx.performance_as_of,
            jurisdictions,
        },
    )
}

#[derive(Serialize)]
struct RawRow<'a> {
    capture: Capture<'a>,
    date: &'a str,
    name: &'a str,
    venue: &'a str,
    slug: Option<&'a str>,
    aria_label: Option<&'a str>,
}

pub(super) fn retain_raw(
    ctx: &AdapterContext<'_>,
    page: &Page<'_>,
    ordinal: usize,
    row: &MeetRow,
) -> CrawlResult<()> {
    let payload = RawRow {
        capture: Capture::new(page, ordinal),
        date: &row.date,
        name: &row.name,
        venue: &row.location,
        slug: row.slug.as_deref(),
        aria_label: row.aria_label.as_deref(),
    };
    let effect = effect(RAW_PHASE, &payload)?;
    if ctx.effect_is_committed(&effect.operation, &effect.digest)? {
        return Ok(());
    }
    let mut batch = ctx.write_batch();
    batch.journal_done(RAW_PHASE, &effect.digest, &payload)?;
    batch.commit_once(&effect.operation, &effect.digest)?;
    Ok(())
}

pub(super) fn commit_projection(
    ctx: &AdapterContext<'_>,
    effect: &Effect,
    meet: Option<&CanonicalMeet>,
) -> CrawlResult<bool> {
    let mut batch = ctx.write_batch();
    if let Some(meet) = meet {
        batch.append_many(Table::Meets, std::slice::from_ref(meet))?;
    }
    batch.journal_done(PROJECTION_PHASE, &effect.digest, &effect.digest)?;
    Ok(batch
        .commit_once(&effect.operation, &effect.digest)?
        .written())
}
