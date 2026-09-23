//! Source request construction: the `SourceResource` mapping onto bounded request specs.
//!
//! The wire shapes (`RequestSpec`, `RequestAction`, the bodies) and the rankings spec the transport
//! rebuilds per page live in `athleticnet-browser::request`; what stays here is the producer that
//! needs this crate's source vocabulary and the domain's profile URLs.

mod build;

pub(crate) use self::build::build;
pub(crate) const MAX_START: u32 = 1_000_000;
pub(crate) const MAX_QUERY_BYTES: usize = 2_048;

#[cfg(test)]
use crate::domain::identity::ProfileUrl;
#[cfg(test)]
use crate::runtime::protocol::SourceResource;
#[cfg(test)]
use athleticnet_browser::protocol::RankingsCapture;
#[cfg(test)]
use url::Url;

#[cfg(test)]
mod tests;
