//! `Entity` for the canonical rows: the seven types the merge materializes, keyed by the id the
//! cross-source reconciliation stamps on them.

use census_domain::model::{
    id_collision, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, NaturalKey, RetainedConflict,
};

use super::super::Entity;
use super::union_vec;

/// The finding for one merge of two rows that share a canonical id, or `None` when the two rows state
/// the same natural key and are therefore one subject.
///
/// Two natural keys that minted one id are a collision, not a match: an id is the first 64 bits of a
/// SHA-256, and the merge keys on the id alone, so absorbing the other row would merge two subjects
/// into a third one that is neither. The row already there therefore keeps every field it holds, and
/// the finding names the id, both sides' material and both sides' sources for an operator to resolve.
///
/// The test is the cheap one: already-decoded fields compared, no hashing, and nothing formatted or
/// allocated unless it fails.
fn collision<T: NaturalKey>(id: &str, kept: &T, dropped: &T) -> Option<RetainedConflict> {
    if kept.same_natural_key(dropped) {
        return None;
    }
    Some(id_collision(id, kept, dropped))
}

/// Take one finding onto a row, unless a previous read already took it: the store re-merges a row on
/// every read of the table it lives in, so one collision stays one row in the findings table.
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
        if self.professional_email.is_none() {
            self.professional_email = other.professional_email;
        }
        if self.phone.is_none() {
            self.phone = other.phone;
        }
        // Mailboxes do not merge - the row already there keeps its own - but the withheld fact does:
        // for a row whose mailbox was dropped, the flag is the only surviving evidence that one was
        // ever observed, and the copy that recorded it is not necessarily the one that survived.
        self.email_withheld |= other.email_withheld;
        union_vec(&mut self.source_identities, &other.source_identities);
        union_vec(&mut self.evidence, &other.evidence);
    }

    fn publish(&mut self) {
        let Some(email) = self.professional_email.as_deref() else {
            // Nothing is left to derive from: the mailbox was dropped, so the flag the row was read
            // with is what says whether one was dropped or never observed at all.
            return;
        };
        match census_domain::model::professional_email(email) {
            Some(published) => {
                // A mailbox that ships settles the flag whichever way the row was read: a withheld
                // marking a file carries never keeps a public mailbox off the wire.
                self.professional_email = Some(published);
                self.email_withheld = false;
            }
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
        if let Some(conflict) = collision(self.id.as_str(), self, &other) {
            record(&mut self.retained_conflicts, conflict);
            return;
        }
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
        self.publish();
    }

    /// Derive the identity confidence from the row's own grade observations.
    ///
    /// A row written by one pass is never merged again, so deriving this only in
    /// [`merge`](Entity::merge) left an athlete whose single observation agrees with its cohort
    /// carrying the constructor's default — a row the workbook then reported as below the identity
    /// bar and the review queue kept as a cohort finding. Every read publishes, so the derived value
    /// is the one the report, the workbook, the snapshot and the Restate handlers see.
    fn publish(&mut self) {
        if let Some(confidence) = self.derived_identity_confidence() {
            self.identity_confidence = confidence;
        }
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
        if self.source_athlete.is_none() {
            self.source_athlete = other.source_athlete;
        }
        if self.timing.is_none() {
            self.timing = other.timing;
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
