use super::{read, TestResult, FAMILIES};
use crate::export::ExportDataset;
use crate::workbook::publication;
use census_domain::model::*;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use std::path::{Path, PathBuf};

const HEADER: [&str; 7] = [
    "performance_id",
    "retained_athlete_id",
    "event_id",
    "reason_category",
    "reason_code",
    "selection_policy_version",
    "input_generation",
];
const REASONS: [&str; 7] = [
    "missing_specification",
    "missing_completion",
    "specification_conflict",
    "incomplete",
    "total_mismatch",
    "points_overflow",
    "conflicting_specification",
];

struct Fixture {
    directory: tempfile::TempDir,
    store: Store,
    options: crate::workbook::Options,
    expected: Vec<Vec<String>>,
}

impl Fixture {
    fn new(with_winner: bool) -> TestResult<Self> {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path().join("store"))?;
        let (school, id) = CanonicalSchool::new(
            UsJurisdiction::Michigan,
            "Combined High",
            "combined high",
            None,
        );
        let source = SourceRef::new("wiaa_results", Some("https://example.test/combined".into()));
        let evidence = Evidence::parsed(source.clone(), "2026-05-04");
        let mut athlete = CanonicalAthlete::new(
            &id,
            "Combined Athlete",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "combined-2027"),
        )?;
        athlete.published_graduations.push(PublishedGraduation {
            grad_year: GradYear::CO2027,
            source,
        });
        athlete.evidence.push(evidence.clone());
        athlete.sports.push(Sport::OutdoorTrack);
        let team = CanonicalTeam {
            id: CanonicalTeam::mint(&id, Sport::OutdoorTrack, Gender::Girls, SchoolYear::DEFAULT),
            school: id,
            sport: Sport::OutdoorTrack,
            gender: Gender::Girls,
            school_year: SchoolYear::DEFAULT,
            level: None,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        };
        store.append(Table::Schools, &school)?;
        store.append(Table::Athletes, &athlete)?;
        store.append(Table::Teams, &team)?;
        let mut expected = Vec::new();
        for reason in REASONS {
            let (event, performance) = append(&store, &athlete, &team, &evidence, reason)?;
            expected.push(vec![
                performance.id.to_string(),
                athlete.id.to_string(),
                event.id.to_string(),
                if reason == "conflicting_specification" {
                    "unresolved_specification"
                } else {
                    "eligibility"
                }
                .into(),
                reason.into(),
            ]);
        }
        if with_winner {
            append(&store, &athlete, &team, &evidence, "eligible")?;
        }
        expected.sort();
        Ok(Self {
            directory,
            store,
            expected,
            options: crate::workbook::Options {
                address_corpus: None,
                grad_year: Some(2027),
                out: None,
                limit: None,
                scope: crate::report::Scope::AllSources,
                jurisdictions: vec![UsJurisdiction::Michigan],
                school_year: SchoolYear::DEFAULT,
            },
        })
    }

    fn publish(&self) -> TestResult<(PathBuf, ExportDataset)> {
        let workbook = crate::workbook::build(&self.store, &self.options)?;
        let directory = workbook.parent().ok_or("missing generation")?.to_path_buf();
        let dataset = ExportDataset::reopen_frozen(&directory.join("frozen-input.json"))?;
        Ok((directory, dataset))
    }

    fn rows(&self, dataset: &ExportDataset) -> Vec<Vec<String>> {
        self.expected
            .iter()
            .map(|row| {
                let mut row = row.clone();
                row.push(dataset.lineage.policy_revision.to_string());
                row.push(dataset.lineage.input_generation.clone());
                row
            })
            .collect()
    }
}

fn specification(revision: u32) -> TestResult<CombinedEventSpecification> {
    let lineup = [
        (EventKind::Track100m, None),
        (EventKind::LongJump, None),
        (
            EventKind::ShotPut,
            Some(ImplementMass::try_from(4_000_000_000)?),
        ),
        (EventKind::HighJump, None),
        (EventKind::Track200m, None),
    ]
    .into_iter()
    .map(|(kind, mass)| CombinedDiscipline::new(kind, None, mass))
    .collect::<Result<Vec<_>, _>>()?;
    Ok(CombinedEventSpecification::new(
        EventKind::Pentathlon,
        lineup,
        "Independent normalized fixture scoring".into(),
        revision,
    )?)
}

