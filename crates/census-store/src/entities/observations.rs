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
