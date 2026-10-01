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
fn altered_metadata_cells_are_rejected() {
    let dir = tempfile::tempdir().expect("a temp dir");
    let store = Store::open(dir.path()).expect("a store");
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Abbotsford", "abbotsford");
    store.append(Table::Schools, &school).expect("a school row");

    let options = Options::default();
    let good = crate::workbook::build(&store, &options).expect("the untampered workbook builds");
    let frozen = good
        .parent()
        .expect("the published generation directory")
        .join("frozen-input.json");
    let dataset = ExportDataset::reopen_frozen(&frozen).expect("the published frozen input");
    verify_frozen(&good, &dataset, &options).expect("the untampered workbook verifies");

    let expectations = Expectations::of(&dataset, &options).expect("the frozen expectations");
    let sheets = expected_sheets(&expectations).expect("expected rows");
    let target = dir.path().join("corrupt.xlsx");
    let mut book = Workbook::new();
    let mut applied = 0_usize;
    for (name, mut rows) in sheets {
        applied = applied.saturating_add(tamper(name, &mut rows));
        write_rows(&mut book, name, &rows);
    }
    assert!(
        applied == CORRUPTIONS.len(),
        "only {applied} of {} metadata corruptions landed on a projected row",
        CORRUPTIONS.len()
    );
    book.save(&target).expect("the corrupt workbook saves");

    let error = verify_frozen(&target, &dataset, &options).expect_err("tampering is rejected");
    let detail = error.to_string();
    for (sheet, row, column, text) in CORRUPTIONS {
        let location = cell_at(sheet, row, column);
        assert!(
            detail.contains(&location),
            "{location} is missing from: {detail}"
        );
        assert!(detail.contains(text), "{text} is missing from: {detail}");
    }
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

fn write_rows(book: &mut Workbook, name: &str, rows: &[Vec<Expect>]) {
    let sheet = book.add_worksheet();
    sheet.set_name(name).expect("a sheet name");
    for (r, row) in rows.iter().enumerate() {
        let r = u32::try_from(r).expect("a row index");
        for (c, value) in row.iter().enumerate() {
            let c = u16::try_from(c).expect("a column index");
            match value {
                Expect::Text(text) => {
                    sheet
                        .write_string(r, c, text.as_str())
                        .expect("a text cell");
                }
                Expect::Number(number) => {
                    sheet.write_number(r, c, *number).expect("a number cell");
                }
                Expect::Empty => {}
            }
        }
    }
}
