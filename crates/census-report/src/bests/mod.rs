use crate::report::Scope;

pub(crate) mod context;
mod events;
mod exclusions;
mod notation;
mod parents;
pub(crate) mod participation;
mod reduce;
mod selection;
mod write;

pub use events::sport_of;
pub use exclusions::{CombinedExclusion, CombinedExclusionReason, SelectionOutcome};
pub use notation::{disagreement, format_time, mark_text};
pub use participation::{
    ParticipationExclusion, ParticipationExclusionOutcome, ParticipationExclusionReason,
};
pub(crate) use reduce::{
    build_admitted_outcome, build_from_derivation, build_selected_outcome, reported_best_groups,
    ReportedBestGroup,
};
pub use reduce::{build_from_dataset, build_with_exclusions};
pub use selection::{
    Conflict, HistoricalInventory, Population, ResultGroupInventory, SelectionAthlete,
    SelectionInventory, SelectionMeet, SelectionResult, SelectionSource, SharedSelection,
};
pub use write::write;
pub(crate) use write::write_outcome;

pub(crate) use parents::Parents;

#[derive(Debug, Clone)]
pub struct Options {
    pub scope: Scope,
    pub grad_year: Option<i16>,
    pub limit: Option<usize>,
    pub jurisdictions: Vec<census_domain::UsJurisdiction>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
            jurisdictions: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) fn build_participation_outcome(
    admission: &crate::report::EventAdmission<'_>,
    options: &Options,
    view: &crate::export::VerifiedParticipationView,
) -> crate::report::ReportResult<SelectionOutcome> {
    reduce::build_outcome(
        admission,
        options,
        participation::ParticipationSelection::Selected(view),
    )
}

#[cfg(test)]
pub(crate) use write::selected::write as write_selected_identity_for_test;
#[cfg(test)]
pub(crate) use write::tests::public_identity::row as public_identity_selection_for_test;
