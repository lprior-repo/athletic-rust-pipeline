//! Event metadata (`eventsTF`) for track-and-field results.
//!
//! Split into a directory module when this file outgrew the repository's 300-line budget: the keyed
//! record types stay here and keep their paths, `schema` recognises one metadata object, and
//! `index` assembles the keyed index and resolves a result's join into it.

mod index;
mod schema;

use std::collections::BTreeMap;

pub(super) use index::{index, lookup};

type Key = (u64, Option<u64>);
pub(super) type Index = BTreeMap<Key, Option<Event>>;

#[derive(PartialEq, Eq)]
pub(super) struct Event {
    pub(super) name: String,
    pub(super) description: Option<String>,
    pub(super) kind: Option<String>,
    pub(super) units: Option<String>,
    pub(super) personal: bool,
}

#[cfg(test)]
mod tests;
