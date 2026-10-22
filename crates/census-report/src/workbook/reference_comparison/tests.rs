use super::{index::Index, reference, ComparisonCounts};
use crate::workbook::verify::canonical::Value;
use census_domain::model::{SourceIdentity, SourceNamespace};
use std::collections::BTreeSet;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exact_provider_match_allows_broader_independent_output() -> TestResult {
    let mut index = Index::empty();
    index.add("Example Runner".into(), 2027, true, BTreeSet::from([1]))?;
    index.add("Other Runner".into(), 2027, true, BTreeSet::from([2]))?;
    index.add("Unlinked Runner".into(), 2027, true, BTreeSet::new())?;
    let mut counts = ComparisonCounts::default();
    index.observe(1, " Example  Runner ", 2027, &mut counts)?;
    index.finish(&mut counts);
    check!(eq; counts.matched_unique, 1);
    check!(eq; counts.output_only_provider_ids, 1);
    check!(eq; counts.output_athletes_without_provider_id, 1);
    Ok(())
}

#[test]
fn duplicate_reference_and_ambiguous_output_never_choose_first() -> TestResult {
    let mut index = Index::empty();
    index.add("Example Runner".into(), 2027, true, BTreeSet::from([1]))?;
    index.add("Another Runner".into(), 2027, true, BTreeSet::from([1]))?;
    let mut counts = ComparisonCounts::default();
    index.observe(1, "Example Runner", 2027, &mut counts)?;
    index.observe(1, "Example Runner", 2027, &mut counts)?;
    index.observe(3, "Absent Runner", 2027, &mut counts)?;
    index.finish(&mut counts);
    check!(eq; counts.matched_unique, 0);
    check!(eq; counts.ambiguous_output_ids, 1);
    check!(eq; counts.duplicate_reference_ids, 1);
    check!(eq; counts.reference_only, 1);
    check!(!counts.passed());
    Ok(())
}

#[test]
fn many_reference_identities_to_one_output_are_explicit() -> TestResult {
    let mut index = Index::empty();
    index.add("Example Runner".into(), 2027, true, BTreeSet::from([1, 2]))?;
    let mut counts = ComparisonCounts::default();
    index.observe(1, "Example Runner", 2027, &mut counts)?;
    index.observe(2, "Example Runner", 2027, &mut counts)?;
    index.finish(&mut counts);
    check!(eq; counts.many_reference_ids_to_one_output, 1);
    check!(!counts.passed());
    Ok(())
}

#[test]
fn cohort_and_name_differences_are_not_hidden_by_id_match() -> TestResult {
    let mut index = Index::empty();
    index.add("Different Runner".into(), 2028, false, BTreeSet::from([1]))?;
    let mut counts = ComparisonCounts::default();
    index.observe(1, "Example Runner", 2027, &mut counts)?;
    check!(eq; counts.graduation_year_mismatches, 1);
    check!(eq; counts.name_mismatches, 1);
    check!(eq; counts.required_fields_unassessed, 1);
    check!(!counts.passed());
    Ok(())
}

#[test]
fn plural_names_are_unassessed_and_commas_are_not_split() -> TestResult {
    let mut index = Index::empty();
    index.add("Runner, Example".into(), 2027, true, BTreeSet::from([1]))?;
    index.add("Example Runner".into(), 2027, true, BTreeSet::from([2]))?;
    let mut counts = ComparisonCounts::default();
    index.observe(1, "Runner, Example", 2027, &mut counts)?;
    index.observe(2, "Example Runner; Other Runner", 2027, &mut counts)?;
    check!(eq; counts.name_mismatches, 0);
    check!(eq; counts.required_fields_unassessed, 1);
    Ok(())
}

#[test]
fn source_identity_namespace_and_numeric_contract_are_reused() -> TestResult {
    let current = SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "7");
    let legacy = SourceIdentity::new(
        SourceNamespace::LegacyAthleticNet {
            kind: "athlete".into(),
        },
        "7",
    );
    let team = SourceIdentity::new(SourceNamespace::athletic_net("team"), "7");
    let identities = [current.clone(), current, legacy, team];
    check!(eq;
        super::index::provider_keys(identities.iter())?,
        BTreeSet::from([7])
    );
    let bad = SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "07");
    check!(super::index::provider_keys([bad].iter()).is_err());
    check!(eq;
        reference::id(&Value::Number(7.0))?,
        reference::id(&Value::Text("7".into()))?
    );
    for value in [
        Value::Number(0.0),
        Value::Number(1.5),
        Value::Number(9_007_199_254_740_992.0),
        Value::Text("07".into()),
        Value::Text("7e0".into()),
    ] {
        check!(reference::id(&value).is_err());
    }
    Ok(())
}

#[test]
fn bounded_inline_workbook_reads_every_row_and_refuses_wrong_profile() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("reference.xlsx");
    reference_book(&path, "All Athletes", "AthleteID", "Example Runner")?;
    let mut index = Index::empty();
    index.add("Example Runner".into(), 2027, true, BTreeSet::from([1]))?;
    let counts = reference::compare(&path, 1, &mut index, 2027)?;
    check!(eq; counts.reference_rows, 1);
    check!(counts.passed());
    check!(reference::compare(&path, 2, &mut Index::empty(), 2027).is_err());
    reference_book(&path, "Wrong Sheet", "AthleteID", "Example Runner")?;
    check!(reference::compare(&path, 1, &mut Index::empty(), 2027).is_err());
    reference_book(&path, "All Athletes", "Wrong ID", "Example Runner")?;
    check!(reference::compare(&path, 1, &mut Index::empty(), 2027).is_err());
    Ok(())
}

