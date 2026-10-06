use std::collections::{BTreeMap, HashMap};

use census_domain::model::{
    AthleteCandidateId, CanonicalAthlete, ObservedGrade, ReviewCase, ReviewEvidenceFact,
    ReviewPacket, SourceIdentity,
};
use census_store::{StoreError, StoreResult};

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

    pub(super) fn compare_members(
        &self,
        subject_id: &str,
        members: &[AthleteCandidateId],
    ) -> Option<(&CanonicalAthlete, &CanonicalAthlete, Vec<String>)> {
        let subject = self.rows.get(subject_id)?;
        let (other_id, candidates) = match members {
            [] => {
                let idx = self.group_index.get(subject_id)?;
                let group = self.groups.get(*idx)?;
                let other_id = group.iter().find(|id| id.as_str() != subject_id)?;
                (other_id.as_str(), group.clone())
            }
            [first, second] if first != second => {
                let other_id = match (first.as_str(), second.as_str()) {
                    (first, second) if first == subject_id => second,
                    (first, second) if second == subject_id => first,
                    _ => return None,
                };
                let mut candidates = vec![first.as_str().to_string(), second.as_str().to_string()];
                candidates.sort();
                (other_id, candidates)
            }
            _ => return None,
        };
        let other = self.rows.get(other_id)?;
        Some((subject, other, candidates))
    }
}

pub(super) fn packet(
    case: &ReviewCase,
    subject: &CanonicalAthlete,
    other: &CanonicalAthlete,
    group: &[String],
) -> StoreResult<ReviewPacket> {
    let mut packet = ReviewPacket::new(case.subject_id.clone(), case.subject.clone())
        .with_case(case_fact(case))
        .with_evidence(fact("question", "are the two sides the same athlete?"))
        .with_evidence(fact("answer_field", IDENTITY_FIELD))
        .with_evidence(fact("answer_values", "same_person | different_person"))
        .with_evidence(fact("candidate_ids", &group.join(", ")));
    let sides = side_facts(Side::Subject, subject)?
        .into_iter()
        .chain(side_facts(Side::Other, other)?);
    for evidence in sides {
        packet = packet.with_evidence(evidence);
    }
    for flag in flags(subject, other) {
        packet = packet.with_evidence(flag.evidence());
    }
    Ok(packet)
}

pub(super) fn canonical_athlete(row: &CanonicalAthlete) -> StoreResult<CanonicalAthlete> {
    let mut canonical = row.clone();
    ordered(&mut canonical.known_names)?;
    ordered(&mut canonical.sports)?;
    ordered(&mut canonical.observed_grades)?;
    ordered(&mut canonical.published_graduations)?;
    ordered(&mut canonical.public_profile_urls)?;
    ordered(&mut canonical.source_links)?;
    ordered(&mut canonical.evidence)?;
    ordered(&mut canonical.retained_conflicts)?;
    Ok(canonical)
}

fn canonical_athlete_json(row: &CanonicalAthlete) -> StoreResult<String> {
    serde_json::to_string(&canonical_athlete(row)?).map_err(|source| StoreError::Json {
        detail: format!("serializing canonical athlete {}", row.id),
        source,
    })
}

fn ordered<T: serde::Serialize>(values: &mut Vec<T>) -> StoreResult<()> {
    let mut keyed = Vec::with_capacity(values.len());
    for value in values.drain(..) {
        let key = serde_json::to_string(&value).map_err(|source| StoreError::Json {
            detail: "ordering a packet fact set".to_string(),
            source,
        })?;
        keyed.push((key, value));
    }
    keyed.sort_by(|left, right| left.0.cmp(&right.0));
    values.extend(keyed.into_iter().map(|(_, value)| value));
    Ok(())
}

fn side_facts(side: Side, row: &CanonicalAthlete) -> StoreResult<Vec<ReviewEvidenceFact>> {
    let prefix = side.prefix();
    let field = |name: &str| format!("{prefix}_{name}");
    let canonical = canonical_athlete_json(row)?;
    let mut facts = vec![
        fact(&field("id"), row.id.as_str()),
        fact(&field("name"), &row.canonical_name),
        fact(&field("school"), row.school.as_str()),
        fact(&field("grad_year"), &row.grad_year.to_string()),
        fact(&field("gender"), row.gender.stable_key()),
        fact(&field("canonical_athlete"), &canonical),
    ];
    for observation in observations(row) {
        facts.push(fact(
            &field("grad_evidence"),
            &observation_text(observation),
        ));
    }
    for observation in graduations(row) {
        facts.push(fact(
            &field("grad_evidence"),
            &format!(
                "published graduation {} (from {} at {})",
                observation.grad_year,
                observation.source.id,
                observation
                    .source
                    .url
                    .as_deref()
                    .map_or("no source URL", |value| value)
            ),
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
    Ok(facts)
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
            .map(|gy| gy.get())
            .cmp(&b.grad_year().map(|gy| gy.get()))
            .then_with(|| a.grade.get().cmp(&b.grade.get()))
            .then_with(|| a.source.id.cmp(&b.source.id))
    });
    observations
}

fn graduations(row: &CanonicalAthlete) -> Vec<&census_domain::model::PublishedGraduation> {
    let mut observations: Vec<_> = row.published_graduations.iter().collect();
    observations.sort_by(|a, b| {
        a.grad_year
            .get()
            .cmp(&b.grad_year.get())
            .then_with(|| a.source.id.cmp(&b.source.id))
            .then_with(|| a.source.url.cmp(&b.source.url))
    });
    observations
}

fn observation_text(observation: &ObservedGrade) -> String {
    let grad_year_text = match observation.grad_year() {
        Some(year) => year.to_string(),
        None => "(outside supported cohort range)".to_string(),
    };
    format!(
        "grade {} in {} implies {} (from {})",
        observation.grade,
        observation.school_year.short(),
        grad_year_text,
        observation.source.id
    )
}

#[cfg(test)]
#[path = "athlete_packet_tests.rs"]
mod tests;
