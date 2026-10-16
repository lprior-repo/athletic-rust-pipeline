use super::{ExportDataset, SelectedInputOrigin};

impl ExportDataset {
    pub(crate) fn is_historical_selected_replay(&self) -> bool {
        matches!(
            self.selected_input_origin,
            super::SelectedInputOrigin::HistoricalReplay
        )
    }

    pub(crate) fn uses_historical_identity_contract(&self) -> bool {
        matches!(
            self.selected_input_origin,
            SelectedInputOrigin::HistoricalReplay | SelectedInputOrigin::HistoricalIdentityReplay
        )
    }

    pub(crate) fn uses_historical_participation_contract(&self) -> bool {
        matches!(
            self.selected_input_origin,
            SelectedInputOrigin::HistoricalParticipationReplay
        )
    }
}
