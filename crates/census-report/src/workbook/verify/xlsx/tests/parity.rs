use super::fixture;
use super::TestResult;
use crate::report::CorruptionClass;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::read::tests::{alter_sheet, copy_bundle, publication};
use crate::workbook::verify::read::{
    Book, Budget, Readback, EXCEL_COLUMNS_PER_SHEET, EXCEL_ROWS_PER_SHEET,
};
use crate::workbook::verify::xlsx::{Archive, Profile};
use calamine::{Data, Reader};
use std::collections::BTreeMap;

#[test]
fn xlsx_profile_reads_every_cell_of_the_production_workbook() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (path, _dataset, _options) = publication(directory.path())?;
    let mut book = Book::open(&path)?;
    check!(eq; book.readback(), Readback::InlineProfile, "the production workbook is inline strings");
    let mut source: calamine::Xlsx<_> = calamine::open_workbook(&path)?;
    let names = book.names();
    check!(!names.is_empty(), "the workbook holds sheets");
    let mut total = 0_u64;
    for name in names {
        let range = source.worksheet_range(&name)?;
        let expected: BTreeMap<(usize, usize), Value> = range
            .used_cells()
            .filter_map(|(row, column, value)| {
                let position = (row, column);
                Some((position, value_of(value)?))
            })
            .filter(|(_, value)| !value.is_empty())
            .collect();
        let mut found: BTreeMap<(usize, usize), Value> = BTreeMap::new();
        let mut visited: Vec<(usize, bool)> = Vec::new();
        let shape = book.read(
            &name,
            Budget {
                rows: EXCEL_ROWS_PER_SHEET,
                columns: EXCEL_COLUMNS_PER_SHEET,
            },
            |row| {
                visited.push((row.index(), row.blank()));
                for (column, value) in row.extras(0) {
                    found.insert((row.index(), column), value.clone());
                }
            },
        )?;
        let shape = shape.ok_or_else(|| format!("sheet {name} is missing from the workbook"))?;
        check!(eq; found, expected, "{name} read cell by cell");
        total = total.saturating_add(u64::try_from(found.len()).map_or(u64::MAX, |count| count));
        for (position, (index, _)) in visited.iter().enumerate() {
            check!(eq; *index, position, "{name} visits every row in order");
        }
        check!(eq; visited.len(), shape.rows, "{name} row count");
        let last = expected
            .keys()
            .map(|(row, _)| *row)
            .max()
            .map_or(0, |row| row);
        check!(
            visited.len() > last,
            "{name}: {} visited rows stop before row {last}",
            visited.len()
        );
        for (index, blank) in &visited {
            if !expected.keys().any(|(row, _)| row == index) {
                check!(*blank, "{name} padding row {index} holds values");
            }
        }
    }
    check!(total > 0, "the production workbook holds cells");
    Ok(())
}

#[test]
fn verify_corrupt_shared_string_workbook_is_refused_by_the_inline_profile() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    let path = copy_bundle(&good, &directory.path().join("shared"))?;
    alter_sheet(&path, "Run Metrics", |_| Ok(()))?;
    let archive = Archive::open(&path, Profile::inline())?;
    check!(
        archive.uses_shared_strings(),
        "the rebuilt workbook must store shared strings"
    );
    let error = fixture::failure(&archive, "Run Metrics")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("sharedStrings.xml"), "{error}");
    let mut legacy = Book::open(&path)?;
    check!(
        eq;
        legacy.readback(),
        Readback::LegacySharedStrings,
        "the legacy lane serves shared strings"
    );
    let mut inline = Book::open(&good)?;
    check!(eq; inline.readback(), Readback::InlineProfile);
    check!(
        eq;
        collect(&mut legacy, "Run Metrics")?,
        collect(&mut inline, "Run Metrics")?,
        "both lanes read the same cells"
    );
    crate::workbook::verify::verify_frozen(&path, &dataset, &options)?;
    Ok(())
}

fn collect(book: &mut Book, name: &str) -> TestResult<BTreeMap<(usize, usize), Value>> {
    let mut found: BTreeMap<(usize, usize), Value> = BTreeMap::new();
    let shape = book.read(
        name,
        Budget {
            rows: EXCEL_ROWS_PER_SHEET,
            columns: EXCEL_COLUMNS_PER_SHEET,
        },
        |row| {
            for (column, value) in row.extras(0) {
                found.insert((row.index(), column), value.clone());
            }
        },
    )?;
    shape.ok_or_else(|| format!("sheet {name} is missing from the workbook"))?;
    Ok(found)
}

fn value_of(data: &Data) -> Option<Value> {
    match data {
        Data::Empty => None,
        Data::String(text) => Some(Value::text(text)),
        Data::Float(number) => Some(Value::Number(*number)),
        Data::Int(number) => Some(Value::Number(f64::from(
            i32::try_from(*number).map_or(i32::MAX, |number| number),
        ))),
        Data::Bool(value) => Some(Value::text(value.to_string())),
        _ => None,
    }
}
