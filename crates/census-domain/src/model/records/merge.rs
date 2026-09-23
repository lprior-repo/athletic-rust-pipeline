//! The record of a canonical merge: which id was retired, which one survived, and the decision that
//! said so.
//!
//! A merge is a decision this program made, and decisions can be wrong. A pure `retired -> surviving`
//! redirect cannot be reversed — nothing in it says why the two ids were distinct, nor which sources
//! backed each side — so this row keeps both sides' material and provenance beside the decision's own
//! provenance. An operator reading a merge can therefore re-derive either side without fetching a
//! source again, and a later pass that disagrees has the words the earlier decision was made in.
//!
//! One row per retired id: a re-derivation that reaches the same conclusion replaces the row, and a
//! decision that is revised writes the row for the id it retires now.

use serde::{Deserialize, Serialize};

use super::source::SourceEntityKind;

/// One canonical id retired into another, with the evidence on both sides and the decision's own.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalMerge {
    /// The store row id: `{entity.slug()}:{retired_id}`, one row per retired id.
    pub id: String,
    /// Which canonical table both ids name.
    pub entity: SourceEntityKind,
    /// The id that no longer resolves.
    pub retired_id: String,
    /// The id it now redirects to.
    pub surviving_id: String,
    /// The retired row's natural-key material, rendered the way its own natural key renders it.
    pub retired_key: String,
    /// The survivor's material, rendered the same way, so an operator reads the same words for both.
    pub surviving_key: String,
    /// The source identities the retired id was bound from (`namespace:id`, with the URL when known).
    pub retired_sources: String,
    /// The source identities the surviving id is bound from.
    pub surviving_sources: String,
    /// The decision itself: the family and verdict that declared the two ids one subject, with the
    /// detail the verdict carried.
    pub rationale: String,
    /// The day the decision was recorded (`yyyy-mm-dd`).
    pub decided_on: String,
}

impl CanonicalMerge {
    /// Mint the row for one retired id, with the decision still to be stated.
    pub fn new(
        retired_id: impl Into<String>,
        surviving_id: impl Into<String>,
        entity: SourceEntityKind,
    ) -> Self {
        let retired_id = retired_id.into();
        Self {
            id: format!("{}:{retired_id}", entity.slug()),
            entity,
            retired_id,
            surviving_id: surviving_id.into(),
            retired_key: String::new(),
            surviving_key: String::new(),
            retired_sources: String::new(),
            surviving_sources: String::new(),
            rationale: String::new(),
            decided_on: String::new(),
        }
    }

    /// Carry the retired row's own natural-key material, as that row's natural key renders it.
    pub fn with_retired_key(mut self, retired_key: impl Into<String>) -> Self {
        self.retired_key = retired_key.into();
        self
    }

    /// Carry the survivor's material, rendered the same way, so both sides read in the same words.
    pub fn with_surviving_key(mut self, surviving_key: impl Into<String>) -> Self {
        self.surviving_key = surviving_key.into();
        self
    }

    /// Carry both sides' source identities: what lets a split be re-derived.
    pub fn with_sources(
        mut self,
        retired_sources: impl Into<String>,
        surviving_sources: impl Into<String>,
    ) -> Self {
        self.retired_sources = retired_sources.into();
        self.surviving_sources = surviving_sources.into();
        self
    }

    /// Carry the decision: the family and verdict that declared the two ids one subject.
    pub fn with_rationale(mut self, rationale: impl Into<String>) -> Self {
        self.rationale = rationale.into();
        self
    }

    /// Carry the day the decision was recorded.
    pub fn with_decided_on(mut self, decided_on: impl Into<String>) -> Self {
        self.decided_on = decided_on.into();
        self
    }
}
