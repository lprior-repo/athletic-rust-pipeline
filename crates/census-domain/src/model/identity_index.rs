use super::identity_decision::person_key;
use super::{
    athlete_identity_digest, AthleteCandidateId, AthleteIndexId, CanonicalAthlete, CaseEvidence,
    EvidenceMethod, Gender, GradYear, IdentityMember, IdentityStatus, ATHLETE_IDENTITY_POLICY,
    MEMBER_SET_LABEL,
};
use std::collections::{BTreeMap, BTreeSet};

type PersonKey = (&'static str, u64);

#[derive(Debug, thiserror::Error)]
pub enum IdentityError {
    #[error("cannot encode athlete identity evidence: {0}")]
    Evidence(#[from] serde_json::Error),
    #[error("identity context references missing subject {0}")]
    UnknownSubject(String),
    #[error("identity context repeats source subject {0}")]
    DuplicateSubject(String),
    #[error("identity review records repeat id {0}")]
    DuplicateReviewRecord(String),
    #[error("identity decision count overflow")]
    CounterOverflow,
    #[error("identity aliases violate the strictly decreasing parent invariant")]
    AliasCycle,
}

pub(super) struct IdentityFact {
    pub(super) digest: String,
    pub(super) grad_year: GradYear,
    pub(super) gender: Gender,
    pub(super) conflicted: bool,
    pub(super) parsed: bool,
    pub(super) source_bound: bool,
    pub(super) status: IdentityStatus,
    pub(super) authorized: bool,
    bucket: AthleteIndexId,
    primary: Option<PersonKey>,
    links: Vec<PersonKey>,
}

impl IdentityFact {
    fn of(athlete: &CanonicalAthlete) -> Result<Self, IdentityError> {
        let primary = person_key(&athlete.source);
        let mut links: Vec<_> = athlete
            .source_links
            .iter()
            .filter_map(person_key)
            .filter(|key| Some(*key) != primary)
            .collect();
        links.sort_unstable();
        links.dedup();
        let alias_conflict = links.windows(2).any(|pair| pair[0].0 == pair[1].0)
            || primary.is_some_and(|key| links.iter().any(|link| link.0 == key.0));
        let parsed = athlete.evidence.iter().any(|evidence| {
            evidence.method == EvidenceMethod::Parsed
                && evidence
                    .source
                    .url
                    .as_ref()
                    .is_some_and(|url| !url.is_empty())
        });
        Ok(Self {
            digest: athlete_identity_digest(athlete)?,
            grad_year: athlete.grad_year,
            gender: athlete.gender,
            conflicted: alias_conflict
                || !athlete.retained_conflicts.is_empty()
                || athlete
                    .observed_grades
                    .iter()
                    .any(|grade| grade.grad_year() != athlete.grad_year),
            parsed,
            source_bound: primary.is_some() && parsed,
            status: IdentityStatus::Unverified,
            authorized: false,
            bucket: athlete.candidate_key().index_id(),
            primary,
            links,
        })
    }

    pub(super) fn keys(&self) -> impl Iterator<Item = PersonKey> + '_ {
        self.primary
            .iter()
            .copied()
            .chain(self.links.iter().copied())
    }

    pub(super) fn shares_primary(&self, other: &Self) -> bool {
        self.primary.is_some_and(|key| other.primary == Some(key))
    }
}

#[derive(Default)]
pub struct AthleteIdentityIndex {
    pub(super) facts: BTreeMap<AthleteCandidateId, IdentityFact>,
    by_candidate: BTreeMap<AthleteIndexId, BTreeSet<AthleteCandidateId>>,
    by_person: BTreeMap<PersonKey, BTreeSet<AthleteCandidateId>>,
}

impl AthleteIdentityIndex {
    pub fn observe(&mut self, athlete: &CanonicalAthlete) -> Result<(), IdentityError> {
        let id: AthleteCandidateId = athlete.id.cast();
        if self.facts.contains_key(id.as_str()) {
            return Err(IdentityError::DuplicateSubject(id.to_string()));
        }
        let fact = IdentityFact::of(athlete)?;
        self.by_candidate
            .entry(fact.bucket.clone())
            .or_default()
            .insert(id.clone());
        for key in fact.keys() {
            self.by_person.entry(key).or_default().insert(id.clone());
        }
        self.facts.insert(id, fact);
        Ok(())
    }

    pub(super) fn into_facts(self) -> BTreeMap<AthleteCandidateId, IdentityFact> {
        self.facts
    }

    pub fn subjects(&self) -> impl Iterator<Item = &AthleteCandidateId> {
        self.facts.keys()
    }

    pub fn candidate_groups(&self) -> impl Iterator<Item = &BTreeSet<AthleteCandidateId>> {
        self.by_candidate.values().filter(|group| group.len() > 1)
    }

    pub fn member(&self, id: &AthleteCandidateId) -> Option<IdentityMember> {
        self.facts.get(id.as_str()).map(|fact| IdentityMember {
            subject: id.clone(),
            evidence_digest: fact.digest.clone(),
        })
    }

    pub fn isolated_source(&self, id: &AthleteCandidateId) -> bool {
        let Some(fact) = self.facts.get(id.as_str()) else {
            return false;
        };
        fact.source_bound
            && !fact.conflicted
            && self
                .by_candidate
                .get(&fact.bucket)
                .is_some_and(|members| members.len() == 1)
            && fact.keys().all(|key| {
                self.by_person
                    .get(&key)
                    .is_some_and(|members| members.len() == 1)
            })
    }

    pub fn case_evidence(
        &self,
        subject: &str,
        detail: &str,
        members: &[AthleteCandidateId],
    ) -> Result<CaseEvidence, IdentityError> {
        let related = self.context_ids(members)?;
        let fingerprints: Vec<_> = related
            .iter()
            .map(|id| {
                self.facts
                    .get(id.as_str())
                    .map(|fact| fact.digest.as_str())
                    .ok_or_else(|| IdentityError::UnknownSubject(id.to_string()))
            })
            .collect::<Result<_, _>>()?;
        let policy = format!("identity-policy:{ATHLETE_IDENTITY_POLICY}");
        Ok(CaseEvidence::of(
            [policy.as_str(), subject, detail]
                .into_iter()
                .chain(fingerprints),
        )
        .with_members(MEMBER_SET_LABEL, members.iter().cloned())
        .with_members("identity_context", related.into_iter().cloned()))
    }

    fn context_ids<'a>(
        &'a self,
        members: &[AthleteCandidateId],
    ) -> Result<BTreeSet<&'a AthleteCandidateId>, IdentityError> {
        let mut seen = BTreeSet::new();
        let mut queue = Vec::new();
        for member in members {
            let (id, _) = self
                .facts
                .get_key_value(member.as_str())
                .ok_or_else(|| IdentityError::UnknownSubject(member.to_string()))?;
            if seen.insert(id) {
                queue.push(id);
            }
        }
        let mut buckets = BTreeSet::new();
        let mut providers = BTreeSet::new();
        for position in 0..self.facts.len() {
            let Some(id) = queue.get(position).copied() else {
                break;
            };
            let fact = self
                .facts
                .get(id.as_str())
                .ok_or_else(|| IdentityError::UnknownSubject(id.to_string()))?;
            if buckets.insert(&fact.bucket) {
                if let Some(group) = self.by_candidate.get(&fact.bucket) {
                    extend_context(group, &mut seen, &mut queue);
                }
            }
            for key in fact.keys() {
                if providers.insert(key) {
                    if let Some(group) = self.by_person.get(&key) {
                        extend_context(group, &mut seen, &mut queue);
                    }
                }
            }
        }
        Ok(seen)
    }
}

fn extend_context<'a>(
    group: &'a BTreeSet<AthleteCandidateId>,
    seen: &mut BTreeSet<&'a AthleteCandidateId>,
    queue: &mut Vec<&'a AthleteCandidateId>,
) {
    for member in group {
        if seen.insert(member) {
            queue.push(member);
        }
    }
}
