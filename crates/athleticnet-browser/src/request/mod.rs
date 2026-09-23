//! The wire action a browser fetch is handed, and the body that action turns into.
//!
//! The `SourceResource` mapping stays in the pipeline crate: `build`/`search`/`profile`/`team`
//! need the source vocabulary and the domain's profile URLs. What lives here is the shape the
//! transport consumes plus the spec the rankings transport rebuilds per page, so a second acquirer
//! (the census) can build the same spec without depending on the pipeline's source types.

mod action;
mod body;
mod build;

#[cfg(test)]
mod tests;

pub use self::action::{RankingsAction, RequestAction, RequestSpec, SearchBody};
pub use self::body::{RankingsQParamsInner, RankingsQuery, RequestBody};
pub use self::build::{endpoint, rankings_spec, safe, safe_text};
