use super::*;

mod readback;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn audit_csv_preserves_capture_day_states_unknown_reasons_and_independent_references() -> TestResult
{
    let fixture = fixture::fixture()?;
    let directory = tempfile::tempdir()?;
    check!(eq; write(directory.path(), &fixture.dataset)?, 7);
    check!(eq; verify(directory.path(), &fixture.dataset)?, 7);
    let mut reader = csv::Reader::from_path(directory.path().join(FILE))?;
    check!(eq; reader.headers()?.iter().collect::<Vec<_>>(), HEADER);
    let rows = reader.records().collect::<Result<Vec<_>, _>>()?;
    check!(rows
        .windows(2)
        .all(|pair| matches!(pair, [left, right] if left.get(0) < right.get(0))));
    let affiliations = fixture.dataset.verified_current_affiliations()?;
    for athlete in &fixture.dataset.athletes {
        let index: usize = athlete
            .source
            .as_ref()
            .and_then(|source| source.id.strip_prefix("96"))
            .ok_or("synthetic audit source")?
            .parse()?;
        let row = rows
            .iter()
            .find(|row| row.get(0) == Some(athlete.id.as_str()))
            .ok_or("missing audit subject")?;
        check!(eq; row.get(2), Some(affiliations.resolution(&athlete.id).status()));
        check!(eq; row.get(5), Some(affiliations.profile_id().as_str()));
        check!(eq; row.get(6), Some("2026-10-01"));
        check!(eq; row.get(7), Some(fixture.dataset.lineage.input_generation.as_str()));
        if index == 3 || index == 5 {
            let reason = if index == 3 {
                "outside_capture_day"
            } else {
                "no_retained_row"
            };
            check!(eq; row.get(2), Some("current_affiliation_unknown"));
            check!(eq; row.get(3), Some("[]"));
            check!(eq; row.get(4), Some(format!("[\"{reason}\"]").as_str()));
        } else if index < 6 {
            check!(eq; row.get(3), Some(format!("[\"synthetic-audit-reference-{index}\",\"synthetic-shared-reference\"]").as_str()));
            check!(eq; row.get(4), Some("[\"outside_selected_sport\"]"));
        } else {
            check!(eq; row.get(3), Some("[]"));
            check!(eq; row.get(4), Some("[\"missing_current_affiliation_evidence\"]"));
        }
        check!(eq; row.get(1), Some(if index == 0 { athlete.school.as_str() } else { "" }));
    }
    Ok(())
}

#[test]
fn current_affiliation_csv_requires_retained_runtime_authority_before_creating_output() -> TestResult
{
    let fixture = fixture::fixture()?;
    let directory = tempfile::tempdir()?;
    let frozen = directory.path().join("frozen.json");
    fixture.dataset.save_frozen(&frozen)?;
    let reopened = ExportDataset::reopen_frozen(&frozen)?;
    let absent = directory.path().join("uncreated");
    check!(write(&absent, &reopened).is_err());
    check!(!absent.exists());
    check!(verify(directory.path(), &reopened).is_err());
    Ok(())
}