pub(super) fn reference_book(
    path: &std::path::Path,
    sheet: &str,
    id_header: &str,
    name: &str,
) -> TestResult {
    let mut book = rust_xlsxwriter::Workbook::new();
    let worksheet = book.add_worksheet_with_constant_memory()?;
    worksheet.set_name(sheet)?;
    for (column, header) in [
        id_header,
        "Names",
        "GradeID",
        "Teams",
        "States",
        "Events",
        "Result Count",
        "Other Observed Grades",
        "Identity Match Status",
    ]
    .iter()
    .enumerate()
    {
        worksheet.write_string(0, u16::try_from(column)?, *header)?;
    }
    worksheet.write_number(1, 0, 1)?;
    worksheet.write_string(1, 1, name)?;
    worksheet.write_number(1, 2, 11)?;
    worksheet.write_string(1, 8, "Not identity-matched")?;
    book.save(path)?;
    Ok(())
}

#[test]
fn invalid_profile_refuses_before_opening_publication() -> TestResult {
    let request = super::ComparisonRequest {
        workbook: std::path::Path::new("absent"),
        reference: std::path::Path::new("absent"),
        reference_profile: super::REFERENCE_PROFILE,
        graduation_year: 2028,
    };
    let error = super::compare(&request)
        .err()
        .ok_or("wrong graduation year must refuse")?;
    check!(matches!(error, super::ReportError::Invariant { .. }));
    Ok(())
}

#[test]
fn normalized_csv_uses_shared_schema_row_count_and_bounded_decoder() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("athletes.csv");
    let mut writer = csv::Writer::from_path(&path)?;
    writer.write_record([
        "athlete_id",
        "display_name",
        "graduation_year",
        "graduation_status",
        "competition_category",
        "competition_category_status",
        "current_school_id",
        "current_school_status",
        "cohort_evidence_refs",
        "affiliation_evidence_refs",
        "performance_availability",
        "source_identities",
        "record_revision",
        "record_revision_status",
        "input_generation",
        "cohort_claims",
        "identity_status",
        "entity_evidence_refs",
        "affiliation_evidence_status",
    ])?;
    let identity = SourceIdentity::new(SourceNamespace::athletic_net("athlete"), "1");
    let sources = serde_json::to_string(&(Some(identity), Vec::<SourceIdentity>::new()))?;
    let mut row = vec![String::new(); 19];
    for (slot, value) in row.iter_mut().zip([
        "synthetic:1",
        "Example Runner",
        "2027",
        "resolved_by_shared_cohort_policy",
        "male",
        "observed",
        "synthetic:school",
        "unavailable",
        "[]",
        "[]",
        "no_eligible_selected_mark_in_snapshot",
        &sources,
        "unavailable",
        "unavailable",
        "synthetic-generation",
        "[]",
        "accepted",
        "[]",
        "unavailable",
    ]) {
        *slot = value.to_string();
    }
    writer.write_record(&row)?;
    writer.flush()?;
    let bytes = std::fs::metadata(&path)?.len();
    let mut index = Index::read(&path, bytes, 1, 6)?;
    let mut counts = ComparisonCounts::default();
    index.observe(1, "Example Runner", 2027, &mut counts)?;
    check!(counts.passed());
    check!(Index::read(&path, bytes, 0, 6).is_err());
    check!(Index::read(&path, bytes, 2, 6).is_err());
    check!(Index::read(&path, 129 * 1024 * 1024, 1, 6).is_err());
    check!(Index::read(&path, bytes, 1_000_001, 6).is_err());
    drop(writer);
    let mut reader = csv::Reader::from_path(&path)?;
    let headers = reader.headers()?.clone();
    drop(reader);
    let mut writer = csv::Writer::from_path(&path)?;
    writer.write_record(
        headers
            .iter()
            .chain(["canonical_candidate_id", "identity_resolution"]),
    )?;
    row.extend([
        "synthetic-candidate".into(),
        "{\"state\":\"canonical\"}".into(),
    ]);
    writer.write_record(&row)?;
    writer.flush()?;
    let bytes = std::fs::metadata(&path)?.len();
    Index::read(&path, bytes, 1, 7)?;
    Index::read(&path, bytes, 1, 8)?;
    Index::read(&path, bytes, 1, 10)?;
    check!(Index::read(&path, bytes, 1, 6).is_err());
    check!(Index::read(&path, bytes, 1, 9).is_err());
    check!(Index::read(&path, bytes, 1, 11).is_err());
    Ok(())
}

#[test]
fn late_workbook_corruption_never_returns_completed_counts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("reference.xlsx");
    reference_book(&path, "All Athletes", "AthleteID", "Example Runner")?;
    let file = std::fs::OpenOptions::new().write(true).open(&path)?;
    let length = file.metadata()?.len();
    file.set_len(length.saturating_sub(12))?;
    check!(reference::compare(&path, 1, &mut Index::empty(), 2027).is_err());
    Ok(())
}

#[test]
fn aggregate_json_has_no_athlete_identifiers_or_values() -> TestResult {
    let mut index = Index::empty();
    index.add(
        "Synthetic Private Name".into(),
        2027,
        true,
        BTreeSet::from([987654321]),
    )?;
    let mut counts = ComparisonCounts::default();
    index.observe(987654321, "Synthetic Private Name", 2027, &mut counts)?;
    index.finish(&mut counts);
    let output = serde_json::to_string(&counts)?;
    check!(!output.contains("Synthetic Private Name"));
    check!(!output.contains("987654321"));
    Ok(())
}
