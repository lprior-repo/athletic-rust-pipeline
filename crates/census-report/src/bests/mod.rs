use crate::report::Scope;

pub(crate) mod context;
mod events;
mod notation;
mod parents;
mod reduce;
mod selection;
mod write;

pub use events::sport_of;
pub use notation::{disagreement, format_time, mark_text};

pub(crate) use reduce::build_admitted;
pub use reduce::build_from_dataset;
pub use selection::{
    Conflict, Population, SelectionAthlete, SelectionMeet, SelectionResult, SelectionSource,
    SharedSelection,
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
