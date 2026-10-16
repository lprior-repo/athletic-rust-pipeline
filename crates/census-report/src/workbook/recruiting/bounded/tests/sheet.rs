use super::{bounded_claims, coach, TestResult, WORKBOOK_GROUP_BUDGET};

use crate::report::{Derivation, ReportResult, Scope};
use crate::workbook::cells::{write_sheet, Cell, SheetChrome, SheetLayout};
use crate::workbook::recruiting::{coaches, coalesce, Recruiting};
use calamine::{open_workbook, Data, Range, Reader, Xlsx};
use census_domain::model::{
    normalize_name, CanonicalCoach, CanonicalSchool, CoachContactClaim, CoachContactProgram,
    CoachRole, CoachTenure, CoachTenureEvidence, Gender, RetainedConflict, SchoolYear, SourceRef,
    Sport,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;

fn tenure_evidence(
    coach: &CanonicalCoach,
    mailbox: &str,
    school_year: SchoolYear,
) -> TestResult<CoachTenureEvidence> {
    Ok(CoachTenureEvidence {
        tenure: CoachTenure::Current { school_year },
        source: SourceRef::new(
            "parity_directory",
            Some("https://parity.test/contacts".to_string()),
        ),
        source_sha256: "b".repeat(64),
        retrieved_at: "2026-09-20T00:00:00Z".to_string(),
        statement: "Synthetic parity appointment".to_string(),
        claim: Some(CoachContactClaim {
            coach: coach.id.clone(),
            school: coach.school.clone(),
            role: coach.role,
            program: match coach.role {
                CoachRole::AthleticDirector => CoachContactProgram::SchoolAthletics,
                _ => CoachContactProgram::Team {
                    sport: coach.sport.ok_or("fixture coach has no program")?,
                    gender: coach.gender,
                },
            },
            mailbox: Some(mailbox.to_string()),
        }),
    })
}

fn rendered(range: &Range<Data>) -> Vec<Vec<String>> {
    (0..range.height())
        .map(|row| {
            (0..range.width())
                .map(|column| {
                    range
                        .get((row, column))
                        .map_or_else(String::new, |value| value.to_string())
                })
                .collect()
        })
        .collect()
}

fn column_of(rows: &[Vec<String>], header: &str) -> TestResult<usize> {
    rows.first()
        .and_then(|headers| headers.iter().position(|value| value == header))
        .ok_or_else(|| format!("missing sheet column {header}").into())
}

fn sheet_fixture() -> TestResult<(tempfile::TempDir, Store, Vec<CanonicalCoach>, SchoolYear)> {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let school_year = SchoolYear::new(2026).ok_or("invalid fixture season")?;

    let (west, west_id) = CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Parity West High",
        normalize_name("Parity West High"),
        Some("West"),
    );
    store.append(Table::Schools, &west)?;
    let (east, east_id) = CanonicalSchool::new(
        UsJurisdiction::Minnesota,
        "Parity East High",
        normalize_name("Parity East High"),
        Some("East"),
    );
    store.append(Table::Schools, &east)?;

    let mut repeated = coach(
        &west_id,
        "Ronald Thacker",
        Some(Sport::OutdoorTrack),
        Gender::Mixed,
        CoachRole::HeadCoach,
    );
    repeated.professional_email = Some("first@parity.test".to_string());
    repeated.tenure_evidence = vec![tenure_evidence(
        &repeated,
        "first@parity.test",
        school_year,
    )?];
    repeated.retained_conflicts.push(RetainedConflict::new(
        "Coach identity",
        repeated.id.as_str(),
        "Ronald Thacker",
        "observation rows disagree",
    ));
    store.append(Table::Coaches, &repeated)?;

    let mut disagreed = repeated.clone();
    disagreed.professional_email = Some("second@parity.test".to_string());
    disagreed.tenure_evidence = vec![tenure_evidence(
        &disagreed,
        "second@parity.test",
        school_year,
    )?];
    store.append(Table::Coaches, &disagreed)?;

    let mut girls_head = coach(
        &west_id,
        "Dana Girls",
        Some(Sport::OutdoorTrack),
        Gender::Girls,
        CoachRole::HeadCoach,
    );
    girls_head.professional_email = Some("dana@parity.test".to_string());
    girls_head.tenure_evidence = vec![tenure_evidence(
        &girls_head,
        "dana@parity.test",
        school_year,
    )?];
    store.append(Table::Coaches, &girls_head)?;

    let mut assistant = coach(
        &west_id,
        "Teresa Johnson",
        Some(Sport::CrossCountry),
        Gender::Mixed,
        CoachRole::AssistantCoach,
    );
    assistant.professional_email = Some("teresa@parity.test".to_string());
    assistant.tenure_evidence = vec![tenure_evidence(
        &assistant,
        "teresa@parity.test",
        school_year,
    )?];
    store.append(Table::Coaches, &assistant)?;

    let mut director = coach(
        &east_id,
        "Erik Lund",
        None,
        Gender::Mixed,
        CoachRole::AthleticDirector,
    );
    director.professional_email = Some("erik@parity.test".to_string());
    director.tenure_evidence = vec![tenure_evidence(&director, "erik@parity.test", school_year)?];
    store.append(Table::Coaches, &director)?;

    let dataset = crate::export::ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[])?;
    let observations = derivation.coach_observations().to_vec();
    Ok((dir, store, observations, school_year))
}

