//! Source request construction: action/body wire shapes and bounded URL validation.
//!
//! Each item below was previously declared in `request.rs`; every path that resolved there
//! (`request::RequestSpec`, `request::MAX_START`, ...) still resolves through this module.

mod action;
mod body;
mod build;

pub(crate) use self::action::{RankingsAction, RequestAction, RequestSpec, SearchBody};
pub(crate) use self::body::{RankingsQParamsInner, RankingsQuery, RequestBody};
pub(crate) use self::build::{build, rankings_spec};

pub(crate) const MAX_START: u32 = 1_000_000;
pub(crate) const MAX_QUERY_BYTES: usize = 2_048;

#[cfg(test)]
use crate::domain::identity::ProfileUrl;
#[cfg(test)]
use crate::runtime::protocol::{RankingsCapture, SourceResource};
#[cfg(test)]
use url::Url;

#[cfg(test)]
mod tests;
