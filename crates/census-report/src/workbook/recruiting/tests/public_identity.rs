use super::*;
use crate::export::selected_test_fixture::Fixture as SelectedGraphFixture;
use crate::workbook::publication::selected::fixture::Fixture as SelectedFixture;

#[tokio::test]
async fn selected_recruiting_csv_and_sheets_publish_public_ids_with_explicit_candidate_audit(
) -> TestResult {
    let fixture = SelectedGraphFixture::composed(true).await?;
    let objects = tempfile::tempdir()?;
    let captured = fixture.capture(objects.path())?;
    let school_year = SchoolYear::new(2026).ok_or("school year")?;
    let derivation = Derivation::of(
        &captured,
        Scope::AllSources,
        Some(2027),
        &[UsJurisdiction::Alabama, UsJurisdiction::Wisconsin],
    )?;
    let admission = crate::report::EventAdmission::of(&captured)?;
    let dataset = dataset::Dataset::of(&derivation, school_year, Vec::new(), &admission)?;
    let candidate = fixture.roster_athlete.as_str();
    let public_id = captured
        .verified_public_identities()?
        .public_id(candidate)
        .ok_or("public identity")?;
    check!(ne; public_id.as_str(), candidate);
    let rows = athletes::rows(&dataset).collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.len(), 2);
    let header = rows.first().ok_or("athlete headers")?;
    check!(eq; header.len(), athletes::HEADERS.len() + 2);
    let row = rows.get(1).ok_or("athlete cells")?;
    check!(
        matches!(row.first(), Some(crate::workbook::cells::Cell::Text(value)) if value == public_id.as_str())
    );
    let candidate_column = athletes::HEADERS.len();
    check!(
        matches!(row.get(candidate_column), Some(crate::workbook::cells::Cell::Text(value)) if value == candidate)
    );
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("recruiting.csv");
    write_recruiting_csv(&derivation, school_year, &path)?;
    let mut reader = ::csv::Reader::from_path(&path)?;
    let headers = reader.headers()?.clone();
    let mut records = reader.records();
    let record = records.next().ok_or("recruiting row")??;
    check!(records.next().transpose()?.is_none());
    check!(eq; record.get(0), Some(public_id.as_str()));
    let column = headers
        .iter()
        .position(|name| name == "canonical_candidate_id")
        .ok_or("candidate column")?;
    check!(eq; record.get(column), Some(candidate));
    Ok(())
}

#[test]
fn selected_pr_sheet_keeps_public_key_and_raw_canonical_candidate_separate() -> TestResult {
    let fixture = SelectedFixture::new()?;
    let athlete = fixture.dataset.athletes.first().ok_or("candidate")?;
    let selection = crate::bests::public_identity_selection_for_test(
        athlete,
        census_domain::model::EventSemantics::from_revision(
            fixture.dataset.lineage.policy_revision,
        )
        .ok_or("captured event policy missing")?,
    );
    let view = fixture.dataset.verified_public_identities()?;
    let public_id = view
        .public_id(athlete.id.as_str())
        .ok_or("public identity")?;
    let schools = crate::export::current_school::SelectedSchools::of(&fixture.dataset)?;
    let rows = prs::rows(std::slice::from_ref(&selection), Some(view), Some(&schools))
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; rows.first().ok_or("PR header")?.len(), prs::HEADERS.len() + 5);
    let row = rows.get(1).ok_or("PR data")?;
    check!(
        matches!(row.first(), Some(crate::workbook::cells::Cell::Text(value)) if value == public_id.as_str())
    );
    check!(
        matches!(row.get(prs::HEADERS.len()), Some(crate::workbook::cells::Cell::Text(value)) if value == athlete.id.as_str())
    );
    Ok(())
}
