use crate::report::Scope;

mod events;
mod key;
mod measure;
mod notation;
mod parents;
mod reduce;
mod selection;
mod write;

pub use events::{is_relay, sport_of};
pub use key::{
    classify_wind, is_wind_sensitive, resolve_timing, should_replace, tie_break_later, PrKey,
    SurfaceClass, TimingClass, WindClass,
};
pub use measure::{field_micrometres, mark_unit, Measure};
pub use notation::{disagreement, format_time, mark_text};
pub use reduce::build;
pub use selection::{Conflict, Population, SharedSelection};
pub use write::write;

pub(crate) use parents::Parents;

#[derive(Debug, Clone)]
pub struct Options {
    pub scope: Scope,
    pub grad_year: Option<i16>,
    pub limit: Option<usize>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            scope: Scope::Core,
            grad_year: Some(2027),
            limit: None,
        }
    }
}

#[cfg(test)]
mod tests;