fn completion(reason: &str) -> TestResult<Option<CombinedEventCompletion>> {
    if reason == "missing_completion" {
        return Ok(None);
    }
    let revision = if reason == "specification_conflict" {
        2
    } else {
        1
    };
    let mut components = vec![CombinedComponentResult::Completed(CentiPoints::new(100000)); 5];
    match reason {
        "incomplete" => {
            *components.first_mut().ok_or("missing component")? =
                CombinedComponentResult::DidNotFinish;
        }
        "total_mismatch" => {
            *components.first_mut().ok_or("missing component")? =
                CombinedComponentResult::Completed(CentiPoints::new(99999));
        }
        "points_overflow" => {
            components = [i32::MAX, 1, 0, 0, 0]
                .into_iter()
                .map(|value| CombinedComponentResult::Completed(CentiPoints::new(value)))
                .collect();
        }
        _ => {}
    }
    Ok(Some(CombinedEventCompletion::new(
        specification(revision)?,
        components,
    )?))
}

fn append(
    store: &Store,
    athlete: &CanonicalAthlete,
    team: &CanonicalTeam,
    evidence: &Evidence,
    reason: &str,
) -> TestResult<(CanonicalEvent, CanonicalPerformance)> {
    let mut meet = CanonicalMeet::new_checked(
        Some(UsJurisdiction::Michigan),
        format!("Independent Combined {reason}"),
        "2026-05-01",
        CompetitionLevel::Invitational,
    )?;
    meet.sports.push(Sport::OutdoorTrack);
    let specification = EventSpecification {
        combined_event: if reason == "missing_specification" {
            None
        } else {
            Some(specification(1)?)
        },
        ..EventSpecification::default()
    };
    let mut event = crate::test_events::owned(CanonicalEvent::new(
        EventIdentity {
            meet: &meet.id,
            kind: EventKind::Pentathlon,
            gender: Gender::Girls,
            division: None,
            round: None,
        },
        specification,
    )?);
    if reason == "conflicting_specification" {
        event.specification.category = Some(CompetitionCategory::Boys);
    }
    let mut performance = CanonicalPerformance::new(
        PerformanceIdentity {
            athlete: &athlete.id,
            event: &event.id,
            meet: &meet.id,
            date: "2026-05-01",
            source_key: reason,
        },
        PerformanceResult {
            team: &team.id,
            mark: Mark::Points(CentiPoints::new(if reason == "points_overflow" {
                i32::MAX
            } else {
                500000
            })),
            wind_mps: None,
            adjustment: None,
            place: Some(1),
        },
    )?;
    performance.combined_completion = completion(reason)?;
    performance.evidence.push(evidence.clone());
    store.append(Table::Meets, &meet)?;
    store.append(Table::Events, &event)?;
    store.append(Table::Performances, &performance)?;
    Ok((event, performance))
}

fn check_count(directory: &Path, name: &str, expected: u64) -> TestResult {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision"), Some(&serde_json::json!(6)));
    check!(eq; manifest.get("normalized_csv_rows").and_then(|counts| counts.get(name)).and_then(serde_json::Value::as_u64), Some(expected));
    Ok(())
}

#[test]
fn zero_winners_still_publish_all_seven_independently_expected_combined_reasons() -> TestResult {
    let fixture = Fixture::new(false)?;
    let before = fixture.store.snapshot().tables_digest(&Table::ALL)?;
    let (directory, dataset) = fixture.publish()?;
    let (header, rows) = read(&directory, "combined_exclusions.csv")?;
    check!(eq; header, HEADER.map(str::to_owned));
    check!(eq; rows, fixture.rows(&dataset));
    check!(read(&directory, "best_marks.csv")?.1.is_empty());
    check!(read(&directory, "comparison_classes.csv")?.1.is_empty());
    check_count(&directory, "combined_exclusions.csv", 7)?;
    check_count(&directory, "best_marks.csv", 0)?;
    publication::verify_published(&directory.join("workbook.xlsx"))?;
    check!(eq; fixture.store.snapshot().tables_digest(&Table::ALL)?, before);
    let repeated = fixture.publish()?.0;
    check!(eq; repeated, directory);
    check!(eq; read(&repeated, "combined_exclusions.csv")?.1, fixture.rows(&dataset));
    Ok(())
}

#[test]
fn a_selected_winner_and_row_limit_do_not_hide_combined_exclusions() -> TestResult {
    let mut fixture = Fixture::new(true)?;
    fixture.options.limit = Some(1);
    let (directory, dataset) = fixture.publish()?;
    check!(eq; read(&directory, "combined_exclusions.csv")?.1, fixture.rows(&dataset));
    check!(eq; read(&directory, "best_marks.csv")?.1.len(), 1);
    check_count(&directory, "combined_exclusions.csv", 7)?;
    check_count(&directory, "best_marks.csv", 1)?;
    publication::verify_published(&directory.join("workbook.xlsx"))?;
    Ok(())
}

