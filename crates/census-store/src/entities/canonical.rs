use census_domain::model::{
    id_collision, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, NaturalKey, RetainedConflict,
};

use super::super::Entity;
use super::union_vec;

fn collision<T: NaturalKey>(id: &str, kept: &T, dropped: &T) -> Option<RetainedConflict> {
    if kept.same_natural_key(dropped) {
        return None;
    }
    Some(id_collision(id, kept, dropped))
}

fn record(conflicts: &mut Vec<RetainedConflict>, conflict: RetainedConflict) {
    if !conflicts.contains(&conflict) {
        conflicts.push(conflict);
    }
}

impl Entity for CanonicalSchool {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        if self.city.is_none() {
            self.city = other.city;
        }
        if self.association.is_none() {
            self.association = other.association;
        }
        if self.classification.is_none() {
            self.classification = other.classification;
        }
        if self.enrollment.is_none() {
            self.enrollment = other.enrollment;
        }
        if self.school_website.is_none() {
            self.school_website = other.school_website;
        }
        if self.athletics_website.is_none() {
            self.athletics_website = other.athletics_website;
        }
        self.co_op |= other.co_op;
        if other.name.len() > self.name.len() && other.name.starts_with(&self.name) {
            self.name = other.name;
        }
        union_vec(&mut self.aliases, &other.aliases);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalTeam {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        if self.level.is_none() {
            self.level = other.level;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalCoach {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        self.publish();
        route_published_email(self, other.professional_email);
        route_published_email(self, other.personal_email);
        if self.phone.is_none() {
            self.phone = other.phone;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
        union_vec(&mut self.tenure_evidence, &other.tenure_evidence);
    }

    fn publish(&mut self) {
        let professional = self.professional_email.take();
        let personal = self.personal_email.take();
        route_published_email(self, professional);
        route_published_email(self, personal);
    }
}

fn route_published_email(coach: &mut CanonicalCoach, address: Option<String>) {
    if let Some(address) = address {
        coach.set_published_email(&address);
    }
}

impl Entity for CanonicalAthlete {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        union_vec(&mut self.known_names, &other.known_names);
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.public_profile_urls, &other.public_profile_urls);
        other
            .source
            .into_iter()
            .chain(other.source_links)
            .for_each(|identity| self.add_identity(identity));
        union_vec(&mut self.evidence, &other.evidence);
        for observation in other.observed_grades {
            if !self.observed_grades.contains(&observation) {
                self.observed_grades.push(observation);
            }
        }
        self.publish();
    }
}

impl Entity for CanonicalMeet {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        if self.location.is_none() {
            self.location = other.location;
        }
        if self.end_date.is_none() {
            self.end_date = other.end_date;
        }
        if self.level == census_domain::model::CompetitionLevel::Unknown {
            self.level = other.level;
        }
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.source_urls, &other.source_urls);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalEvent {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        union_vec(&mut self.source_labels, &other.source_labels);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalPerformance {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
        if self.wind_mps.is_none() {
            self.wind_mps = other.wind_mps;
        }
        if self.place.is_none() {
            self.place = other.place;
        }
        if self.observed_grade.is_none() {
            self.observed_grade = other.observed_grade;
        }
        if self.timing.is_none() {
            self.timing = other.timing;
        }
        if self.source_athlete.is_none() {
            self.source_athlete = other.source_athlete;
        }
        union_vec(&mut self.evidence, &other.evidence);
    }
}

#[cfg(test)]
#[path = "canonical_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "canonical_performance_tests.rs"]
mod performance_tests;
