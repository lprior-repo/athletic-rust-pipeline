use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{normalize_name, CanonicalAthlete, ReviewEvidenceFact, SourceNamespace};

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
    let (implied_a, implied_b) = (implied_years(a), implied_years(b));
    if !implied_a.is_empty() && !implied_b.is_empty() && implied_a != implied_b {
        flags.push(Flag {
            kind: FlagKind::GradYearEvidenceDiffers,
            detail: format!(
                "grade observations imply {} vs {}",
                years(&implied_a),
                years(&implied_b)
            ),
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

type NamespaceIds<'a> = BTreeMap<&'a SourceNamespace, BTreeSet<&'a str>>;

fn namespace_ids(row: &CanonicalAthlete) -> NamespaceIds<'_> {
    let mut ids: NamespaceIds<'_> = BTreeMap::new();
    for identity in row.identities() {
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

fn implied_years(row: &CanonicalAthlete) -> BTreeSet<i16> {
    row.observed_grades
        .iter()
        .map(|observation| observation.grad_year().get())
        .collect()
}

fn years(years: &BTreeSet<i16>) -> String {
    years
        .iter()
        .map(i16::to_string)
        .collect::<Vec<String>>()
        .join("/")
}
