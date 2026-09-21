//! `Entity` for the canonical types: the id a row is keyed by, and how a duplicate observation is
//! absorbed at read time.

use census_domain::model::{
    CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet, CanonicalPerformance,
    CanonicalSchool, CanonicalTeam,
};

use super::Entity;

fn union_vec<T: PartialEq + Clone>(left: &mut Vec<T>, right: &[T]) {
    for item in right {
        if !left.contains(item) {
            left.push(item.clone());
        }
    }
}

impl Entity for CanonicalSchool {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
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
            // keep the longer, more specific name
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
        if self.professional_email.is_none() {
            self.professional_email = other.professional_email;
        }
        if self.phone.is_none() {
            self.phone = other.phone;
        }
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }

    fn publish(&mut self) {
        let Some(email) = self.professional_email.as_deref() else {
            return;
        };
        match census_domain::model::professional_email(email) {
            Some(published) if published == email => {}
            Some(published) => self.professional_email = Some(published),
            None => {
                // A personal mailbox never ships, whichever adapter accepted one.
                self.professional_email = None;
                self.email_withheld = true;
            }
        }
    }

    fn withheld_mailboxes(&self) -> usize {
        usize::from(self.email_withheld)
    }
}

impl Entity for CanonicalAthlete {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
        union_vec(&mut self.known_names, &other.known_names);
        union_vec(&mut self.sports, &other.sports);
        union_vec(&mut self.public_profile_urls, &other.public_profile_urls);
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
        for observation in other.observed_grades {
            if !self.observed_grades.contains(&observation) {
                self.observed_grades.push(observation);
            }
        }
        // Any observation that disagrees with the cohort lowers confidence instead of silently
        // rewriting the athlete's graduating class.
        if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() != self.grad_year)
        {
            self.identity_confidence = census_domain::model::Confidence::LOW;
        } else if self
            .observed_grades
            .iter()
            .any(|observation| observation.grad_year() == self.grad_year)
        {
            self.identity_confidence = census_domain::model::Confidence::HIGH;
        }
    }
}

impl Entity for CanonicalMeet {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
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
        union_vec(&mut self.source_labels, &other.source_labels);
        union_vec(&mut self.evidence, &other.evidence);
    }
}

impl Entity for CanonicalPerformance {
    fn entity_id(&self) -> &str {
        self.id.as_str()
    }

    fn merge(&mut self, other: Self) {
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
        union_vec(&mut self.evidence, &other.evidence);
    }
}
