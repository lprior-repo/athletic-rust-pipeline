pub use census_domain::model::HistoryWindow;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::census::MeetCensus;
use crate::restate_services::results_arms::ResultsStageOutcome;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoricalProgress {
    pub meets: BTreeMap<u16, MeetCensus>,
    pub results: BTreeMap<u16, ResultsStageOutcome>,
}

impl HistoricalProgress {
    pub fn meets_terminal(&self, window: &HistoryWindow) -> bool {
        window
            .years()
            .all(|year| self.meets.get(&year).is_some_and(MeetCensus::is_terminal))
    }
    pub fn results_terminal(&self, window: &HistoryWindow) -> bool {
        window.years().all(|year| {
            self.results
                .get(&year)
                .is_some_and(ResultsStageOutcome::is_terminal)
        })
    }
}
