use census_crawl::milesplit::TeamRef;
use census_crawl::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;
use census_store::Store;

const PHASE: &str = "milesplit_roster_inputs_v1";
const MAX_FIELD_BYTES: usize = 4096;
const MAX_WINDOW_BYTES: usize = 8 * 1024 * 1024;

pub(super) fn persist(
    store: &Store,
    jurisdiction: UsJurisdiction,
    teams: &[TeamRef],
) -> CrawlResult<()> {
    teams.chunks(64).try_for_each(|window| {
        let bytes = window
            .iter()
            .try_fold(0usize, |bytes, team| add(bytes, measure(team)?))?;
        if bytes > MAX_WINDOW_BYTES {
            return Err(resource(bytes, MAX_WINDOW_BYTES));
        }
        let mut batch = store.write_batch();
        window
            .iter()
            .try_for_each(|team| batch.journal_done(PHASE, &key(jurisdiction, &team.id), team))?;
        batch.commit()?;
        Ok(())
    })
}

pub(super) fn load(
    store: &Store,
    jurisdiction: UsJurisdiction,
    id: &str,
) -> CrawlResult<Option<TeamRef>> {
    let Some(payload) = store.journal_payload(PHASE, &key(jurisdiction, id))? else {
        return Ok(None);
    };
    let team: TeamRef =
        serde_json::from_value(payload).map_err(|source| CrawlError::Canonical {
            table: PHASE.to_string(),
            source: census_domain::model::CanonicalJsonError::Unsupported(source.to_string()),
        })?;
    measure(&team)?;
    if team.id != id {
        return Err(CrawlError::Invariant {
            detail: format!("retained roster input identity differs from {id}"),
        });
    }
    Ok(Some(team))
}

fn measure(team: &TeamRef) -> CrawlResult<usize> {
    [
        &team.id,
        &team.slug,
        &team.url,
        &team.name,
        &team.city_state,
    ]
    .into_iter()
    .try_fold(256usize, |bytes, field| {
        if field.len() > MAX_FIELD_BYTES {
            return Err(resource(field.len(), MAX_FIELD_BYTES));
        }
        let escaped = field.len().checked_mul(6).ok_or_else(arithmetic)?;
        add(bytes, escaped)
    })
}

fn key(jurisdiction: UsJurisdiction, id: &str) -> String {
    format!("{}:{id}", jurisdiction.code())
}
fn add(left: usize, right: usize) -> CrawlResult<usize> {
    left.checked_add(right).ok_or_else(arithmetic)
}
fn arithmetic() -> CrawlError {
    CrawlError::Arithmetic {
        detail: "retained roster input byte count overflow".to_string(),
    }
}
fn resource(requested: usize, limit: usize) -> CrawlError {
    CrawlError::Resource {
        resource: "retained roster input bytes",
        requested,
        limit,
    }
}
