mod aliases;
mod captured;
mod multiple;
mod ordering;
mod ownership;

use super::*;
use crate::export::postal::tests::{captured_school, CAPTURE_DAY, SUMMARY, SUMMARY_URL};
use crate::export::ExportDataset;
use crate::workbook::{verify::verify_frozen, Options};
use sha2::{Digest, Sha256};

const CAPTURED_ADDRESS: &str = "1 Rocket Drive, Asheville, NC 28803";

fn seed(store: &Store) -> TestResult<(CanonicalSchool, CanonicalAthlete)> {
    let school = captured_school()?;
    store.append(Table::Schools, &school)?;
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Captured School Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "postal-runner"),
    )?;
    athlete.sports = vec![Sport::OutdoorTrack];
    athlete.evidence = evidence("nchsaa", Some(SUMMARY_URL));
    publish_fixture_cohort(&mut athlete, "nchsaa", "captured-postal", CAPTURE_DAY);
    store.append(Table::Athletes, &athlete)?;
    coach(
        store,
        &school.id,
        "Published Contact",
        Some(Sport::OutdoorTrack),
        CoachRole::HeadCoach,
        "published@school.test",
    )?;
    Ok((school, athlete))
}

fn published(store: &Store) -> TestResult<(std::path::PathBuf, ExportDataset, Options)> {
    let options = Options {
        address_corpus: None,
        grad_year: Some(2027),
        out: None,
        limit: None,
        scope: crate::report::Scope::AllSources,
        school_year: SchoolYear::new(2026).ok_or("invalid fixture season")?,
        jurisdictions: Vec::new(),
    };
    let path = crate::workbook::build(store, &options)?;
    let generation = path.parent().ok_or("missing generation directory")?;
    let dataset = ExportDataset::reopen_frozen(&generation.join("frozen-input.json"))?;
    verify_frozen(&path, &dataset, &options)?;
    Ok((path, dataset, options))
}

fn expected_postal(school: &CanonicalSchool) -> [(&'static str, String); 12] {
    [
        ("Postal School ID", school.id.to_string()),
        ("Postal Street", "1 Rocket Drive".into()),
        ("Postal Second Line", "".into()),
        ("Postal City", "Asheville".into()),
        ("Postal State", "NC".into()),
        ("Postal ZIP", "28803".into()),
        ("Postal Owner Namespace", "association_school:nchsaa".into()),
        ("Postal Owner ID", "ZCUM49".into()),
        ("Postal Source", "athletic-association:NC".into()),
        ("Postal Source URL", SUMMARY_URL.into()),
        ("Postal Observed Date", CAPTURE_DAY.into()),
        (
            "Postal Capture SHA256",
            format!("{:x}", Sha256::digest(SUMMARY)),
        ),
    ]
}

fn corrupt_workbook(
    source: &std::path::Path,
    target: &std::path::Path,
    name: &str,
    header: &str,
    replacement: &str,
) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = Workbook::new();
    for sheet_name in reader.sheet_names() {
        let range = reader.worksheet_range(&sheet_name)?;
        let selected = if sheet_name == name {
            Some(column_of(&range, header)?)
        } else {
            None
        };
        let sheet = writer.add_worksheet();
        sheet.set_name(&sheet_name)?;
        for (row, cells) in range.rows().enumerate() {
            for (column, value) in cells.iter().enumerate() {
                let r = u32::try_from(row)?;
                let c = u16::try_from(column)?;
                if row == 1 && selected == Some(column) {
                    sheet.write_string(r, c, replacement)?;
                    continue;
                }
                match value {
                    Data::Empty => {}
                    Data::Float(value) => {
                        sheet.write_number(r, c, *value)?;
                    }
                    Data::Int(value) => {
                        sheet.write_number(r, c, *value as f64)?;
                    }
                    _ => {
                        sheet.write_string(r, c, value.to_string())?;
                    }
                }
            }
        }
    }
    writer.save(target)?;
    Ok(())
}
