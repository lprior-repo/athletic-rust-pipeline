use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{
    normalize_name, person_key, CanonicalAthlete, ReviewEvidenceFact, SourceNamespace,
};

use super::cohort_evidence::CohortEvidence;
use super::packets::fact;

pub type IdentityKey = (String, String, i16);

pub fn key(row: &CanonicalAthlete) -> IdentityKey {
    (
        row.school.as_str().to_string(),
        normalize_name(&row.canonical_name),
        row.grad_year.get(),
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagKind {
    SharedSourceIdentity,
    DistinctProviderObjects,
    GradYearDiffers,
    GradYearEvidenceDiffers,
    GenderDiffers,
    RetainedSourceConflict,
    NameSchoolCohortAgree,
}

impl FlagKind {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::SharedSourceIdentity => "shared_source_identity",
            Self::DistinctProviderObjects => "distinct_provider_objects",
            Self::GradYearDiffers => "grad_year_differs",
            Self::GradYearEvidenceDiffers => "grad_year_evidence_differs",
            Self::GenderDiffers => "gender_differs",
            Self::NameSchoolCohortAgree => "name_school_cohort_agree",
            Self::RetainedSourceConflict => "retained_source_conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    pub kind: FlagKind,
    pub detail: String,
}

impl Flag {
    pub(super) fn evidence(&self) -> ReviewEvidenceFact {
        fact("flag", &format!("{}: {}", self.kind.slug(), self.detail))
    }
}

pub fn flags(a: &CanonicalAthlete, b: &CanonicalAthlete) -> Vec<Flag> {
    let (left, right) = (namespace_ids(a), namespace_ids(b));
    let mut flags = Vec::new();
    let shared = shared_source_identities(&left, &right);
    if !shared.is_empty() {
        flags.push(Flag {
            kind: FlagKind::SharedSourceIdentity,
            detail: format!(
                "{} on both {} and {}",
                shared.join(", "),
                a.id.as_str(),
                b.id.as_str()
            ),
        });
    }
    let distinct = distinct_provider_objects(&left, &right);
    if !distinct.is_empty() {
        flags.push(Flag {
            kind: FlagKind::DistinctProviderObjects,
            detail: distinct.join("; "),
        });
    }
    if a.grad_year != b.grad_year {
        flags.push(Flag {
            kind: FlagKind::GradYearDiffers,
            detail: format!("{} vs {}", a.grad_year, b.grad_year),
        });
    }
    flags.extend(retained_source_conflicts(a, b));
    let (implied_a, implied_b) = (CohortEvidence::of(a), CohortEvidence::of(b));
    if a.has_cohort_conflict() || b.has_cohort_conflict() || implied_a.conflicts_with(&implied_b) {
        flags.push(Flag {
            kind: FlagKind::GradYearEvidenceDiffers,
            detail: format!("cohort observations support {implied_a} vs {implied_b}"),
        });
    }
    if a.gender != b.gender {
        flags.push(Flag {
            kind: FlagKind::GenderDiffers,
            detail: format!("{} vs {}", a.gender.stable_key(), b.gender.stable_key()),
        });
    }
    if key(a) == key(b) {
        flags.push(Flag {
            kind: FlagKind::NameSchoolCohortAgree,
            detail: format!(
                "{} at {} in {}",
                normalize_name(&a.canonical_name),
                a.school.as_str(),
                a.grad_year
            ),
        });
    }
    flags
}

fn retained_source_conflicts(a: &CanonicalAthlete, b: &CanonicalAthlete) -> Vec<Flag> {
    [a, b]
        .into_iter()
        .filter(|row| !row.retained_conflicts.is_empty())
        .map(|row| Flag {
            kind: FlagKind::RetainedSourceConflict,
            detail: format!(
                "{} retains source conflicts: {}",
                row.id,
                row.retained_conflicts
                    .iter()
                    .map(|conflict| conflict.id.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })
        .collect()
}

type NamespaceIds<'a> = BTreeMap<&'a SourceNamespace, BTreeSet<&'a str>>;

fn namespace_ids(row: &CanonicalAthlete) -> NamespaceIds<'_> {
    let mut ids: NamespaceIds<'_> = BTreeMap::new();
    for identity in row.identities() {
        if person_key(identity).is_none() {
            continue;
        }
        ids.entry(&identity.namespace)
            .or_default()
            .insert(identity.id.as_str());
    }
    ids
}

fn shared_source_identities(left: &NamespaceIds<'_>, right: &NamespaceIds<'_>) -> Vec<String> {
    let mut shared: BTreeSet<String> = BTreeSet::new();
    for (namespace, ids) in left {
        if let Some(other) = right.get(namespace) {
            for id in ids.intersection(other) {
                shared.insert(format!("{namespace}:{id}"));
            }
        }
    }
    shared.into_iter().collect()
}

fn distinct_provider_objects(left: &NamespaceIds<'_>, right: &NamespaceIds<'_>) -> Vec<String> {
    let mut distinct: Vec<String> = Vec::new();
    for (namespace, ids) in left {
        if let Some(other) = right.get(namespace) {
            if ids.is_disjoint(other) {
                distinct.push(format!("{namespace}: {} vs {}", ids_of(ids), ids_of(other)));
            }
        }
    }
    distinct
}

fn ids_of(ids: &BTreeSet<&str>) -> String {
    ids.iter().copied().collect::<Vec<&str>>().join(", ")
}
