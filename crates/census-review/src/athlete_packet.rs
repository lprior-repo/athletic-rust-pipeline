use std::collections::{BTreeMap, HashMap};

use census_domain::model::{
    CanonicalAthlete, ObservedGrade, ReviewCase, ReviewEvidenceFact, ReviewPacket, SourceIdentity,
};

use super::athlete_flags::{flags, key, IdentityKey};
use super::families::IDENTITY_FIELD;
use super::packets::{case_fact, fact, index_by_id};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Side {
    Subject,
    Other,
}

impl Side {
    const fn prefix(self) -> &'static str {
        match self {
            Self::Subject => "side_a",
            Self::Other => "side_b",
        }
    }
}

#[derive(Debug, Default)]
pub(super) struct AthleteIndex {
    rows: HashMap<String, CanonicalAthlete>,
    group_index: HashMap<String, usize>,
    groups: Vec<Vec<String>>,
}

impl AthleteIndex {
    pub(super) fn read(rows: Vec<CanonicalAthlete>) -> Self {
        let mut by_key: BTreeMap<IdentityKey, Vec<String>> = BTreeMap::new();
        for row in &rows {
            by_key.entry(key(row)).or_default().push(row.id.to_string());
        }
        let mut group_index = HashMap::new();
        let mut groups = Vec::new();
        for mut ids in by_key.into_values() {
            if ids.len() < 2 {
                continue;
            }
            ids.sort();
            let idx = groups.len();
            for id in &ids {
                group_index.insert(id.clone(), idx);
            }
            groups.push(ids);
        }
        Self {
            rows: index_by_id(rows, |row| row.id.to_string()),
            group_index,
            groups,
        }
    }

    pub(super) fn compare(
        &self,
        subject_id: &str,
    ) -> Option<(&CanonicalAthlete, &CanonicalAthlete, &[String])> {
        let subject = self.rows.get(subject_id)?;
        let idx = self.group_index.get(subject_id)?;
        let group = self.groups.get(*idx)?;
        let other_id = group.iter().find(|id| id.as_str() != subject_id)?;
        let other = self.rows.get(other_id)?;
        Some((subject, other, group))
    }
}

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

fn side_facts(side: Side, row: &CanonicalAthlete) -> Vec<ReviewEvidenceFact> {
    let prefix = side.prefix();
    let field = |name: &str| format!("{prefix}_{name}");
    let mut facts = vec![
        fact(&field("id"), row.id.as_str()),
        fact(&field("name"), &row.canonical_name),
        fact(&field("school"), row.school.as_str()),
        fact(&field("grad_year"), &row.grad_year.to_string()),
        fact(&field("gender"), row.gender.stable_key()),
    ];
    for observation in observations(row) {
        facts.push(fact(
            &field("grad_evidence"),
            &observation_text(observation),
        ));
    }
    for identity in identities(row) {
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

fn identities(row: &CanonicalAthlete) -> Vec<&SourceIdentity> {
    let mut identities: Vec<&SourceIdentity> = row.identities().collect();
    identities.sort_by(|a, b| a.namespace.cmp(&b.namespace).then_with(|| a.id.cmp(&b.id)));
    identities
}

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
