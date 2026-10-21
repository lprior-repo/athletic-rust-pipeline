use super::{
    build, check_selected_output, isolated, selected_fixture, selected_options, TestResult,
};
use census_report::export::{decoded_generations, resident_generations, ExportJobRequest};
use census_report::report::{ReportError, Scope};
use std::path::Path;

fn excluded_49(path: &Path, candidate: &str) -> TestResult<csv::StringRecord> {
    let mut reader = csv::Reader::from_path(path)?;
    let headers = reader.headers()?.clone();
    let mut rows = reader.records();
    let row = rows
        .next()
        .transpose()?
        .ok_or("outside-window exclusion absent")?;
    check!(eq; rows.next().transpose()?, None);
    for (field, expected) in [
        ("retained_candidate_id", candidate),
        ("performance_id", "perf_49996bc198c5b855"),
        ("disposition", "excluded"),
    ] {
        let index = headers
            .iter()
            .position(|value| value == field)
            .ok_or("exclusion field absent")?;
        check!(eq; row.get(index), Some(expected));
    }
    let index = headers
        .iter()
        .position(|value| value == "reasons")
        .ok_or("exclusion reasons absent")?;
    let reasons: Vec<String> =
        serde_json::from_str(row.get(index).ok_or("exclusion reasons missing")?)?;
    check!(eq; reasons, ["outside_participation_window"]);
    Ok(row)
}

#[test]
fn selected_workbook_service_refuses_profile_population_overrides() -> TestResult {
    if !isolated(
        "selected_options::selected_workbook_service_refuses_profile_population_overrides",
    )? {
        return Ok(());
    }
    let controlled = selected_fixture::install()?;
    let request = ExportJobRequest::SelectedPublicationV1 {
        selection: controlled.selection()?,
    };
    let mut options = selected_options(controlled.profile())?;
    let decoded = decoded_generations();
    let resident = resident_generations();
    let full = build(
        controlled.store(),
        &options,
        "controlled-selected-full",
        &request,
    )?;
    check!(eq; decoded_generations(), decoded.checked_add(1).ok_or("counter overflow")?);
    check!(eq; resident_generations(), resident);
    check_selected_output(
        Path::new(&full.path),
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
        "50.00",
    )?;
    let root = Path::new(&full.path)
        .parent()
        .ok_or("full generation absent")?;
    let bytes = std::fs::read(&full.path)?;
    let manifest = std::fs::read(root.join("manifest.json"))?;
    let exclusion = excluded_49(
        &root.join("participation_exclusions.csv"),
        controlled.candidate_id(),
    )?;
    for (job, limit, scope) in [
        ("controlled-selected-zero", Some(0), Scope::AllSources),
        ("controlled-selected-core", None, Scope::Core),
    ] {
        options.limit = limit;
        options.scope = scope;
        let error = build(controlled.store(), &options, job, &request)
            .err()
            .ok_or("profile population override was silently discarded")?;
        check!(matches!(error, ReportError::Invariant { .. }));
    }
    check!(eq; std::fs::read(&full.path)?, bytes);
    check!(eq; std::fs::read(root.join("manifest.json"))?, manifest);
    check!(eq; excluded_49(&root.join("participation_exclusions.csv"), controlled.candidate_id())?, exclusion);
    check_selected_output(
        Path::new(&full.path),
        controlled.public_id(),
        controlled.candidate_id(),
        controlled.current_school_id(),
        "50.00",
    )?;
    let verified = census_report::workbook::publication::verify_published(Path::new(&full.path))?;
    check!(eq; verified.workbook.mapped_athletes, 1);
    Ok(())
}
