mod action;
mod body;
mod build;

#[cfg(test)]
mod tests;

pub use self::action::{RankingsAction, RequestAction, RequestSpec, SearchBody};
pub use self::body::{RankingsQParamsInner, RankingsQuery, RequestBody};
pub use self::build::{endpoint, rankings_spec, safe, safe_text, same_origin};
