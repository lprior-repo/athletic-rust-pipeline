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
    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64), Some(8));
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
    let metrics = book.worksheet_range("Run Metrics")?;
    let metrics: Vec<Vec<String>> = metrics
        .rows()
        .map(|row| row.iter().map(ToString::to_string).collect())
        .collect();
    for (label, value) in [
        ("Recruiting athletes", "1"),
        ("Best-mark rows reduced", "0"),
        ("Retained athlete candidate records", "2"),
        ("Accepted candidate records", "1"),
        ("Withheld or unknown candidate records", "1"),
        ("Accepted public people", "1"),
    ] {
        let observed = metrics
            .iter()
            .find(|row| row.first().is_some_and(|cell| cell == label))
            .and_then(|row| row.get(1))
            .ok_or("identity metric")?;
        check!(eq; observed, value);
    }
    let people = metrics
        .iter()
        .rev()
        .find(|row| row.first().is_some_and(|cell| cell == "Athletes"))
        .ok_or("people reconciliation")?;
    check!(eq; people.get(1).map(String::as_str), Some("1"));
    check!(eq; people.get(2).map(String::as_str), Some("1"));
    check!(eq; people.get(3).map(String::as_str), Some("reconciled"));
    let review = book.worksheet_range("Review")?;
    check!(review.rows().any(|row| row
        .first()
        .is_some_and(|cell| cell == "Public identity candidate withheld")
        && row.get(2).is_some_and(|cell| cell == unknown)));
    assert_no_pr(&mut book, public_id)?;
    Ok(())
}

fn assert_no_pr(book: &mut Xlsx<std::io::BufReader<std::fs::File>>, public_id: &str) -> TestResult {
    let prs = book.worksheet_range("PRs")?;
    check!(eq; prs.height(), 1);
    let athletes = book.worksheet_range("Athletes")?;
    check!(athletes
        .rows()
        .any(|row| row.iter().any(|cell| cell == public_id)));
    Ok(())
}
