use census_domain::model::{Mark, RelayResult, SourceObservation};

use super::super::Entity;

impl Entity for SourceObservation {
    fn entity_id(&self) -> &str {
        self.id()
    }

    fn merge(&mut self, other: Self) {
        self.absorb(other);
    }
}

impl Entity for RelayResult {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if matches!(self.mark, Mark::Raw(_)) && !matches!(other.mark, Mark::Raw(_)) {
            self.mark = other.mark;
        }
        if self.place.is_none() {
            self.place = other.place;
        }
        if self.round.is_none() {
            self.round = other.round;
        }
        if self.timing.is_none() {
            self.timing = other.timing;
        }
        if self.source_team.is_none() {
            self.source_team = other.source_team;
        }
        other.members.into_iter().for_each(|member| {
            if !self.members.iter().any(|held| held.order == member.order) {
                self.members.push(member);
            }
        });
        self.members.sort_by_key(|member| member.order);
        super::union_vec(&mut self.evidence, other.evidence);
    }
}

#[cfg(test)]
#[path = "observations_tests.rs"]
mod tests;
