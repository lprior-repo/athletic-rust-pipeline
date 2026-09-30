use super::Options;
use crate::{CrawlError, CrawlResult};
use census_domain::UsJurisdiction;

pub(in crate::arbiter) fn targets(
    options: &Options,
) -> CrawlResult<Vec<(UsJurisdiction, &'static str)>> {
    let requested: Vec<UsJurisdiction> = if options.states.is_empty() {
        super::super::covered_states().collect()
    } else {
        options.states.clone()
    };
    let mut targets = Vec::with_capacity(requested.len());
    for state in requested {
        let org = super::super::org_for(state).ok_or_else(|| CrawlError::Invariant {
            detail: format!("no Arbiter organisation is registered for {}", state.code()),
        })?;
        if targets.iter().any(|(seen, _)| *seen == state) {
            continue;
        }
        targets.push((state, org));
    }
    Ok(targets)
}
