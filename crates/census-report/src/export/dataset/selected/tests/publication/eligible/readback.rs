use super::{csv_rows, json, TestResult};
use calamine::{open_workbook, Reader, Xlsx};
use std::path::Path;

pub(super) fn artifacts(
    directory: &Path,
    workbook: &Path,
    public_id: &str,
    candidate: &str,
    unknown: &str,
) -> TestResult {
    let manifest = json(&directory.join("manifest.json"))?;
    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64), Some(11));
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized counts")?;
    check!(eq; counts.len(), 11);
    for (name, count) in counts {
        let actual = u64::try_from(csv_rows(&directory.join(name))?.len())?;
        check!(eq; count.as_u64(), Some(actual));
    }
    for (name, count) in [
        ("athletes.csv", 1),
        ("best_marks.csv", 0),
        ("schools.csv", 1),
        ("comparison_classes.csv", 0),
        ("coach_contacts.csv", 0),
        ("combined_exclusions.csv", 0),
        ("coverage.csv", 0),
        ("provenance.csv", 8),
        ("identity_aliases.csv", 2),
        ("current_affiliations.csv", 2),
        ("participation_exclusions.csv", 2),
    ] {
        check!(eq; counts.get(name).and_then(serde_json::Value::as_u64), Some(count));
    }
    for name in ["athletes.csv", "recruiting.csv"] {
        let rows = csv_rows(&directory.join(name))?;
        check!(eq; rows.len(), 1);
        check!(eq; rows.first().and_then(|row| row.get("athlete_id")).map(String::as_str), Some(public_id));
    }
    check!(eq; json(&directory.join("census-all-sources.json"))?.pointer("/totals/athletes").and_then(serde_json::Value::as_u64), Some(1));
    let aliases = csv_rows(&directory.join("identity_aliases.csv"))?;
    let accepted = aliases
        .iter()
        .find(|row| row.get("original_id").map(String::as_str) == Some(candidate))
        .ok_or("accepted candidate alias")?;
    check!(eq; accepted.get("public_athlete_id").map(String::as_str), Some(public_id));
    let exclusions = csv_rows(&directory.join("participation_exclusions.csv"))?;
    check!(eq; exclusions.len(), 2);
    for row in &exclusions {
        check!(eq; row.get("retained_candidate_id").map(String::as_str), Some(candidate));
        check!(eq; row.get("disposition").map(String::as_str), Some("unresolved"));
        check!(eq; row.get("reasons").map(String::as_str), Some("[\"high_school_status_missing\",\"summer_boundary_unresolved\"]"));
        check!(eq; row.get("diagnostics").map(String::as_str), Some("[]"));
    }
    check!(csv_rows(&directory.join("best-results-co2027.csv"))?.is_empty());
    check!(std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?.is_empty());
    assert_unknown(directory, unknown)?;
    assert_workbook(workbook, public_id, unknown)
}

fn assert_unknown(directory: &Path, unknown: &str) -> TestResult {
    let aliases = csv_rows(&directory.join("identity_aliases.csv"))?;
    let withheld = aliases
        .iter()
        .find(|row| row.get("original_id").is_some_and(|id| id == unknown))
        .ok_or("unknown alias")?;
    check!(eq; withheld.get("public_athlete_id").map(String::as_str), Some(""));
    check!(eq; withheld.get("publication_membership").map(String::as_str), Some("canonical_outside_publication"));
    check!(eq; withheld.get("publication_identity_status").map(String::as_str), Some("unknown"));
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    let withheld = affiliations
        .iter()
        .find(|row| {
            row.get("retained_candidate_id")
                .is_some_and(|id| id == unknown)
        })
        .ok_or("unknown affiliation")?;
    check!(eq; withheld.get("resolution_status").map(String::as_str), Some("current_affiliation_unknown"));
    Ok(())
}

fn assert_workbook(path: &Path, public_id: &str, unknown: &str) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    let athletes = book.worksheet_range("Athletes")?;
    check!(eq; athletes.height(), 2);
    let mut rows = athletes.rows();
    let headers = rows.next().ok_or("athlete headers")?;
    let row = rows.next().ok_or("selected athlete")?;
    for (label, expected) in [("Athlete ID", public_id), ("Observed bests", "")] {
        let column = headers
            .iter()
            .position(|cell| cell == label)
            .ok_or("athlete column")?;
        check!(eq; row.get(column).ok_or("athlete cell")?.to_string(), expected);
    }
    check!(!athletes
        .rows()
        .skip(1)
        .any(|row| row.iter().any(|cell| cell == unknown)));
    let aliases = csv_rows(
        &path
            .parent()
            .ok_or("generation")?
            .join("identity_aliases.csv"),
    )?;
    check!(eq; aliases.len(), 2);
    let accepted = aliases
        .iter()
        .filter_map(|row| row.get("public_athlete_id"))
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    check!(eq; accepted.len(), 1);
    check!(eq; accepted.into_iter().collect::<std::collections::BTreeSet<_>>().len(), 1);
    Ok(())
}
