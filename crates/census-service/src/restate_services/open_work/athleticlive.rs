use super::{count, obligation, push, ReadJurisdiction};
use crate::restate_services::wire::{AthleticLiveObligation, SourceObjectOpen};
use census_crawl::CollectionDisposition as Disposition;
use restate_sdk::prelude::HandlerError;

pub(super) fn project(
    jurisdictions: &mut [ReadJurisdiction],
    endpoints: &mut Vec<SourceObjectOpen>,
) -> Result<(), HandlerError> {
    jurisdictions.iter_mut().try_for_each(|entry| {
        let owed = entry
            .state
            .athleticlive
            .as_ref()
            .filter(|work| !work.is_terminal());
        entry.row.athleticlive_manifest = owed.cloned();
        entry.row.athleticlive_manifest_reason = owed.map(AthleticLiveObligation::reason);
        let Some(work) = owed else {
            return Ok(());
        };
        let mut row = obligation(
            format!("{}/athleticlive_manifest", entry.row.identity),
            Disposition::Unknown,
        );
        if let AthleticLiveObligation::Finished { outcome, .. } = work {
            row.observations = count(outcome.records)?;
            row.disposition = if outcome.disposition.is_complete() {
                Disposition::Partial
            } else {
                outcome.disposition
            };
        }
        push(endpoints, row)
    })
}
