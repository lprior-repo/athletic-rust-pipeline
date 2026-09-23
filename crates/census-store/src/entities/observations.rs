//! `Entity` for the observation rows: what one source published about one of its own objects.
//!
//! These rows are the reversal point for a canonical decision, so their merge is deliberately narrow.
//! A second sighting of the same provider object — the same directory page read again, a roster
//! re-crawled the next week — folds into the row the first sighting wrote: the identity (the
//! provider's object id and the row it was read from) is never rewritten, a field the newer sighting
//! publishes and the older one does not fills the blank, and the day kept is the earliest the object
//! was seen. A merge that cannot rewrite evidence is what lets a wrong canonical merge be re-decided
//! from these rows alone.

use census_domain::model::SourceObservation;

use super::super::Entity;

impl Entity for SourceObservation {
    fn entity_id(&self) -> &str {
        self.id()
    }

    fn merge(&mut self, other: Self) {
        self.absorb(other);
    }
}

#[cfg(test)]
#[path = "observations_tests.rs"]
mod tests;
