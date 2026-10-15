use super::*;
use crate::export::current_school::fixture::{bind_resolution, capture, capture_with_school};
use crate::export::public_identity::tests::fixture::source_births;
use crate::export::CurrentAffiliationResolution;
use census_domain::model::EventSemantics;

#[test]
fn selected_bests_refuse_every_unresolved_capture_day_affiliation_without_artifact_effects(
) -> TestResult {
    let fixture = source_births()?;
    let [first, _, _] = &fixture.athletes;
    let rows = [super::public_identity::row(first)];
    let output = fixture.directory.path().join("existing-affiliation-output");
    std::fs::create_dir(&output)?;
    let jsonl = output.join("best-results-co2027.jsonl");
    let csv = output.join("best-results-co2027.csv");
    std::fs::write(&jsonl, b"previous jsonl\n")?;
    std::fs::write(&csv, b"previous csv\n")?;
    for resolution in [
        CurrentAffiliationResolution::Unknown,
        CurrentAffiliationResolution::Conflicted,
        CurrentAffiliationResolution::CoopCampusUnresolved,
    ] {
        let (mut dataset, _) = capture(&fixture.store)?;
        bind_resolution(&mut dataset, resolution)?;
        let result = super::super::selected::write(
            &output,
            &rows,
            "co2027",
            EventSemantics::ResultGroups21,
            &dataset,
        );
        check!(result.is_err());
        check!(eq; std::fs::read(&jsonl)?.as_slice(), b"previous jsonl\n");
        check!(eq; std::fs::read(&csv)?.as_slice(), b"previous csv\n");
    }
    Ok(())
}

#[test]
fn selected_bests_label_historical_school_separately_from_pinned_current_school() -> TestResult {
    let fixture = source_births()?;
    let (dataset, _) = capture(&fixture.store)?;
    let [first, _, _] = &fixture.athletes;
    let rows = [super::public_identity::row(first)];
    let original = serde_json::to_value(&rows)?;
    let (jsonl, csv) = super::super::selected::write(
        &fixture.directory.path().join("separate-affiliations"),
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        &dataset,
    )?;
    let wire: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(jsonl)?)?;
    check!(eq; wire.get("current_state"), Some(&serde_json::json!("WI")));
    check!(eq; wire.get("current_school_as_of"), Some(&serde_json::json!("2026-10-01")));
    let mut reader = csv::Reader::from_path(csv)?;
    let headers = reader.headers()?.clone();
    let record = reader.records().next().ok_or("CSV row")??;
    check!(eq; record.get(column(&headers, "school_at_performance")?), Some("Synthetic School"));
    check!(eq; record.get(column(&headers, "current_school")?), Some("Synthetic Current Wisconsin High"));
    check!(eq; record.get(column(&headers, "current_state")?), Some("WI"));
    check!(!headers
        .iter()
        .any(|name| name == "school" || name == "state"));
    check!(eq; serde_json::to_value(&rows)?, original);
    Ok(())
}

#[test]
fn selected_bests_refuse_absent_resolved_current_school_without_artifact_effects() -> TestResult {
    let fixture = source_births()?;
    let (mut dataset, _) = capture(&fixture.store)?;
    let absent = census_domain::model::SchoolId::mint("sch", &["absent-current-school"]);
    check!(!dataset.schools.contains_key(&absent));
    bind_resolution(
        &mut dataset,
        CurrentAffiliationResolution::Resolved {
            school: absent,
            evidence_refs: vec!["synthetic-accepted-current-affiliation".into()],
        },
    )?;
    let [first, _, _] = &fixture.athletes;
    refuses_preserving_artifacts(
        fixture.directory.path(),
        &dataset,
        first,
        "selected best current school is absent from the frozen input",
    )
}

#[test]
fn selected_bests_refuse_unknown_current_school_jurisdiction_without_artifact_effects() -> TestResult
{
    let fixture = source_births()?;
    let (mut school, _) = census_domain::model::CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Synthetic Unknown Jurisdiction High",
        "synthetic unknown jurisdiction high",
        None,
    );
    school.state = None;
    let (dataset, _) = capture_with_school(&fixture.store, school)?;
    let [first, _, _] = &fixture.athletes;
    refuses_preserving_artifacts(
        fixture.directory.path(),
        &dataset,
        first,
        "selected best current school jurisdiction is unknown",
    )
}

fn refuses_preserving_artifacts(
    root: &std::path::Path,
    dataset: &crate::export::ExportDataset,
    athlete: &census_domain::model::CanonicalAthlete,
    expected: &str,
) -> TestResult {
    let output = root.join("existing-current-school-output");
    std::fs::create_dir(&output)?;
    let jsonl = output.join("best-results-co2027.jsonl");
    let csv = output.join("best-results-co2027.csv");
    std::fs::write(&jsonl, b"previous jsonl\n")?;
    std::fs::write(&csv, b"previous csv\n")?;
    let rows = [super::public_identity::row(athlete)];
    let result = super::super::selected::write(
        &output,
        &rows,
        "co2027",
        EventSemantics::ResultGroups21,
        dataset,
    );
    match result {
        Err(crate::report::ReportError::Store(census_store::StoreError::Invariant { detail })) => {
            check!(eq; detail, expected);
        }
        Err(error) => return Err(format!("unexpected current-school refusal: {error}").into()),
        Ok(_) => {
            return Err("unusable current school authorized selected artifact replacement".into())
        }
    }
    check!(eq; std::fs::read(&jsonl)?.as_slice(), b"previous jsonl\n");
    check!(eq; std::fs::read(&csv)?.as_slice(), b"previous csv\n");
    Ok(())
}
