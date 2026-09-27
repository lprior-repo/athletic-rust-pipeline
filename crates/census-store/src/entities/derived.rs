use census_domain::model::{
    AppliedAthleteIdentity, CollectionSnapshot, CoverageRow, RetainedConflict, ReviewCase,
    ReviewState, ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef, SourceObjectIdentity,
};

use super::super::Entity;

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

impl Entity for ReviewVerdictRecord {
    fn entity_id(&self) -> &str {
        &self.id
    }

    fn merge(&mut self, _other: Self) {}
}

impl Entity for AppliedAthleteIdentity {
    fn entity_id(&self) -> &str {
        &self.id
    }
    fn merge(&mut self, other: Self) {
        *self = other;
    }
}
