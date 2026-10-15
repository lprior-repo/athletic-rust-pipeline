use super::{Expected, TestResult};
use calamine::{open_workbook, Xlsx};
use std::collections::BTreeSet;
use std::path::Path;

mod rows;
use rows::{csv_rows, fields, read, sheet, unique};

pub(super) fn artifacts(directory: &Path, workbook: &Path, expected: &Expected) -> TestResult {
    inventory(directory)?;
    directory_rows(directory, expected)?;
    audit_rows(directory, expected)?;
    exclusions(directory, expected)?;
    check!(csv_rows(&directory.join("best_marks.csv"))?.is_empty());
    empty_bests(directory)?;
    let frozen: serde_json::Value =
        serde_json::from_slice(&read(&directory.join("frozen-input.json"))?)?;
    for (name, original) in &expected.retained {
        check!(eq; frozen.pointer(&format!("/data/{name}")), Some(original));
    }
    workbook_rows(workbook, expected)
}

fn inventory(directory: &Path) -> TestResult {
    let manifest: serde_json::Value =
        serde_json::from_slice(&read(&directory.join("manifest.json"))?)?;
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized inventory")?;
    let names: BTreeSet<_> = counts.keys().map(String::as_str).collect();
    check!(eq; names, BTreeSet::from([
        "athletes.csv", "best_marks.csv", "schools.csv", "coach_contacts.csv",
        "comparison_classes.csv", "provenance.csv", "coverage.csv", "identity_aliases.csv",
        "combined_exclusions.csv", "current_affiliations.csv", "participation_exclusions.csv",
    ]));
    for (name, count) in counts {
        check!(eq; count.as_u64(), Some(u64::try_from(csv_rows(&directory.join(name))?.len())?));
    }
    Ok(())
}

fn empty_bests(directory: &Path) -> TestResult {
    let path = directory.join("best-results-co2027.csv");
    check!(csv_rows(&path)?.is_empty());
    let contents = read(&path)?;
    let mut reader = csv::Reader::from_reader(contents.as_slice());
    let headers = reader.headers()?;
    for name in [
        "athlete_id",
        "canonical_candidate_id",
        "school_at_performance",
        "state_at_performance",
        "current_school_id",
        "current_school",
        "current_state",
        "current_school_as_of",
        "performance_id",
    ] {
        check!(headers.iter().any(|value| value == name));
    }
    check!(read(&directory.join("best-results-co2027.jsonl"))?.is_empty());
    Ok(())
}

fn directory_rows(directory: &Path, expected: &Expected) -> TestResult {
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    let recruiting = csv_rows(&directory.join("recruiting.csv"))?;
    check!(eq; athletes.len(), usize::from(expected.included));
    check!(eq; recruiting.len(), usize::from(expected.included));
    if expected.included {
        fields(
            athletes.first().ok_or("selected athlete")?,
            &[
                ("athlete_id", &expected.public_id),
                ("canonical_candidate_id", &expected.canonical),
                ("current_school_id", &expected.current_school),
                ("identity_status", "accepted_public_identity"),
                (
                    "performance_availability",
                    "no_eligible_selected_mark_in_snapshot",
                ),
            ],
        )?;
        fields(
            recruiting.first().ok_or("recruiting athlete")?,
            &[
                ("athlete_id", &expected.public_id),
                ("canonical_candidate_id", &expected.canonical),
                ("state", "WI"),
                ("school", "Synthetic Transfer High"),
                ("school_city", "Madison"),
                (
                    "school_address",
                    "123 Synthetic Transfer Lane, Madison, WI 53703",
                ),
            ],
        )?;
    }
    let schools = csv_rows(&directory.join("schools.csv"))?;
    check!(eq; schools.len(), 1);
    let school = schools.first().ok_or("in-scope school")?;
    if expected.included {
        fields(
            school,
            &[
                ("school_id", &expected.current_school),
                ("canonical_name", "Synthetic Transfer High"),
                ("postal_school_id", &expected.current_school),
                ("postal_street", "123 Synthetic Transfer Lane"),
                ("postal_city", "Madison"),
                ("postal_state", "WI"),
                ("postal_zip", "53703"),
            ],
        )?;
    } else {
        fields(
            school,
            &[
                ("school_id", &expected.old_school),
                ("canonical_name", "Synthetic selected school"),
            ],
        )?;
    }
    Ok(())
}

