use super::*;
use std::collections::BTreeSet;

pub(super) fn observe(
    dataset: &ExportDataset,
    directory: &Path,
    expected: &Expected<'_>,
) -> TestResult {
    manifest(directory)?;
    let athletes = artifacts::csv(&directory.join("athletes.csv"))?;
    artifacts::fields(
        artifacts::unique(&athletes, "athlete_id", expected.public.as_str())?,
        &[
            ("identity_status", "accepted_public_identity"),
            ("performance_availability", "eligible_selected_mark"),
        ],
    )?;
    let marks = artifacts::csv(&directory.join("best_marks.csv"))?;
    check!(eq; marks.len(),1);
    artifacts::fields(
        marks.first().ok_or("normalized genuine best")?,
        &[
            ("athlete_id", expected.public.as_str()),
            ("performance_id", expected.performance.id.as_str()),
            ("display_mark", "11:23.56"),
            ("meet_date", "2026-03-11"),
            ("school_at_performance", expected.school.as_str()),
            (
                "input_generation",
                dataset.lineage.input_generation.as_str(),
            ),
            ("eligible_result_group_count", "1"),
            ("tied_result_group_count", "1"),
        ],
    )?;
    sidecars(directory, expected)?;
    affiliations(dataset, directory, expected)
}

fn manifest(directory: &Path) -> TestResult {
    let manifest: serde_json::Value =
        serde_json::from_slice(&artifacts::bytes(&directory.join("manifest.json"))?)?;
    check!(eq; manifest.get("schema_revision").and_then(serde_json::Value::as_u64),Some(8));
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized current inventory")?;
    let expected = [
        "athletes.csv",
        "best_marks.csv",
        "coach_contacts.csv",
        "combined_exclusions.csv",
        "comparison_classes.csv",
        "coverage.csv",
        "current_affiliations.csv",
        "identity_aliases.csv",
        "participation_exclusions.csv",
        "provenance.csv",
        "schools.csv",
    ]
    .into_iter()
    .collect::<BTreeSet<_>>();
    check!(eq; counts.keys().map(String::as_str).collect::<BTreeSet<_>>(),expected);
    for (name, count) in counts {
        check!(eq; count.as_u64(),Some(u64::try_from(artifacts::csv(&directory.join(name))?.len())?),"inventory file {name}");
    }
    check!(eq; counts.get("best_marks.csv").and_then(serde_json::Value::as_u64),Some(1));
    check!(eq; counts.get("participation_exclusions.csv").and_then(serde_json::Value::as_u64),Some(0));
    Ok(())
}

fn sidecars(directory: &Path, expected: &Expected<'_>) -> TestResult {
    let rows = artifacts::csv(&directory.join("best-results-co2027.csv"))?;
    check!(eq; rows.len(),1);
    let row = rows.first().ok_or("genuine adjusted Best CSV")?;
    artifacts::fields(
        row,
        &[
            ("athlete_id", expected.public.as_str()),
            (
                "canonical_candidate_id",
                expected.performance.athlete.as_str(),
            ),
            ("performance_id", expected.performance.id.as_str()),
            ("source_key", expected.performance.source_key.as_str()),
            ("best_mark", "11:23.56"),
            ("original_mark", "11:29.42"),
            ("adjustment_kind", "track_size_conversion"),
            ("date", "2026-03-11"),
            ("current_school_id", expected.school.as_str()),
            ("current_school", "Roncalli"),
            ("current_state", "IN"),
            ("current_school_as_of", "2026-10-21"),
        ],
    )?;
    let note = expected
        .selection
        .result
        .adjustment_note
        .as_deref()
        .ok_or("actual converted note")?;
    check!(eq; expected.selection.result.adjustment_source,None);
    artifacts::fields(row, &[("adjustment_note", note), ("adjustment_source", "")])?;
    let data = artifacts::bytes(&directory.join("best-results-co2027.jsonl"))?;
    let text = std::str::from_utf8(&data)?;
    let rows = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<serde_json::Value>, _>>()?;
    let [row] = rows.as_slice() else {
        return Err("one genuine selected Best JSONL row required".into());
    };
    check!(eq; row.get("schema").and_then(serde_json::Value::as_str),Some("selected_identity_v2"));
    check!(eq; row.get("athlete_id").and_then(serde_json::Value::as_str),Some(expected.public.as_str()));
    check!(eq; row.get("canonical_candidate_id").and_then(serde_json::Value::as_str),Some(expected.performance.athlete.as_str()));
    check!(eq; row.get("selection"),Some(&serde_json::to_value(expected.selection)?));
    let selection = row.get("selection").ok_or("selected Best JSONL value")?;
    check!(selection.get("adjustment_source").is_none());
    check!(eq; selection.get("result_url").and_then(serde_json::Value::as_str),Some(expected.selection.source.result_url.as_str()));
    check!(eq; selection.get("source_athlete").and_then(serde_json::Value::as_str),Some("8779607"));
    Ok(())
}

fn affiliations(dataset: &ExportDataset, directory: &Path, expected: &Expected<'_>) -> TestResult {
    let rows = artifacts::csv(&directory.join("current_affiliations.csv"))?;
    let row = artifacts::unique(
        &rows,
        "retained_candidate_id",
        expected.performance.athlete.as_str(),
    )?;
    artifacts::fields(
        row,
        &[
            ("current_school_id", expected.school.as_str()),
            ("resolution_status", "resolved_at_selected_cutoff"),
            ("as_of", "2026-10-21"),
            (
                "profile_id",
                dataset
                    .verified_current_affiliations()?
                    .profile_id()
                    .as_str(),
            ),
            (
                "input_generation",
                dataset.lineage.input_generation.as_str(),
            ),
        ],
    )?;
    check!(row
        .get("evidence_refs")
        .is_some_and(|value| !value.is_empty() && value != "[]"));
    Ok(())
}
