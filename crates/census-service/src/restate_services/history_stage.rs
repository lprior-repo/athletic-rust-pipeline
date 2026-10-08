use super::wire::{HistoryWindow, JurisdictionRequest};
use census_domain::UsJurisdiction;

#[derive(Clone, Copy)]
pub(super) struct HistoricalStageScope {
    pub(super) jurisdiction: UsJurisdiction,
    pub(super) year: u16,
    pub(super) refresh: bool,
    pub(super) window: HistoryWindow,
    pub(super) observed_on: chrono::NaiveDate,
}

impl HistoricalStageScope {
    pub(super) fn for_year(
        request: &JurisdictionRequest,
        year: u16,
        observed_on: chrono::NaiveDate,
    ) -> Self {
        Self {
            jurisdiction: request.jurisdiction,
            year,
            refresh: request.refresh,
            window: request.history,
            observed_on,
        }
    }
}