#[test]
fn parity_grouping_streams_the_coaches_sheet_as_the_retained_vector_writer() -> TestResult {
    let (dir, store, observations, school_year) = sheet_fixture()?;
    let oracle = coalesce::claims(&observations);
    let bounded = bounded_claims(&observations, WORKBOOK_GROUP_BUDGET)?;
    check!(eq; bounded, oracle, "the fixture folds as the oracle");
    check!(eq; bounded.len(), 4, "four owner ids publish four claims");

    let dataset = crate::export::ExportDataset::load(&store)?;
    let derivation = Derivation::of(&dataset, Scope::AllSources, None, &[])?;
    let admission = crate::report::EventAdmission::of(&dataset)?;
    let projection = Recruiting::of(&derivation, school_year, Vec::new(), &admission)?;

    let streaming_path = dir.path().join("streaming.xlsx");
    let mut book = Workbook::new();
    projection.write_coaches(&mut book, &streaming_path)?;
    book.save(&streaming_path)?;

    let vector_path = dir.path().join("vector.xlsx");
    let rows = coaches::rows(&projection.dataset).collect::<ReportResult<Vec<Vec<Cell>>>>()?;
    let mut book = Workbook::new();
    write_sheet(
        &mut book,
        &vector_path,
        SheetLayout {
            name: coaches::TITLE,
            widths: &coaches::WIDTHS,
            chrome: SheetChrome::Filtered,
        },
        rows,
    )?;
    book.save(&vector_path)?;

    let mut streamed: Xlsx<std::io::BufReader<std::fs::File>> = open_workbook(&streaming_path)?;
    let mut retained: Xlsx<std::io::BufReader<std::fs::File>> = open_workbook(&vector_path)?;
    let streamed_rows = rendered(&streamed.worksheet_range("Coaches")?);
    let retained_rows = rendered(&retained.worksheet_range("Coaches")?);
    check!(eq; streamed_rows.len(), bounded.len().saturating_add(1));
    check!(
        eq;
        streamed_rows,
        retained_rows,
        "the streaming sheet matches the vector writer cell for cell"
    );
    check!(
        eq;
        serde_json::to_string(&streamed_rows)?,
        serde_json::to_string(&retained_rows)?,
        "the streamed sheet serializes byte-for-byte as the vector sheet"
    );

    let id_column = column_of(&streamed_rows, "Coach ID")?;
    let state_column = column_of(&streamed_rows, "Contact Admission State")?;
    let merged = bounded
        .iter()
        .find(|claim| claim.name == "Ronald Thacker")
        .ok_or("the merged coach is missing")?;
    let merged_row = streamed_rows
        .iter()
        .find(|row| {
            row.get(id_column)
                .is_some_and(|id| id == merged.id.as_str())
        })
        .ok_or("the merged coach row is missing")?;
    check!(
        eq;
        merged_row.get(state_column).map(String::as_str),
        Some("mailbox_conflict"),
        "disagreeing observations keep the conflict admission state"
    );
    Ok(())
}
