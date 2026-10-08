use census_crawl::milesplit::TeamRef;
use census_domain::model::SchoolYear;
use census_store::{Store, StoreResult};

use super::rosters_phase;
use census_domain::UsJurisdiction;

pub(super) fn pending_rosters(
    store: &Store,
    teams: &[TeamRef],
    jurisdiction: UsJurisdiction,
    school_year: SchoolYear,
    revision: std::num::NonZeroU32,
) -> StoreResult<Vec<TeamRef>> {
    if teams.len() > 20_000 {
        return Err(census_store::StoreError::TooManyRows {
            table: "configured roster inputs".to_string(),
            max: 20_000,
        });
    }
    let bytes = teams.iter().try_fold(0usize, |total, team| {
        [
            &team.id,
            &team.slug,
            &team.url,
            &team.name,
            &team.city_state,
        ]
        .iter()
        .try_fold(
            total
                .checked_add(std::mem::size_of::<TeamRef>())
                .ok_or(census_store::StoreError::CounterOverflow)?,
            |total, value| {
                total
                    .checked_add(value.len())
                    .ok_or(census_store::StoreError::CounterOverflow)
            },
        )
    })?;
    if bytes > 4 * 1024 * 1024 {
        return Err(census_store::StoreError::Refused {
            detail: format!(
                "configured roster input bytes {bytes} exceed {}",
                4 * 1024 * 1024
            ),
        });
    }
    let phase = rosters_phase(jurisdiction, school_year, revision);
    teams.iter().try_fold(Vec::new(), |mut pending, team| {
        let key = format!("{}:{}", jurisdiction.code(), team.id);
        if !terminal(store, &phase, &key, (team, school_year))? {
            pending
                .try_reserve(1)
                .map_err(|_| census_store::StoreError::Refused {
                    detail: "allocating bounded pending roster inputs".to_string(),
                })?;
            pending.push(team.clone());
        }
        Ok(pending)
    })
}

fn terminal(
    store: &Store,
    phase: &str,
    key: &str,
    scope: (&TeamRef, SchoolYear),
) -> StoreResult<bool> {
    let Some(payload) = store.journal_payload(phase, key)? else {
        return Ok(false);
    };
    let row: super::sweep::roster::journal::Journal =
        serde_json::from_value(payload).map_err(|source| census_store::StoreError::Json {
            detail: format!("reading configured roster completion from {phase}/{key}"),
            source,
        })?;
    Ok(row.is_terminal_for(&scope.0.id, scope.1))
}
