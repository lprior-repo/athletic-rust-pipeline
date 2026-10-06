use std::sync::Arc;

use crate::consensus::tests::support::TestResult;
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
    pub(super) fn new(rows: &[CanonicalAthlete]) -> TestResult<Self> {
        let dir = tempfile::tempdir()?;
        let store = Arc::new(Store::open(dir.path())?);
        store.append_many(Table::Athletes, rows)?;
        let mut case = ReviewCase::pending(
            ATHLETE_IDENTITY_FAMILY,
            rows.first().ok_or("source-backed candidate")?.id.as_str(),
            "Jordan Smith",
            "two named source-backed candidates",
        );
        case.member_ids = rows.iter().map(|row| row.id.cast()).collect();
        store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
        Ok(Self {
            store,
            case,
            _dir: dir,
        })
    }
}

pub(super) fn rows() -> TestResult<Vec<CanonicalAthlete>> {
    let school = CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Madison West",
        "madison west",
        None,
    )
    .0
    .id;
    ["1001", "1002"]
        .into_iter()
        .map(|id| -> TestResult<CanonicalAthlete> {
            let mut row = CanonicalAthlete::new(
                &school,
                "Jordan Smith",
                GradYear::CO2027,
                Gender::Boys,
                SourceIdentity::new(SourceNamespace::MilesplitAthlete, id),
            );
            row.add_identity(SourceIdentity::new(SourceNamespace::TfrrsAthlete, "77"));
            row.observed_grades.push(ObservedGrade {
                grade: Grade::new(11).ok_or("supported grade")?,
                school_year: SchoolYear::new(2025).ok_or("supported year")?,
                source: SourceRef::id("public-roster"),
            });
            row.evidence.push(Evidence::parsed(
                SourceRef::id("public-roster"),
                "2026-10-01",
            ));
            Ok(row)
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
    pub(super) fn apply(self, row: &mut CanonicalAthlete) -> TestResult {
        match self {
            Self::GradeUrl => {
                row.observed_grades[0].source.url =
                    Some("https://source.example/grade".to_string());
            }
            Self::PrimaryOwnership => {
                let primary = row.source.take().ok_or("primary owner")?;
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
        Ok(())
    }
}

pub(super) fn canonical(row: &CanonicalAthlete) -> TestResult<CanonicalAthlete> {
    Ok(crate::athlete_packet::canonical_athlete(row)?)
}

pub(super) fn canonical_side(request: &Value, side: &str) -> TestResult<CanonicalAthlete> {
    let content = request["messages"][1]["content"]
        .as_str()
        .ok_or("request content")?;
    let prefix = format!("- census: {side}_canonical_athlete = ");
    let raw = content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .ok_or("complete source-attributed canonical row")?;
    Ok(serde_json::from_str(raw)?)
}

pub(super) fn assert_preserved(store: &Store, expected: &[CanonicalAthlete]) -> TestResult {
    let actual = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    let actual_len = actual.len();
    let expected_len = expected.len();
    if actual_len != expected_len {
        return Err(format!("source rows: left={actual_len:?} right={expected_len:?}").into());
    }
    if !expected.iter().all(|row| actual.contains(row)) {
        return Err(format!(
            "source rows were not preserved: actual={actual:?} expected={expected:?}"
        )
        .into());
    }
    let aliases = store
        .scan::<census_domain::model::AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    if !aliases.is_empty() {
        return Err(format!("canonical aliases must be empty: {aliases:?}").into());
    }
    Ok(())
}
