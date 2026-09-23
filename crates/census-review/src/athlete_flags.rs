//! The deterministic signals the athlete-identity question starts from.
//!
//! A case was retained because the merge kept two rows apart under one key; what a model is asked is
//! only what those rows' own fields leave open. So every signal is computed here, from the canonical
//! rows alone, and stated in the packet as a flag: the model is never asked to notice a contradiction
//! itself, and never asked to weigh one. A flag is the store talking about its own rows, not a
//! confidence.
//!
//! The key rule lives here, beside the agreement flag it produces, because the two are one question
//! asked twice: [`key`] is what the merge grouped the rows by, and [`FlagKind::NameSchoolCohortAgree`]
//! is that grouping stated as evidence.

use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{normalize_name, CanonicalAthlete, ReviewEvidenceFact, SourceNamespace};

use super::packets::fact;

/// The key the merge keeps two athlete rows apart under: school, normalized name, graduating class.
pub type IdentityKey = (String, String, i16);

/// The group key for one row: the rule the conflict family groups the retained rows by.
pub fn key(row: &CanonicalAthlete) -> IdentityKey {
    (
        row.school.as_str().to_string(),
        normalize_name(&row.canonical_name),
        row.grad_year.get(),
    )
}

/// A deterministic signal about the two rows a case compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlagKind {
    /// Both rows carry one provider object — the same namespace and the same source athlete id — so
    /// the provider says one athlete while the store holds two canonical ids.
    SharedSourceIdentity,
    /// Both rows are known to one provider by *different* athlete objects: the provider issued two,
    /// so its own ids do not say the two rows are one athlete.
    DistinctProviderObjects,
    /// The rows' canonical graduating classes differ.
    GradYearDiffers,
    /// The classes the rows' own grade observations imply differ.
    GradYearEvidenceDiffers,
    /// The rows' genders differ, which is the component their canonical ids differ in.
    GenderDiffers,
    /// The rows agree on the key the finding was retained under: one name, one school, one class.
    NameSchoolCohortAgree,
}

impl FlagKind {
    /// The slug the packet states the flag under.
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

/// One flag: the kind, and the fields it was read off.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flag {
    pub kind: FlagKind,
    pub detail: String,
}

impl Flag {
    /// The evidence fact that carries the flag: the kind, then what it was computed from.
    pub(super) fn evidence(&self) -> ReviewEvidenceFact {
        fact("flag", &format!("{}: {}", self.kind.slug(), self.detail))
    }
}

/// Every flag the two rows give, the disagreements first and the agreement last.
///
/// Each flag is computed from the rows' own fields, never from the fact that they were paired: if a
/// later rule paired two rows that do not share the key, the agreement flag would be absent rather
/// than assumed.
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

/// The provider objects one row is known by, one entry per namespace, ids in sorted order.
type NamespaceIds<'a> = BTreeMap<&'a SourceNamespace, BTreeSet<&'a str>>;

/// Index a row's identities by namespace, so one provider can be read off both rows at once.
fn namespace_ids(row: &CanonicalAthlete) -> NamespaceIds<'_> {
    let mut ids: NamespaceIds<'_> = BTreeMap::new();
    for identity in &row.source_identities {
        ids.entry(&identity.namespace)
            .or_default()
            .insert(identity.id.as_str());
    }
    ids
}

/// The `(namespace, source athlete id)` pairs both rows carry: one provider object, two canonical
/// ids, so the provider's own id cannot tell the two rows apart.
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

/// The namespaces both rows are known in where no id is shared: the provider issued two athlete
/// objects, so it is not saying the two rows are one athlete.
///
/// This is the other half of [`FlagKind::SharedSourceIdentity`], and the reason a provider's ids are
/// reported rather than counted: one namespace can issue two objects for one athlete (a duplicate
/// entry), so the model is told what the provider did and never told what it means.
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

/// A set of ids as one detail: comma-separated, in sorted order.
fn ids_of(ids: &BTreeSet<&str>) -> String {
    ids.iter().copied().collect::<Vec<&str>>().join(", ")
}

/// The graduating classes the row's own grade observations imply.
fn implied_years(row: &CanonicalAthlete) -> BTreeSet<i16> {
    row.observed_grades
        .iter()
        .map(|observation| observation.grad_year().get())
        .collect()
}

/// A set of years as the packet states it.
fn years(years: &BTreeSet<i16>) -> String {
    years
        .iter()
        .map(i16::to_string)
        .collect::<Vec<String>>()
        .join("/")
}
