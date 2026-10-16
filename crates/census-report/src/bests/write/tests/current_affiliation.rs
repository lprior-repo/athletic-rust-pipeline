use super::*;
use crate::export::current_school::fixture::{bind_resolution, capture_with_school};
use crate::export::current_school::SelectedSchools;
use crate::export::selected_test_fixture::consumer::{
    capture, source_births, source_births_with, RosterCase,
};
use crate::export::CurrentAffiliationResolution;
use census_domain::model::EventSemantics;

#[test]
fn selected_bests_refuse_every_unresolved_capture_day_affiliation_without_artifact_effects(
) -> TestResult {
    for (case, expected) in [
        (RosterCase::Missing, CurrentAffiliationResolution::Unknown),
        (RosterCase::Undated, CurrentAffiliationResolution::Unknown),
        (
            RosterCase::OutsideCaptureDay,
            CurrentAffiliationResolution::Unknown,
        ),
        (
            RosterCase::Conflicted,
            CurrentAffiliationResolution::Conflicted,
        ),
        (
            RosterCase::Coop,
            CurrentAffiliationResolution::CoopCampusUnresolved,
        ),
    ] {
        let fixture = source_births_with(case)?;
        let (dataset, _proofs) = capture(&fixture.store)?;
        let [first, _, _] = &fixture.athletes;
        let affiliations = dataset.verified_current_affiliations()?;
        check!(eq; affiliations.resolution(&first.id), &expected);
        let audit = affiliations.audit(&first.id).ok_or("affiliation audit")?;
        match case {
            RosterCase::Missing => check!(audit
                .unavailable()
                .iter()
                .any(|gap| gap == "no_retained_row")),
            RosterCase::Undated => check!(audit
                .unavailable()
                .iter()
                .any(|gap| gap == "missing_temporal_authority")),
            RosterCase::OutsideCaptureDay => check!(audit
                .unavailable()
                .iter()
                .any(|gap| gap == "outside_capture_day")),
            RosterCase::Coop | RosterCase::Conflicted => check!(!audit.references().is_empty()),
            RosterCase::Current => return Err("resolved fixture entered unresolved cases".into()),
        }
        refuses_preserving_artifacts(
            fixture.directory.path(),
            &dataset,
            first,
            "selected best has no resolved current school at the pinned cutoff",
        )?;
    }
    Ok(())
}

#[test]
fn selected_bests_label_historical_school_separately_from_pinned_current_school() -> TestResult {
    let fixture = source_births()?;
    let (dataset, _proofs) = capture(&fixture.store)?;
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
fn selected_school_projection_refuses_absent_resolved_current_school() -> TestResult {
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    let (mut dataset, _) = crate::export::current_school::fixture::capture(&fixture.store)?;
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
    refuses_projection(
        &dataset,
        first,
        "selected best current school is absent from the frozen input",
    )
}

#[test]
fn selected_school_projection_refuses_unknown_current_school_jurisdiction() -> TestResult {
    let fixture = crate::export::public_identity::tests::fixture::source_births()?;
    let (mut school, _) = census_domain::model::CanonicalSchool::new(
        census_domain::UsJurisdiction::Wisconsin,
        "Synthetic Unknown Jurisdiction High",
        "synthetic unknown jurisdiction high",
        None,
    );
    school.state = None;
    let (dataset, _) = capture_with_school(&fixture.store, school)?;
    let [first, _, _] = &fixture.athletes;
    refuses_projection(
        &dataset,
        first,
        "selected best current school jurisdiction is unknown",
    )
}

fn refuses_projection(
    dataset: &crate::export::ExportDataset,
    athlete: &census_domain::model::CanonicalAthlete,
    expected: &str,
) -> TestResult {
    match SelectedSchools::of(dataset)?.fields(athlete.id.as_str()) {
        Err(census_store::StoreError::Invariant { detail }) => check!(eq; detail, expected),
        Err(error) => return Err(format!("unexpected school projection refusal: {error}").into()),
        Ok(_) => return Err("unusable school passed selected projection".into()),
    }
    Ok(())
}

fn refuses_preserving_artifacts(
    root: &std::path::Path,
    dataset: &crate::export::ExportDataset,
    athlete: &census_domain::model::CanonicalAthlete,
    expected: &str,
) -> TestResult {
    dataset.ensure_current_selected_write()?;
    let missing = root.join("missing-current-school-output");
    let output = root.join("existing-current-school-output");
    std::fs::create_dir(&output)?;
    let jsonl = output.join("best-results-co2027.jsonl");
    let csv = output.join("best-results-co2027.csv");
    std::fs::write(&jsonl, b"previous jsonl\n")?;
    std::fs::write(&csv, b"previous csv\n")?;
    let rows = [super::public_identity::row(athlete)];
    for destination in [&missing, &output] {
        let result = super::super::selected::write(
            destination,
            &rows,
            "co2027",
            EventSemantics::ResultGroups21,
            dataset,
        );
        match result {
            Err(crate::report::ReportError::Store(census_store::StoreError::Invariant {
                detail,
            })) => {
                check!(eq; detail, expected);
            }
            Err(error) => return Err(format!("unexpected current-school refusal: {error}").into()),
            Ok(_) => return Err("unusable current school authorized selected output".into()),
        }
    }
    check!(!missing.exists());
    check!(eq; std::fs::read(&jsonl)?.as_slice(), b"previous jsonl\n");
    check!(eq; std::fs::read(&csv)?.as_slice(), b"previous csv\n");
    Ok(())
}
