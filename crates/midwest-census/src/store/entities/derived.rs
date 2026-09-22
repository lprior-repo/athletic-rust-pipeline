//! `Entity` for the derived rows: what the census keeps about its own findings — identities, open
//! conflicts, review cases, coverage, meet references, snapshots and access conditions — rather than
//! about a school or an athlete.

use census_domain::model::{
    CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase, ReviewState,
    SourceAccessCondition, SourceMeetRef, SourceObjectIdentity,
};

use super::super::Entity;

/// The derived rows are observations of state, not history: the newest derivation of one id replaces
/// the earlier one, because the id already binds the subject and a stale copy of a resolved conflict
/// or a renamed school is exactly what the derive step exists to correct.
impl Entity for SourceObjectIdentity {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

impl Entity for RetainedConflict {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

impl Entity for ReviewCase {
    fn entity_id(&self) -> &str {
        &self.id
    }

    /// A decision survives a re-derivation: the case is re-derived as pending on every pass, and a
    /// verdict the lane already reached must not be reset by the next one.
    fn merge(&mut self, other: Self) {
        let decided = match self.state {
            ReviewState::Pending => other.state,
            decided => decided,
        };
        *self = other;
        self.state = decided;
    }
}

impl Entity for CoverageRow {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

impl Entity for SourceMeetRef {
    fn entity_id(&self) -> &str {
        &self.id
    }

    /// A meet's identity is the first sighting; a later one may only complete the row. The fields a
    /// meet owns are never rewritten by a re-crawl, so a name correction upstream cannot silently
    /// change which meet a stored result belongs to.
    fn merge(&mut self, other: Self) {
        if other.observed_on < self.observed_on {
            self.observed_on = other.observed_on;
        }
        if self.name.is_empty() {
            self.name = other.name;
        }
        if self.date.is_none() {
            self.date = other.date;
        }
        if self.venue.is_empty() {
            self.venue = other.venue;
        }
        if self.results_url.is_empty() {
            self.results_url = other.results_url;
        }
    }
}

impl Entity for CollectionSnapshot {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, other: Self) {
        *self = other;
    }
}

impl Entity for SourceAccessCondition {
    fn entity_id(&self) -> &str {
        &self.id
    }

    /// Two observations of one `(kind, host)` fold to the later one, and the block never shortens.
    ///
    /// A re-observation that reports a nearer deadline must not unblock a host an earlier
    /// observation blocked for longer: the conservative reading is the one that costs a run nothing
    /// and costs the source no traffic.
    fn merge(&mut self, other: Self) {
        let later = other.observed_at >= self.observed_at;
        let cooldown = match (
            self.cooldown_until.as_deref(),
            other.cooldown_until.as_deref(),
        ) {
            (Some(mine), Some(theirs)) if theirs > mine => Some(theirs.to_string()),
            (Some(mine), _) => Some(mine.to_string()),
            (None, theirs) => theirs.map(str::to_string),
        };
        let retry_after = match (self.retry_after_seconds, other.retry_after_seconds) {
            (Some(mine), Some(theirs)) => Some(mine.max(theirs)),
            (mine, theirs) => mine.or(theirs),
        };
        if later {
            *self = other;
        }
        self.cooldown_until = cooldown;
        self.retry_after_seconds = retry_after;
    }
}
