//! Ranking index persistence, split by responsibility.
//!
//! Page publication, its batch staging, the reference lookups and the seal
//! lifecycle each live in their own submodule; the `crate::store` paths keep
//! resolving through the re-exports below.

mod batch;
mod read;
mod seal;
mod write;

pub(in crate::store) use read::{ranking_athlete_refs, ranking_name_refs};
pub(in crate::store) use seal::{ranking_snapshot, seal_rankings};
pub(in crate::store) use write::{drop_rankings_page, put_rankings_page};
