use super::super::canonical::cell_at;
use super::{expected_sheets, Expect};
use crate::export::ExportDataset;
use crate::workbook::verify::expectations::Expectations;
use crate::workbook::verify::verify_frozen;
use crate::workbook::Options;
use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_store::{Store, Table};
use rust_xlsxwriter::Workbook;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const CORRUPTIONS: [(&str, usize, usize, &str); 7] = [
    ("Schools", 1, 1, "Corrupted School"),
    ("Meets", 0, 0, "Corrupted Meets"),
    ("Sources", 1, 0, "corrupted source"),
    ("Coverage", 1, 1, "Corrupted Coverage"),
    ("Conflicts", 0, 0, "Corrupted Conflicts"),
    ("Review", 0, 0, "Corrupted Review"),
    ("Run Metrics", 1, 1, "Corrupted Run Metrics"),
];

#[test]
fn altered_metadata_cells_are_rejected() -> TestResult {
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school)?;

    let options = Options::default();
    let good = crate::workbook::build(&store, &options)?;
    let frozen = good
        .parent()
        .ok_or("missing published generation directory")?
        .join("frozen-input.json");
    let dataset = ExportDataset::reopen_frozen(&frozen)?;
    verify_frozen(&good, &dataset, &options)?;

    let expectations = Expectations::of(&dataset, &options)?;
    let sheets = expected_sheets(&expectations)?;
    let target = dir.path().join("corrupt.xlsx");
    let mut book = Workbook::new();
    let mut applied = 0_usize;
    for (name, mut rows) in sheets {
        applied = applied.saturating_add(tamper(name, &mut rows));
        write_rows(&mut book, name, &rows)?;
    }
    check!(
        applied == CORRUPTIONS.len(),
        "only {applied} of {} metadata corruptions landed on a projected row",
        CORRUPTIONS.len()
    );
    book.save(&target)?;

    let error = match verify_frozen(&target, &dataset, &options) {
        Err(error) => error,
        Ok(_) => return Err("tampered metadata accepted".into()),
    };
    let detail = error.to_string();
    for (sheet, row, column, text) in CORRUPTIONS {
        let location = cell_at(sheet, row, column);
        check!(
            detail.contains(&location),
            "{location} is missing from: {detail}"
        );
        check!(detail.contains(text), "{text} is missing from: {detail}");
    }
    Ok(())
}

fn tamper(name: &str, rows: &mut [Vec<Expect>]) -> usize {
    let mut applied = 0_usize;
    for (sheet, row, column, text) in CORRUPTIONS {
        if sheet != name {
            continue;
        }
        if let Some(cell) = rows.get_mut(row).and_then(|row| row.get_mut(column)) {
            *cell = Expect::Text(text.to_string());
            applied = applied.saturating_add(1);
        }
    }
    applied
}

fn write_rows(book: &mut Workbook, name: &str, rows: &[Vec<Expect>]) -> TestResult {
    let sheet = book.add_worksheet();
    sheet.set_name(name)?;
    for (r, row) in rows.iter().enumerate() {
        let r = u32::try_from(r)?;
        for (c, value) in row.iter().enumerate() {
            let c = u16::try_from(c)?;
            match value {
                Expect::Text(text) => {
                    sheet.write_string(r, c, text.as_str())?;
                }
                Expect::Number(number) => {
                    sheet.write_number(r, c, *number)?;
                }
                Expect::Empty => {}
            }
        }
    }
    Ok(())
}
