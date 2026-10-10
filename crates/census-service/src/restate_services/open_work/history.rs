use super::{complete, count};
use crate::restate_services::{meets_arms, results_arms, wire::JurisdictionState};
use census_crawl::{registry, CollectionDisposition as Disposition, ResolutionCounters};
use restate_sdk::prelude::HandlerError;

#[derive(Clone, Copy)]
pub(super) enum Kind {
    Meets,
    Results,
}

impl Kind {
    pub(super) fn name(self) -> &'static str {
        match self {
            Self::Meets => "meets",
            Self::Results => "results",
        }
    }
    pub(super) fn requires(self, slug: &str) -> bool {
        registry::descriptor(slug).is_none_or(|source| match self {
            Self::Meets => {
                source.capabilities.meet_discovery || meets_arms::arm_for(slug).is_some()
            }
            Self::Results => {
                source.capabilities.bulk_results || results_arms::arm_for(slug).is_some()
            }
        })
    }
}

pub(super) fn status(state: &JurisdictionState, kind: Kind) -> Result<Disposition, HandlerError> {
    let (Some(window), Some(plan)) = (state.history_window, state.plan.as_ref()) else {
        return Ok(Disposition::Unknown);
    };
    let required = plan.sweepable.iter().filter(|slug| kind.requires(slug));
    if required.clone().next().is_none() {
        return Ok(Disposition::Unknown);
    }
    window
        .years()
        .try_fold(true, |done, year| {
            let closed =
                required
                    .clone()
                    .try_fold(stage_complete(state, year, kind), |closed, slug| {
                        let disposition = source(state, year, slug, kind)?.0;
                        Ok::<_, HandlerError>(closed && disposition.is_complete())
                    })?;
            Ok(done && closed)
        })
        .map(complete)
}

pub(super) fn stage_complete(state: &JurisdictionState, year: u16, kind: Kind) -> bool {
    match kind {
        Kind::Meets => state
            .history
            .meets
            .get(&year)
            .is_some_and(crate::census::MeetCensus::is_terminal),
        Kind::Results => state
            .history
            .results
            .get(&year)
            .is_some_and(results_arms::ResultsStageOutcome::is_terminal),
    }
}

pub(super) fn source(
    state: &JurisdictionState,
    year: u16,
    slug: &str,
    kind: Kind,
) -> Result<(Disposition, u64, Option<ResolutionCounters>), HandlerError> {
    match kind {
        Kind::Meets => meet(state, year, slug),
        Kind::Results => result(state, year, slug),
    }
}

fn meet(
    state: &JurisdictionState,
    year: u16,
    slug: &str,
) -> Result<(Disposition, u64, Option<ResolutionCounters>), HandlerError> {
    let Some(census) = state.history.meets.get(&year) else {
        return Ok((Disposition::Unknown, 0, None));
    };
    let mut selected = census.sources.iter().filter(|source| source.slug == slug);
    let Some(source) = selected.next() else {
        return Ok((Disposition::Unknown, 0, None));
    };
    let uncertain = !source.errors.is_empty()
        || !source.unfinished.is_empty()
        || source.withheld != Some(0)
        || source
            .unresolved
            .is_none_or(|value| value.rows > 0 || value.labels > 0);
    let disposition =
        if selected.next().is_some() || (source.disposition.is_complete() && uncertain) {
            Disposition::Partial
        } else {
            source.disposition
        };
    Ok((disposition, count(source.rows)?, None))
}

fn result(
    state: &JurisdictionState,
    year: u16,
    slug: &str,
) -> Result<(Disposition, u64, Option<ResolutionCounters>), HandlerError> {
    let Some(outcome) = state.history.results.get(&year) else {
        return Ok((Disposition::Unknown, 0, None));
    };
    let mut selected = outcome
        .per_source
        .iter()
        .filter(|source| source.slug == slug);
    let Some(source) = selected.next() else {
        return Ok((Disposition::Unknown, 0, None));
    };
    let uncertain = source.errors > 0
        || !source.unfinished.is_empty()
        || source.withheld != Some(0)
        || source.rows.is_none()
        || source
            .unresolved
            .is_none_or(|value| value.rows > 0 || value.labels > 0)
        || source.resolution.is_none_or(|value| value.unresolved > 0);
    let disposition =
        if selected.next().is_some() || (source.disposition.is_complete() && uncertain) {
            Disposition::Partial
        } else {
            source.disposition
        };
    Ok((
        disposition,
        source
            .rows
            .map(count)
            .transpose()?
            .map_or(0, core::convert::identity),
        source.resolution,
    ))
}
