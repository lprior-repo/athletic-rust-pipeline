use std::collections::{BTreeMap, BTreeSet};

use census_domain::model::{
    AthleteCandidateId, CanonicalAthlete, CanonicalSchool, CaseEvidence, ReviewCase,
    ReviewVerdictKind, ReviewVerdictRecord, SchoolId, SourceNamespace, ATHLETE_IDENTITY_FAMILY,
    MEMBER_SET_LABEL,
};
use census_store::{Store, StoreResult, Table};

use super::athlete_clusters::RULE_REVIEWER;
use super::cohort_evidence::CohortEvidence;
use crate::athlete_verdict::HardContradiction;
use crate::families::IDENTITY_FIELD;

type SchoolNames = BTreeMap<SchoolId, String>;

#[derive(Debug)]
struct Row<'a> {
    athlete: &'a CanonicalAthlete,
    school: &'a str,
    cohort: CohortEvidence<'a>,
}

impl<'a> Row<'a> {
    fn of(athlete: &'a CanonicalAthlete, schools: &'a SchoolNames) -> Self {
        Self {
            athlete,
            school: schools
                .get(&athlete.school)
                .map_or(athlete.school.as_str(), String::as_str),
            cohort: CohortEvidence::of(&athlete.observed_grades),
        }
    }

    fn line(&self) -> String {
        format!(
            "{} at {} (class {}, {:?}; {})",
            self.athlete.canonical_name,
            self.school,
            self.athlete.grad_year,
            self.athlete.gender,
            self.cohort,
        )
    }

    fn agrees_with(&self, other: &Self) -> bool {
        self.athlete.canonical_name == other.athlete.canonical_name
            && self.athlete.grad_year == other.athlete.grad_year
            && self.athlete.gender == other.athlete.gender
            && !self.cohort.conflicts_with(&other.cohort)
    }
}

#[derive(Debug, Default)]
pub(super) struct Observed {
    schools: SchoolNames,
    pub(super) rows: Vec<CanonicalAthlete>,
}

pub(super) struct Findings<'a> {
    pub(super) objects: usize,
    pub(super) spans: Vec<Span<'a>>,
    pub(super) aliases: Vec<Alias<'a>>,
}

impl Observed {
    pub(super) fn read(store: &Store) -> StoreResult<Self> {
        let snapshot = store.snapshot();
        let mut observed = Self::default();
        snapshot.for_each_merged::<CanonicalSchool>(Table::Schools, |school| {
            observed.schools.insert(school.id, school.name);
            Ok(())
        })?;
        observed.rows = snapshot.athletes()?;
        Ok(observed)
    }

    pub(super) fn findings(&self) -> Findings<'_> {
        let mut objects: BTreeMap<(&SourceNamespace, &str), BTreeMap<&str, Row<'_>>> =
            BTreeMap::new();
        let mut aliases = Vec::new();
        for athlete in &self.rows {
            let mut named: BTreeMap<&SourceNamespace, BTreeSet<&str>> = BTreeMap::new();
            for identity in athlete.identities() {
                objects
                    .entry((&identity.namespace, identity.id.as_str()))
                    .or_default()
                    .entry(athlete.id.as_str())
                    .or_insert_with(|| Row::of(athlete, &self.schools));
                named
                    .entry(&identity.namespace)
                    .or_default()
                    .insert(identity.id.as_str());
            }
            aliases.extend(named.into_iter().filter(|(_, ids)| ids.len() > 1).map(
                |(namespace, objects)| Alias {
                    row: Row::of(athlete, &self.schools),
                    namespace,
                    objects,
                },
            ));
        }
        let count = objects.len();
        let spans = objects
            .into_iter()
            .filter(|(_, rows)| rows.len() > 1)
            .map(|((namespace, object), rows)| Span {
                namespace,
                object,
                rows: rows.into_values().collect(),
            })
            .collect();
        Findings {
            objects: count,
            spans,
            aliases,
        }
    }
}

pub(super) struct Span<'a> {
    namespace: &'a SourceNamespace,
    object: &'a str,
    rows: Vec<Row<'a>>,
}

impl Span<'_> {
    pub(super) fn agrees(&self) -> bool {
        let mut rows = self.rows.iter();
        let Some(first) = rows.next() else {
            return false;
        };
        rows.all(|row| first.agrees_with(row))
    }

    pub(super) fn hard_contradiction(&self) -> Option<HardContradiction> {
        let first = self.rows.first()?;
        if self
            .rows
            .iter()
            .skip(1)
            .any(|row| first.cohort.conflicts_with(&row.cohort))
        {
            return Some(HardContradiction::GradYearEvidenceDiffers);
        }
        self.rows
            .iter()
            .skip(1)
            .any(|row| first.athlete.gender != row.athlete.gender)
            .then_some(HardContradiction::GenderDiffers)
    }

    pub(super) fn case(&self) -> Option<ReviewCase> {
        let first = self.rows.first()?;
        let lines: Vec<String> = self.rows.iter().map(Row::line).collect();
        let reason = match (self.hard_contradiction(), self.agrees()) {
            (Some(flag), _) => format!(
                "The packet carries hard contradiction `{}`; the object does not settle that they are one athlete.",
                flag.slug()
            ),
            (None, true) => "The rows share one provider-owned athlete object and agree on name, cohort evidence and gender; this establishes identity, not a transfer chronology.".to_string(),
            (None, false) => "The rows disagree on name, class or gender, so the object does not settle that they are one athlete.".to_string(),
        };
        let subject = first.line();
        let detail = format!(
            "{} {} is one provider object on {} canonical rows: {}. {}",
            self.namespace,
            self.object,
            self.rows.len(),
            lines.join("; "),
            reason,
        );
        let members: Vec<AthleteCandidateId> =
            self.rows.iter().map(|row| row.athlete.id.cast()).collect();
        let evidence = CaseEvidence::of([subject.as_str(), detail.as_str()])
            .with_members(MEMBER_SET_LABEL, members.iter().cloned());
        let mut case = ReviewCase::pending_with_evidence(
            ATHLETE_IDENTITY_FAMILY,
            first.athlete.id.as_str(),
            subject,
            detail,
            evidence,
        );
        case.member_ids = members;
        Some(case)
    }

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
                self.namespace, self.object, self.rows.len(),
            ),
            reviewer: RULE_REVIEWER.to_string(),
            observed_at: observed_at.to_string(),
            member_ids: case.member_ids.clone(),
        }
    }
}

pub(super) struct Alias<'a> {
    row: Row<'a>,
    namespace: &'a SourceNamespace,
    objects: BTreeSet<&'a str>,
}

impl Alias<'_> {
    pub(super) fn case(&self) -> Option<ReviewCase> {
        let first = self.objects.first()?;
        let detail = format!(
            "{} names {} objects of one row: {}. The provider never said they are one athlete, so which of the two the row is cannot be read off the store.",
            self.row.line(), self.objects.len(), self.objects.iter().copied().collect::<Vec<_>>().join(", "),
        );
        let mut case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            self.row.athlete.id.as_str(),
            format!(
                "{} ({} {})",
                self.row.athlete.canonical_name, self.namespace, first
            ),
            detail,
        );
        case.member_ids.push(self.row.athlete.id.cast());
        Some(case)
    }
}