fn audit_rows(directory: &Path, expected: &Expected) -> TestResult {
    let aliases = csv_rows(&directory.join("identity_aliases.csv"))?;
    check!(eq; aliases.len(), 2);
    let membership = if expected.included {
        "canonical_in_publication"
    } else {
        "canonical_outside_publication"
    };
    let canonical = if expected.included {
        expected.canonical.as_str()
    } else {
        "unavailable"
    };
    for candidate in [&expected.canonical, &expected.current_candidate] {
        fields(
            unique(&aliases, "original_id", candidate)?,
            &[
                ("public_athlete_id", &expected.public_id),
                ("resolved_id", &expected.canonical),
                ("canonical_id", canonical),
                ("publication_membership", membership),
            ],
        )?;
    }
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    check!(eq; affiliations.len(), 2);
    for (candidate, original) in &expected.affiliation_rows {
        let row = unique(&affiliations, "retained_candidate_id", candidate)?;
        check!(eq; row.len(), 8);
        fields(
            row,
            &[
                ("profile_id", &expected.profile),
                ("as_of", &expected.as_of),
                ("input_generation", &expected.generation),
            ],
        )?;
        for (name, value) in original {
            fields(row, &[(name, value)])?;
        }
    }
    let canonical = unique(&affiliations, "retained_candidate_id", &expected.canonical)?;
    fields(
        canonical,
        &[
            ("current_school_id", &expected.current_school),
            ("resolution_status", "resolved_at_selected_cutoff"),
        ],
    )?;
    let references: Vec<String> = serde_json::from_str(
        canonical
            .get("evidence_refs")
            .ok_or("affiliation evidence")?,
    )?;
    check!(!references.is_empty());
    Ok(())
}

fn exclusions(directory: &Path, expected: &Expected) -> TestResult {
    let rows = csv_rows(&directory.join("participation_exclusions.csv"))?;
    check!(eq; rows.len(), if expected.included { 2 } else { 0 });
    if expected.included {
        for performance in &expected.performances {
            let row = unique(&rows, "performance_id", performance.id.as_str())?;
            fields(
                row,
                &[
                    ("retained_candidate_id", &expected.canonical),
                    ("event_id", performance.event.as_str()),
                    ("disposition", "unresolved"),
                    ("diagnostics", "[]"),
                    (
                        "reasons",
                        "[\"high_school_status_missing\",\"summer_boundary_unresolved\"]",
                    ),
                    (
                        "evidence_refs",
                        expected
                            .participation_refs
                            .get(performance.id.as_str())
                            .ok_or("original participation evidence")?,
                    ),
                    ("input_generation", &expected.generation),
                ],
            )?;
        }
    }
    Ok(())
}

fn workbook_rows(path: &Path, expected: &Expected) -> TestResult {
    check!(std::fs::metadata(path)?.len() <= 16 * 1024 * 1024);
    let mut book: Xlsx<_> = open_workbook(path)?;
    let athletes = sheet(&mut book, "Athletes")?;
    check!(eq; athletes.len(), usize::from(expected.included));
    if expected.included {
        fields(
            athletes.first().ok_or("workbook athlete")?,
            &[
                ("Athlete ID", &expected.public_id),
                ("Canonical Candidate ID", &expected.canonical),
                ("School", "Synthetic Transfer High"),
                ("State", "WI"),
                ("School City", "Madison"),
                (
                    "School Address",
                    "123 Synthetic Transfer Lane, Madison, WI 53703",
                ),
            ],
        )?;
    }
    check!(sheet(&mut book, "PRs")?.is_empty());
    let rows = sheet(&mut book, "Performances_001")?;
    check!(eq; rows.len(), if expected.included { 2 } else { 0 });
    if expected.included {
        for performance in &expected.performances {
            let row = unique(&rows, "Canonical Result ID", performance.id.as_str())?;
            fields(
                row,
                &[
                    ("Retained Candidate ID", &expected.canonical),
                    ("School", "Synthetic selected school"),
                    ("Date", &performance.date),
                    ("Source ResultID", &performance.source_key),
                ],
            )?;
            if performance.source_key == "milesplit_result:201782264" {
                fields(row, &[("Mark", "14-9"), ("Date", "2026-04-03")])?;
            }
        }
    }
    Ok(())
}
