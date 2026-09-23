//! The athlete-identity family's question, built from the store: the two canonical rows a case
//! compares, and the flags the store computes before a model sees anything.
//!
//! The merge retained the finding because it kept two rows apart under one key: the same school, the
//! same normalized name and the same graduating class, differing in the gender component the
//! canonical athlete id is minted from. The retained case's detail names the ids it collided with,
//! but ids in prose are not a key, so this module rebuilds the group from the store's own fields with
//! the rule the conflict family groups by, and compares the case's subject with the first other row
//! of that group.
//!
//! Every flag is computed from the canonical rows alone, in [`super::athlete_flags`], and stated in
//! the packet: the model is asked what the flags leave open, never asked to notice a contradiction
//! itself. Each side's own evidence — the provider ids the row is known by, its name, its school, the
//! grade observations that imply a class, its gender and the profile URLs the providers published —
//! rides with it, so the comparison is made on what the store holds.

use std::collections::{BTreeMap, HashMap};

use census_domain::model::{
    CanonicalAthlete, ObservedGrade, ReviewCase, ReviewEvidenceFact, ReviewPacket, SourceIdentity,
};

use super::athlete_flags::{flags, key, IdentityKey};
use super::families::IDENTITY_FIELD;
use super::packets::{case_fact, fact, index_by_id};

/// Which side of the comparison a fact belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    /// The case's subject: the row the operator was pointed at.
    Subject,
    /// The row it collided with.
    Other,
}

impl Side {
    /// The fact-name prefix the packet states this side under.
    const fn prefix(self) -> &'static str {
        match self {
            Self::Subject => "side_a",
            Self::Other => "side_b",
        }
    }
}

/// The athlete table as the lane reads it: every row by id, and the rows the merge kept apart.
#[derive(Debug, Default)]
pub(super) struct AthleteIndex {
    rows: HashMap<String, CanonicalAthlete>,
    /// Canonical ids sharing one merge key with this id, when more than one row holds that key.
    groups: HashMap<String, Vec<String>>,
}

impl AthleteIndex {
    /// Index the table: rows by their id, and the groups a case can be retained from.
    pub(super) fn read(rows: Vec<CanonicalAthlete>) -> Self {
        let mut by_key: BTreeMap<IdentityKey, Vec<String>> = BTreeMap::new();
        for row in &rows {
            by_key.entry(key(row)).or_default().push(row.id.to_string());
        }
        let mut groups: HashMap<String, Vec<String>> = HashMap::new();
        for mut ids in by_key.into_values() {
            if ids.len() < 2 {
                continue;
            }
            ids.sort();
            for id in &ids {
                groups.insert(id.clone(), ids.clone());
            }
        }
        Self {
            rows: index_by_id(rows, |row| row.id.to_string()),
            groups,
        }
    }

    /// The two rows a case compares, and the whole group they came from.
    ///
    /// `None` when the store no longer holds both: the finding the case names is gone, so the lane
    /// leaves the case alone rather than inventing a side to compare with.
    pub(super) fn compare(
        &self,
        subject_id: &str,
    ) -> Option<(&CanonicalAthlete, &CanonicalAthlete, &[String])> {
        let subject = self.rows.get(subject_id)?;
        let group = self.groups.get(subject_id)?;
        let other_id = group.iter().find(|id| id.as_str() != subject_id)?;
        let other = self.rows.get(other_id)?;
        Some((subject, other, group))
    }
}

/// The question one athlete case asks: two rows, their own evidence, and the store's flags.
pub(super) fn packet(
    case: &ReviewCase,
    subject: &CanonicalAthlete,
    other: &CanonicalAthlete,
    group: &[String],
) -> ReviewPacket {
    let mut packet = ReviewPacket::new(case.subject_id.clone(), case.subject.clone())
        .with_case(case_fact(case))
        .with_evidence(fact("question", "are the two sides the same athlete?"))
        .with_evidence(fact("answer_field", IDENTITY_FIELD))
        .with_evidence(fact("answer_values", "same_person | different_person"))
        .with_evidence(fact("candidate_ids", &group.join(", ")));
    let sides = side_facts(Side::Subject, subject)
        .into_iter()
        .chain(side_facts(Side::Other, other));
    for evidence in sides {
        packet = packet.with_evidence(evidence);
    }
    for flag in flags(subject, other) {
        packet = packet.with_evidence(flag.evidence());
    }
    packet
}

/// One row as the packet states it: the fields the merge keyed it on, the grade observations that
/// imply its class, and every provider identity it carries.
fn side_facts(side: Side, row: &CanonicalAthlete) -> Vec<ReviewEvidenceFact> {
    let prefix = side.prefix();
    let field = |name: &str| format!("{prefix}_{name}");
    let mut facts = vec![
        fact(&field("id"), row.id.as_str()),
        fact(&field("name"), &row.canonical_name),
        fact(&field("school"), row.school.as_str()),
        fact(&field("grad_year"), &row.grad_year.to_string()),
        fact(&field("gender"), &format!("{:?}", row.gender)),
    ];
    for observation in observations(row) {
        facts.push(fact(
            &field("grad_evidence"),
            &observation_text(observation),
        ));
    }
    for identity in identities(row) {
        // The namespace is the fact's source: a source athlete id means nothing without the provider
        // that issued it, and two providers can number their athletes independently.
        facts.push(ReviewEvidenceFact::new(
            identity.namespace.to_string(),
            field("athlete_id"),
            identity.id.as_str(),
        ));
        if let Some(url) = identity.url.as_deref() {
            facts.push(ReviewEvidenceFact::new(
                identity.namespace.to_string(),
                field("profile_url"),
                url,
            ));
        }
    }
    facts
}

/// The row's provider identities in a stable order, so two stores holding the same evidence state it
/// the same way whatever order the observations were appended in.
fn identities(row: &CanonicalAthlete) -> Vec<&SourceIdentity> {
    let mut identities: Vec<&SourceIdentity> = row.source_identities.iter().collect();
    identities.sort_by(|a, b| a.namespace.cmp(&b.namespace).then_with(|| a.id.cmp(&b.id)));
    identities
}

/// The row's grade observations in a stable order: implied class, grade, then the source that
/// observed it.
fn observations(row: &CanonicalAthlete) -> Vec<&ObservedGrade> {
    let mut observations: Vec<&ObservedGrade> = row.observed_grades.iter().collect();
    observations.sort_by(|a, b| {
        a.grad_year()
            .get()
            .cmp(&b.grad_year().get())
            .then_with(|| a.grade.get().cmp(&b.grade.get()))
            .then_with(|| a.source.id.cmp(&b.source.id))
    });
    observations
}

/// One observation as the packet states it: what was seen, in which school year, and what class it
/// implies.
fn observation_text(observation: &ObservedGrade) -> String {
    format!(
        "grade {} in {} implies {} (from {})",
        observation.grade,
        observation.school_year.short(),
        observation.grad_year(),
        observation.source.id
    )
}

#[cfg(test)]
#[path = "athlete_packet_tests.rs"]
mod tests;
