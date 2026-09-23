//! The store as one reconciliation pass observes it: every canonical athlete row reduced to the
//! facts the comparisons need, the provider objects those rows are known by, and the two findings
//! that read off the two directions of that relation.
//!
//! Split from the pass itself so each file stays inside the source budget — this file is the model,
//! and `athlete_clusters` is the decision and the write.

use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Gender, ReviewCase, ReviewVerdictKind, ReviewVerdictRecord,
    SourceNamespace, ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, StoreResult, Table};

use super::athlete_clusters::RULE_REVIEWER;
use crate::families::IDENTITY_FIELD;

/// One canonical athlete row, reduced to the facts this pass compares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Row {
    id: String,
    name: String,
    school: String,
    grad_year: i16,
    gender: Gender,
}

impl Row {
    fn of(row: &CanonicalAthlete, schools: &BTreeMap<String, String>) -> Self {
        let stored = row.school.as_str();
        Self {
            id: row.id.as_str().to_string(),
            name: row.canonical_name.clone(),
            // The case is read by an operator and by a model, so a site is named the way the store
            // names it; an unmapped school falls back to the id that is the only thing the store
            // holds about it.
            school: schools
                .get(stored)
                .cloned()
                .unwrap_or_else(|| stored.to_string()),
            grad_year: row.grad_year.get(),
            gender: row.gender,
        }
    }

    /// The line the store names this row by.
    fn line(&self) -> String {
        format!(
            "{} at {} (class {}, {:?})",
            self.name, self.school, self.grad_year, self.gender
        )
    }

    /// Whether two rows say the same athlete: a transfer moves the school and changes nothing else.
    fn agrees_with(&self, other: &Self) -> bool {
        self.name == other.name && self.grad_year == other.grad_year && self.gender == other.gender
    }
}

/// The store read this pass makes: each row's objects, and the rows each object is named by.
#[derive(Debug, Default)]
pub(super) struct Observed {
    schools: BTreeMap<String, String>,
    pub(super) rows: BTreeMap<String, Row>,
    objects_of: BTreeMap<(String, SourceNamespace), BTreeSet<String>>,
    pub(super) by_object: BTreeMap<(SourceNamespace, String), BTreeSet<String>>,
}

impl Observed {
    pub(super) fn read(store: &Store) -> StoreResult<Self> {
        let mut observed = Self::default();
        // The schools first: a finding names the sites it spans, and a site is named the way the
        // school table names it rather than by the id only the store can read.
        store.for_each_merged::<CanonicalSchool>(Table::Schools, |school| {
            observed
                .schools
                .insert(school.id.as_str().to_string(), school.name.clone());
            Ok(())
        })?;
        store.for_each_merged::<CanonicalAthlete>(Table::Athletes, |row| {
            observed.absorb(&row);
            Ok(())
        })?;
        Ok(observed)
    }

    fn absorb(&mut self, row: &CanonicalAthlete) {
        let id = row.id.as_str().to_string();
        for identity in &row.source_identities {
            self.objects_of
                .entry((id.clone(), identity.namespace.clone()))
                .or_default()
                .insert(identity.id.clone());
            self.by_object
                .entry((identity.namespace.clone(), identity.id.clone()))
                .or_default()
                .insert(id.clone());
        }
        self.rows.insert(id, Row::of(row, &self.schools));
    }

    /// Every provider object more than one canonical row is known by.
    pub(super) fn spans(&self) -> Vec<Span> {
        self.by_object
            .iter()
            .filter(|(_, ids)| ids.len() > 1)
            .filter_map(|((namespace, object), ids)| self.span(namespace, object, ids))
            .collect()
    }

    fn span(
        &self,
        namespace: &SourceNamespace,
        object: &str,
        ids: &BTreeSet<String>,
    ) -> Option<Span> {
        let rows: Vec<Row> = ids
            .iter()
            .filter_map(|id| self.rows.get(id).cloned())
            .collect();
        (rows.len() > 1).then(|| Span {
            namespace: namespace.clone(),
            object: object.to_string(),
            rows,
        })
    }

    /// Every row carrying more than one object of one namespace.
    pub(super) fn aliases(&self) -> Vec<Alias> {
        self.objects_of
            .iter()
            .filter(|(_, objects)| objects.len() > 1)
            .filter_map(|((id, namespace), objects)| {
                Some((self.rows.get(id)?.clone(), namespace, objects))
            })
            .map(|(row, namespace, objects)| Alias {
                row,
                namespace: namespace.clone(),
                objects: objects.iter().cloned().collect(),
            })
            .collect()
    }
}

/// One provider object more than one canonical row is known by.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Span {
    namespace: SourceNamespace,
    object: String,
    rows: Vec<Row>,
}

impl Span {
    /// Whether every row the object names says the same athlete.
    pub(super) fn agrees(&self) -> bool {
        let mut rows = self.rows.iter();
        let Some(first) = rows.next() else {
            return false;
        };
        rows.all(|row| first.agrees_with(row))
    }

    pub(super) fn case(&self) -> Option<ReviewCase> {
        let first = self.rows.first()?;
        let lines: Vec<String> = self.rows.iter().map(Row::line).collect();
        let detail = format!(
            "{} {} is one provider object on {} canonical rows: {}. {}",
            self.namespace,
            self.object,
            self.rows.len(),
            lines.join("; "),
            if self.agrees() {
                "The rows agree on name, class and gender, so the differing school is a transfer rather than a second athlete."
            } else {
                "The rows disagree on name, class or gender, so the object does not settle that they are one athlete."
            }
        );
        Some(ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            first.id.as_str(),
            first.line(),
            detail,
        ))
    }

    /// The decision the agreement rule states, as the athlete family's own answer.
    pub(super) fn verdict(&self, case: &ReviewCase, observed_at: &str) -> ReviewVerdictRecord {
        ReviewVerdictRecord {
            id: case.id.clone(),
            case_id: case.id.clone(),
            subject_id: case.subject_id.clone(),
            family: ATHLETE_IDENTITY_FAMILY.to_string(),
            kind: ReviewVerdictKind::ValueProposed.slug().to_string(),
            field: IDENTITY_FIELD.to_string(),
            value: "same_person".to_string(),
            accepted: true,
            confidence: 100,
            rationale: format!(
                "{} {} is one provider object, and the {} rows it is known by agree on name, class and gender",
                self.namespace,
                self.object,
                self.rows.len()
            ),
            reviewer: RULE_REVIEWER.to_string(),
            observed_at: observed_at.to_string(),
        }
    }
}

/// One canonical row carrying more than one object of one namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Alias {
    row: Row,
    namespace: SourceNamespace,
    objects: Vec<String>,
}

impl Alias {
    pub(super) fn case(&self) -> Option<ReviewCase> {
        let first = self.objects.first()?;
        let detail = format!(
            "{} names {} objects of one row: {}. The provider never said they are one athlete, so which of the two the row is cannot be read off the store.",
            self.row.line(),
            self.objects.len(),
            self.objects.join(", ")
        );
        Some(ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            self.row.id.as_str(),
            format!("{} ({} {})", self.row.name, self.namespace, first),
            detail,
        ))
    }
}
