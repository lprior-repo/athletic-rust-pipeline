use std::sync::Arc;

use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, Grade, ObservedGrade,
    RetainedConflict, ReviewCase, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    ATHLETE_IDENTITY_FAMILY,
};
use census_store::{Store, Table};
use serde_json::Value;

pub(super) struct Fixture {
    pub(super) store: Arc<Store>,
    pub(super) case: ReviewCase,
    _dir: tempfile::TempDir,
}

impl Fixture {
    pub(super) fn new(rows: &[CanonicalAthlete]) -> Self {
        let dir = tempfile::tempdir().expect("isolated identity store");
        let store = Arc::new(Store::open(dir.path()).expect("store"));
        store
            .append_many(Table::Athletes, rows)
            .expect("source rows");
        let mut case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            rows[0].id.as_str(),
            "Jordan Smith",
            "two named source-backed candidates",
        );
        case.member_ids = rows.iter().map(|row| row.id.cast()).collect();
        store
            .replace_many(Table::ReviewCases, std::slice::from_ref(&case))
            .expect("identity case");
        Self {
            store,
            case,
            _dir: dir,
        }
    }
}

pub(super) fn rows() -> Vec<CanonicalAthlete> {
    let school = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Madison West",
        "madison west",
    )
    .0
    .id;
    ["1001", "1002"]
        .into_iter()
        .map(|id| {
            let mut row = CanonicalAthlete::new(
                &school,
                "Jordan Smith",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, id),
            );
            row.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "77"));
            row.observed_grades.push(ObservedGrade {
                grade: Grade::new(11).expect("supported grade"),
                school_year: SchoolYear::new(2025).expect("supported year"),
                source: SourceRef::id("public-roster"),
            });
            row.evidence.push(Evidence::parsed(
                SourceRef::id("public-roster"),
                "2026-10-01",
            ));
            row
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub(super) enum Change {
    GradeUrl,
    PrimaryOwnership,
    EvidenceUrl,
    RetainedConflict,
}

impl Change {
    pub(super) fn apply(self, row: &mut CanonicalAthlete) {
        match self {
            Self::GradeUrl => {
                row.observed_grades[0].source.url =
                    Some("https://source.example/grade".to_string());
            }
            Self::PrimaryOwnership => {
                let primary = row.source.take().expect("primary owner");
                row.source = Some(row.source_links.remove(0));
                row.source_links.push(primary);
            }
            Self::EvidenceUrl => {
                row.evidence[0].source.url = Some("https://source.example/parsed-row".to_string());
            }
            Self::RetainedConflict => {
                row.retained_conflicts.push(RetainedConflict::new(
                    "Source ownership conflict",
                    row.id.as_str(),
                    "Jordan Smith",
                    "https://source.example/conflict retains incompatible source ownership",
                ));
            }
        }
    }
}

pub(super) fn canonical_side(request: &Value, side: &str) -> CanonicalAthlete {
    let content = request["messages"][1]["content"]
        .as_str()
        .expect("request content");
    let prefix = format!("- census: {side}_canonical_athlete = ");
    let raw = content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .expect("complete source-attributed canonical row");
    serde_json::from_str(raw).expect("canonical athlete evidence")
}

pub(super) fn assert_preserved(store: &Store, expected: &[CanonicalAthlete]) {
    let actual = store
        .scan::<CanonicalAthlete>(Table::Athletes)
        .expect("source rows");
    assert_eq!(actual.len(), expected.len());
    assert!(expected.iter().all(|row| actual.contains(row)));
    assert!(store
        .scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)
        .expect("canonical aliases")
        .is_empty());
}