#[test]
fn out_of_cohort_combined_candidates_are_not_reported_as_selected_scope_exclusions() -> TestResult {
    let mut fixture = Fixture::new(false)?;
    fixture.options.grad_year = Some(2028);
    let (directory, _) = fixture.publish()?;
    check!(read(&directory, "combined_exclusions.csv")?.1.is_empty());
    check_count(&directory, "combined_exclusions.csv", 0)?;
    publication::verify_published(&directory.join("workbook.xlsx"))?;
    Ok(())
}

fn put(directory: &Path, header: &[String], rows: &[Vec<String>]) -> TestResult {
    let mut writer = csv::Writer::from_path(directory.join("combined_exclusions.csv"))?;
    writer.write_record(header)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}

#[test]
fn independent_combined_readback_refuses_missing_extra_duplicate_and_forged_facts() -> TestResult {
    let fixture = Fixture::new(false)?;
    let (original, dataset) = fixture.publish()?;
    let directory = fixture.directory.path().join("mutated-exclusions");
    std::fs::create_dir(&directory)?;
    for family in FAMILIES.into_iter().chain(["combined_exclusions.csv"]) {
        std::fs::copy(original.join(family), directory.join(family))?;
    }
    let inputs = super::super::super::input::Inputs::new(&dataset, &fixture.options)?;
    let (header, expected) = read(&original, "combined_exclusions.csv")?;
    super::super::verify(&directory, &inputs, 6)?;
    for mutation in [
        "missing",
        "extra",
        "duplicate",
        "reason",
        "category",
        "athlete",
        "event",
        "policy",
        "generation",
    ] {
        let mut rows = expected.clone();
        match mutation {
            "missing" => {
                rows.pop();
            }
            "extra" => {
                let mut row = rows.first().ok_or("missing fixture row")?.clone();
                *row.first_mut().ok_or("missing key")? = "unretained-performance".into();
                rows.push(row);
            }
            "duplicate" => {
                rows.push(rows.first().ok_or("missing fixture row")?.clone());
            }
            _ => {
                let (column, value) = match mutation {
                    "reason" => (4, "eligible"),
                    "category" => (3, "unknown"),
                    "athlete" => (1, "unretained-athlete"),
                    "event" => (2, "unretained-event"),
                    "policy" => (5, "0"),
                    "generation" => (6, "foreign-generation"),
                    _ => return Err("unknown exclusion mutation".into()),
                };
                *rows
                    .first_mut()
                    .and_then(|row| row.get_mut(column))
                    .ok_or("missing fixture cell")? = value.into();
            }
        }
        put(&directory, &header, &rows)?;
        check!(
            super::super::verify(&directory, &inputs, 6).is_err(),
            "accepted {mutation} exclusion mutation"
        );
    }
    put(&directory, &header, &expected)?;
    super::super::verify(&directory, &inputs, 6)?;
    publication::verify_published(&original.join("workbook.xlsx"))?;
    Ok(())
}

#[test]
fn v5_keeps_supporting_candidate_and_provenance_checks_for_selected_winners() -> TestResult {
    let fixture = Fixture::new(true)?;
    let (original, dataset) = fixture.publish()?;
    let directory = fixture.directory.path().join("mutated-support");
    std::fs::create_dir(&directory)?;
    for family in FAMILIES.into_iter().chain(["combined_exclusions.csv"]) {
        std::fs::copy(original.join(family), directory.join(family))?;
    }
    let inputs = super::super::super::input::Inputs::new(&dataset, &fixture.options)?;
    super::super::verify(&directory, &inputs, 6)?;
    super::change(
        &directory,
        "best_marks.csv",
        "supporting_tied_candidate_ids",
        "[]",
    )?;
    check!(super::super::verify(&directory, &inputs, 6).is_err());
    std::fs::copy(
        original.join("best_marks.csv"),
        directory.join("best_marks.csv"),
    )?;
    super::super::verify(&directory, &inputs, 6)?;
    let (header, mut records) = read(&directory, "provenance.csv")?;
    let kind = header
        .iter()
        .position(|name| name == "evidence_type")
        .ok_or("missing evidence type")?;
    let before = records.len();
    records
        .retain(|row| row.get(kind).map(String::as_str) != Some("retained_performance_reference"));
    check!(eq; before.checked_sub(records.len()), Some(1));
    super::files::put(
        &directory,
        "provenance.csv",
        &header.iter().map(String::as_str).collect::<Vec<_>>(),
        &records,
    )?;
    check!(super::super::verify(&directory, &inputs, 6).is_err());
    std::fs::copy(
        original.join("provenance.csv"),
        directory.join("provenance.csv"),
    )?;
    super::super::verify(&directory, &inputs, 6)?;
    publication::verify_published(&original.join("workbook.xlsx"))?;
    Ok(())
}
