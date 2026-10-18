use super::super::source::TestResult;
use super::artifacts;
use super::Expected;
use crate::bests::SelectionOutcome;
use crate::export::ExportDataset;
use census_domain::model::CanonicalPerformance;
use std::collections::BTreeSet;
use std::path::Path;
pub(super) fn matches(
    dataset: &ExportDataset,
    directory: &Path,
    outcome: &SelectionOutcome,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    manifest_matches(directory)?;
    athletes_row(directory, expected)?;
    best_row(directory, dataset, wanted, expected)?;
    best_sidecars(directory, dataset, wanted, expected)?;
    affiliations_row(directory, dataset, wanted, expected)?;
    super::exclusions::rows(directory, dataset, outcome, wanted)
}
fn manifest_matches(directory: &Path) -> TestResult<()> {
    let manifest = artifacts::manifest(directory)?;
    let counts = manifest
        .get("normalized_csv_rows")
        .and_then(serde_json::Value::as_object)
        .ok_or("normalized inventory")?;
    for (name, count) in counts {
        check!(eq; count.as_u64(), Some(u64::try_from(artifacts::csv_rows(&directory.join(name))?.len())?));
    }
    check!(eq; counts.get("athletes.csv").and_then(serde_json::Value::as_u64), Some(1));
    check!(eq; counts.get("best_marks.csv").and_then(serde_json::Value::as_u64), Some(1));
    Ok(())
}
fn athletes_row(directory: &Path, expected: &Expected<'_>) -> TestResult<()> {
    let rows = artifacts::csv_rows(&directory.join("athletes.csv"))?;
    check!(eq; rows.len(), 1);
    artifacts::fields(
        rows.first().ok_or("athletes row")?,
        &[
            ("athlete_id", expected.public.as_str()),
            ("performance_availability", "eligible_selected_mark"),
            ("identity_status", "accepted_public_identity"),
        ],
    )?;
    Ok(())
}
fn best_row(
    directory: &Path,
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let rows = artifacts::csv_rows(&directory.join("best_marks.csv"))?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("best row")?;
    let mut digests = BTreeSet::new();
    for fact in &wanted.evidence {
        digests.insert(census_domain::model::serialized_digest(&(
            "performance",
            wanted.id.as_str(),
            fact,
        ))?);
    }
    check!(!digests.is_empty());
    let refs = serde_json::to_string(&digests)?;
    artifacts::fields(
        row,
        &[
            ("athlete_id", expected.public.as_str()),
            ("performance_id", wanted.id.as_str()),
            ("display_mark", expected.seconds),
            ("meet_date", expected.day),
            ("school_at_performance", expected.historical_school.as_str()),
            ("evidence_refs", refs.as_str()),
            (
                "input_generation",
                dataset.lineage.input_generation.as_str(),
            ),
            (
                "eligible_result_group_count",
                if expected.seconds == "50.00" {
                    "2"
                } else {
                    "1"
                },
            ),
            ("tied_result_group_count", "1"),
            ("result_group_basis", "canonical_performance_id"),
            ("physical_performance_count_status", "unknown"),
            ("selection_policy_version", "21"),
        ],
    )?;
    check!(eq; row.get("raw_mark").map(String::as_str), Some("unavailable"));
    let headers = artifacts::headers_of(&directory.join("best_marks.csv"))?;
    for name in [
        "athlete_id",
        "performance_id",
        "display_mark",
        "meet_date",
        "school_at_performance",
        "evidence_refs",
        "input_generation",
    ] {
        check!(headers.iter().any(|value| value == name));
    }
    Ok(())
}
fn best_sidecars(
    directory: &Path,
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let rows = artifacts::csv_rows(&directory.join("best-results-co2027.csv"))?;
    check!(eq; rows.len(), 1);
    let row = rows.first().ok_or("best csv")?;
    let historical = dataset
        .schools
        .get(expected.historical_school)
        .ok_or("historical name")?;
    let current = dataset
        .schools
        .get(expected.current_school)
        .ok_or("current name")?;
    artifacts::fields(
        row,
        &[
            ("athlete_id", expected.public.as_str()),
            ("canonical_candidate_id", wanted.athlete.as_str()),
            ("best_mark", expected.seconds),
            ("date", expected.day),
            ("performance_id", wanted.id.as_str()),
            ("source_key", wanted.source_key.as_str()),
            ("school_at_performance", historical.name.as_str()),
            ("current_school_id", expected.current_school.as_str()),
            ("current_school", current.name.as_str()),
            ("tied_result_group_count", "1"),
            ("result_group_basis", "canonical_performance_id"),
            ("physical_performance_count_status", "unknown"),
            ("selection_policy_version", "21"),
        ],
    )?;
    check!(eq; row.get("original_mark").map(String::as_str), Some(""));
    check!(eq; row.get("adjustment_kind").map(String::as_str), Some(""));
    let headers = artifacts::headers_of(&directory.join("best-results-co2027.csv"))?;
    check!(!headers.iter().any(|value| value == "raw_mark"));
    for name in [
        "athlete_id",
        "canonical_candidate_id",
        "best_mark",
        "performance_id",
        "current_school_id",
        "current_school_as_of",
    ] {
        check!(headers.iter().any(|value| value == name));
    }
    let text = std::fs::read_to_string(directory.join("best-results-co2027.jsonl"))?;
    let lines: Vec<serde_json::Value> = text
        .lines()
        .map(serde_json::from_str)
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; lines.len(), 1);
    let found = lines.first().ok_or("best jsonl")?;
    check!(eq; found.pointer("/athlete_id").and_then(serde_json::Value::as_str), Some(expected.public.as_str()));
    check!(eq; found.pointer("/canonical_candidate_id").and_then(serde_json::Value::as_str), Some(wanted.athlete.as_str()));
    check!(eq; found.pointer("/selection/performance_id").and_then(serde_json::Value::as_str), Some(wanted.id.as_str()));
    check!(eq; found.pointer("/selection/date").and_then(serde_json::Value::as_str), Some(expected.day));
    check!(eq; found.pointer("/current_school_id").and_then(serde_json::Value::as_str), Some(expected.current_school.as_str()));
    Ok(())
}
fn affiliations_row(
    directory: &Path,
    dataset: &ExportDataset,
    wanted: &CanonicalPerformance,
    expected: &Expected<'_>,
) -> TestResult<()> {
    let rows = artifacts::csv_rows(&directory.join("current_affiliations.csv"))?;
    let row = artifacts::unique(&rows, "retained_candidate_id", wanted.athlete.as_str())?;
    check!(eq; row.len(), 8);
    artifacts::fields(
        row,
        &[
            ("current_school_id", expected.current_school.as_str()),
            ("resolution_status", "resolved_at_selected_cutoff"),
            (
                "profile_id",
                dataset
                    .verified_current_affiliations()?
                    .profile_id()
                    .as_str(),
            ),
            (
                "as_of",
                dataset.verified_current_affiliations()?.as_of().as_str(),
            ),
            (
                "input_generation",
                dataset.lineage.input_generation.as_str(),
            ),
        ],
    )?;
    let headers = artifacts::headers_of(&directory.join("current_affiliations.csv"))?;
    for name in [
        "retained_candidate_id",
        "current_school_id",
        "resolution_status",
        "evidence_refs",
        "unavailable_reasons",
        "profile_id",
        "as_of",
        "input_generation",
    ] {
        check!(headers.iter().any(|value| value == name));
    }
    Ok(())
}
