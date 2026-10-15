use crate::report::Scope;

pub(crate) mod context;
mod events;
mod exclusions;
mod notation;
mod parents;
mod reduce;
mod selection;
mod write;

pub use events::sport_of;
pub use exclusions::{CombinedExclusion, CombinedExclusionReason, SelectionOutcome};
pub use notation::{disagreement, format_time, mark_text};
pub(crate) use reduce::build_admitted_outcome;
pub use reduce::{build_from_dataset, build_with_exclusions};
pub use selection::{
    Conflict, HistoricalInventory, Population, ResultGroupInventory, SelectionAthlete,
    SelectionInventory, SelectionMeet, SelectionResult, SelectionSource, SharedSelection,
};
pub use write::write;

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
