use super::{invalid, ExportDataset, SelectedInputOrigin};
use crate::report::ReportResult;

impl ExportDataset {
    pub(crate) fn ensure_current_selected_authority(&self) -> ReportResult<()> {
        if self.selected_profile_id().is_none() {
            return Ok(());
        }
        self.ensure_input_binding()?;
        self.require_current_selected_policy()
    }

    pub(super) fn require_current_selected_policy(&self) -> ReportResult<()> {
        if matches!(
            self.selected_input_origin,
            SelectedInputOrigin::HistoricalReplay
                | SelectedInputOrigin::HistoricalIdentityReplay
                | SelectedInputOrigin::HistoricalParticipationReplay
                | SelectedInputOrigin::HistoricalDocumentaryReplay
        ) {
            return Err(invalid(
                "historical selected replay cannot authorize current publication",
            ));
        }
        self.selected_proofs
            .as_ref()
            .ok_or_else(|| invalid("selected publication requires retained identity proofs"))?
            .require_current()?;
        census_domain::profile_execution::validate_current_selected_qualification_profile(
            self.selected_authority()?.profile(),
        )
        .map_err(|source| {
            invalid(&format!(
                "unsupported selected publication profile: {source}"
            ))
        })?;
        Ok(())
    }

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
