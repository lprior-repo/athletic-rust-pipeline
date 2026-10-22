use super::TestResult;
use calamine::{open_workbook, Reader, Xlsx};
use std::collections::BTreeMap;
use std::path::Path;

pub(super) struct Expected {
    pub public_id: String,
    pub canonical: String,
    pub current_candidate: String,
    pub old_school: String,
    pub current_school: String,
    pub retained_result: String,
}

pub(super) fn artifacts(directory: &Path, workbook: &Path, expected: &Expected) -> TestResult {
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("manifest.json"))?)?;
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized manifest inventory")?;
    check!(eq; counts.len(), 11);
    for (name, count) in counts {
        check!(eq; count.as_u64(), Some(u64::try_from(csv_rows(&directory.join(name))?.len())?));
    }
    let athletes = csv_rows(&directory.join("athletes.csv"))?;
    check!(eq; athletes.len(), 1);
    fields(
        athletes.first().ok_or("accepted athlete")?,
        &[
            ("athlete_id", &expected.public_id),
            ("current_school_id", &expected.current_school),
            (
                "performance_availability",
                "no_eligible_selected_mark_in_snapshot",
            ),
        ],
    )?;
    let bests = csv_rows(&directory.join("best_marks.csv"))?;
    check!(bests.is_empty());
    retained_history(directory, expected)?;
    let recruiting = csv_rows(&directory.join("recruiting.csv"))?;
    check!(eq; recruiting.len(), 1);
    fields(
        recruiting.first().ok_or("recruiting person")?,
        &[
            ("athlete_id", &expected.public_id),
            ("state", "WI"),
            ("school", "Synthetic Transfer High"),
            ("school_city", "Madison"),
            (
                "school_address",
                "123 Synthetic Transfer Lane, Madison, WI 53703",
            ),
        ],
    )?;
    selected_bests(directory, expected)?;
    aliases(directory, expected)?;
    schools(directory, expected)?;
    workbook_rows(workbook, expected)
}

fn aliases(directory: &Path, expected: &Expected) -> TestResult {
    let aliases = csv_rows(&directory.join("identity_aliases.csv"))?;
    check!(eq; aliases.len(), 2);
    for candidate in [&expected.canonical, &expected.current_candidate] {
        let row = aliases
            .iter()
            .find(|row| row.get("original_id") == Some(candidate))
            .ok_or("retained candidate alias")?;
        fields(row, &[("public_athlete_id", &expected.public_id)])?;
    }
    let exclusions = csv_rows(&directory.join("participation_exclusions.csv"))?;
    check!(eq; exclusions.len(), 2);
    for row in &exclusions {
        fields(
            row,
            &[
                ("retained_candidate_id", &expected.canonical),
                ("disposition", "unresolved"),
                (
                    "reasons",
                    "[\"high_school_status_missing\",\"summer_boundary_unresolved\"]",
                ),
                ("diagnostics", "[]"),
            ],
        )?;
    }
    check!(exclusions
        .iter()
        .any(|row| row.get("performance_id") == Some(&expected.retained_result)));
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    check!(!affiliations.is_empty());
    Ok(())
}

fn schools(directory: &Path, expected: &Expected) -> TestResult {
    let schools = csv_rows(&directory.join("schools.csv"))?;
    check!(eq; schools.len(), 2);
    let current = schools
        .iter()
        .find(|row| row.get("school_id") == Some(&expected.current_school))
        .ok_or("current school")?;
    fields(
        current,
        &[
            ("canonical_name", "Synthetic Transfer High"),
            ("postal_street", "123 Synthetic Transfer Lane"),
            ("postal_city", "Madison"),
            ("postal_state", "WI"),
            ("postal_zip", "53703"),
            ("postal_school_id", &expected.current_school),
        ],
    )?;
    let old = schools
        .iter()
        .find(|row| row.get("school_id") == Some(&expected.old_school))
        .ok_or("retained performance school")?;
    fields(old, &[("canonical_name", "Synthetic selected school")])
}

fn workbook_rows(path: &Path, expected: &Expected) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    check!(eq; book.sheet_names(), vec!["Athletes".to_owned()]);
    sheet(
        &mut book,
        "Athletes",
        &[
            ("Athlete ID", &expected.public_id),
            ("School", "Synthetic Transfer High"),
            ("School State", "WI"),
            ("Observed bests", ""),
            ("School City", "Madison"),
            (
                "School Address",
                "123 Synthetic Transfer Lane, Madison, WI 53703",
            ),
        ],
    )?;
    Ok(())
}

fn sheet(
    book: &mut Xlsx<std::io::BufReader<std::fs::File>>,
    name: &str,
    expected: &[(&str, &str)],
) -> TestResult {
    let range = book.worksheet_range(name)?;
    check!(eq; range.height(), 2);
    let mut rows = range.rows();
    let header = rows.next().ok_or("worksheet header")?;
    let row = rows.next().ok_or("worksheet person")?;
    for (label, value) in expected {
        let column = header
            .iter()
            .position(|cell| cell == *label)
            .ok_or_else(|| format!("missing {name} header {label}"))?;
        check!(eq; row.get(column).ok_or("worksheet value")?.to_string(), *value);
    }
    Ok(())
}

fn fields(row: &BTreeMap<String, String>, expected: &[(&str, &str)]) -> TestResult {
    for (name, value) in expected {
        check!(eq; row.get(*name).map(String::as_str), Some(*value));
    }
    Ok(())
}

fn csv_rows(path: &Path) -> TestResult<Vec<BTreeMap<String, String>>> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    reader
        .records()
        .map(|record| {
            let record = record?;
            Ok(headers
                .iter()
                .zip(record.iter())
                .map(|(name, value)| (name.to_owned(), value.to_owned()))
                .collect())
        })
        .collect()
}

fn selected_bests(directory: &Path, expected: &Expected) -> TestResult {
    let rows = csv_rows(&directory.join("best-results-co2027.csv"))?;
    check!(rows.is_empty());
    let contents = std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?;
    check!(contents.is_empty());
    let affiliations = csv_rows(&directory.join("current_affiliations.csv"))?;
    check!(affiliations
        .iter()
        .any(|row| row.get("retained_candidate_id") == Some(&expected.canonical)));
    Ok(())
}

fn retained_history(directory: &Path, expected: &Expected) -> TestResult {
    let frozen: serde_json::Value =
        serde_json::from_slice(&std::fs::read(directory.join("frozen-input.json"))?)?;
    let performances: Vec<census_domain::model::CanonicalPerformance> = serde_json::from_value(
        frozen
            .pointer("/data/performances")
            .ok_or("retained performances")?
            .clone(),
    )?;
    let teams: BTreeMap<String, census_domain::model::CanonicalTeam> = serde_json::from_value(
        frozen
            .pointer("/data/teams")
            .ok_or("retained teams")?
            .clone(),
    )?;
    check!(eq; performances.len(), 2);
    for performance in &performances {
        check!(eq; performance.athlete.as_str(), expected.canonical);
        let team = teams
            .values()
            .find(|row| row.id == performance.team)
            .ok_or("historical team")?;
        check!(eq; team.school.as_str(), expected.old_school);
        check!(ne; team.school.as_str(), expected.current_school);
    }
    let retained = performances
        .iter()
        .find(|row| row.id.as_str() == expected.retained_result)
        .ok_or("retained stronger result")?;
    check!(eq; crate::bests::mark_text(&retained.mark), "14-9");
    check!(eq; retained.date, "2026-04-03");
    check!(eq; retained.source_key, "milesplit_result:201782264");
    Ok(())
}
