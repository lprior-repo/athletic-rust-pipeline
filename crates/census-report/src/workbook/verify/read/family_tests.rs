mod fixture;
mod publication;

use super::tests::{alter_sheet, replace_cell, TestResult};
use super::{Book, Budget, Readback, EXCEL_COLUMNS_PER_SHEET, EXCEL_ROWS_PER_SHEET};
use crate::report::{ReportError, ReportResult};
use crate::workbook::verify::canonical::Value;
use fixture::{collect, refused, workbook};

#[test]
fn two_pages_preserve_the_header_and_first_continuation_record() -> TestResult {
    let directory = tempfile::tempdir()?;
    for lane in [Readback::InlineProfile, Readback::LegacySharedStrings] {
        let path = directory.path().join(format!("two-{lane:?}.xlsx"));
        workbook(
            &path,
            &[
                ("Review", &[("Alpha", 11.5), ("Beta", 12.25)]),
                ("Review_002", &[("Gamma", 13.75)]),
            ],
            lane,
        )?;
        let mut book = Book::open(&path)?;
        check!(eq; book.readback(), lane);
        let (shape, cells) = collect(&mut book, Budget::family(3)?)?;
        check!(eq; shape, Some(4));
        check!(eq; cells, vec![
            (0, Value::text("Name"), Value::text("Mark")),
            (1, Value::text("Alpha"), Value::Number(11.5)),
            (2, Value::text("Beta"), Value::Number(12.25)),
            (3, Value::text("Gamma"), Value::Number(13.75)),
        ]);
    }
    Ok(())
}

#[test]
fn three_variable_pages_are_read_in_canonical_not_workbook_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    for lane in [Readback::InlineProfile, Readback::LegacySharedStrings] {
        let path = directory.path().join(format!("three-{lane:?}.xlsx"));
        workbook(
            &path,
            &[
                ("Review_003", &[("Delta", 14.0), ("Epsilon", 15.5)]),
                ("Review", &[("Alpha", 11.5)]),
                ("Review_002", &[("Beta", 12.25), ("Gamma", 13.75)]),
            ],
            lane,
        )?;
        let mut book = Book::open(&path)?;
        let (shape, cells) = collect(&mut book, Budget::family(5)?)?;
        check!(eq; shape, Some(6));
        check!(eq; cells, vec![
            (0, Value::text("Name"), Value::text("Mark")),
            (1, Value::text("Alpha"), Value::Number(11.5)),
            (2, Value::text("Beta"), Value::Number(12.25)),
            (3, Value::text("Gamma"), Value::Number(13.75)),
            (4, Value::text("Delta"), Value::Number(14.0)),
            (5, Value::text("Epsilon"), Value::Number(15.5)),
        ]);
    }
    Ok(())
}

#[test]
fn single_page_and_absent_family_keep_their_original_meaning() -> TestResult {
    let directory = tempfile::tempdir()?;
    for lane in [Readback::InlineProfile, Readback::LegacySharedStrings] {
        let path = directory.path().join(format!("single-{lane:?}.xlsx"));
        workbook(&path, &[("Review", &[("Original", 10.0)])], lane)?;
        let mut book = Book::open(&path)?;
        check!(eq; book.readback(), lane);
        let (shape, cells) = collect(&mut book, Budget::family(1)?)?;
        check!(eq; shape, Some(2));
        check!(eq; cells, vec![
            (0, Value::text("Name"), Value::text("Mark")),
            (1, Value::text("Original"), Value::Number(10.0)),
        ]);
        let mut visited = Vec::new();
        let absent = book.read_family("Schools", Budget::family(0)?, |row| {
            visited.push(row.index())
        })?;
        check!(eq; absent.map(|shape| shape.rows), None);
        check!(eq; visited, Vec::<usize>::new());
    }
    Ok(())
}

#[test]
fn orphan_gapped_and_noncanonical_names_refuse_before_consuming_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cases: &[&[&str]] = &[
        &["Review_002"],
        &["Review_003"],
        &["Review", "Review_003"],
        &["Review", "Review_002", "Review_004"],
        &["Review", "Review_001"],
        &["Review", "Review_2"],
        &["Review", "Review_0002"],
        &["Review", "Review_notes"],
        &["Review", "Review_999999999999999999999999"],
    ];
    let data = [("Alpha", 11.5)];
    for (index, names) in cases.iter().enumerate() {
        let path = directory.path().join(format!("names-{index}.xlsx"));
        let pages: Vec<_> = names.iter().map(|name| (*name, data.as_slice())).collect();
        workbook(&path, &pages, Readback::InlineProfile)?;
        let mut book = Book::open(&path)?;
        let mut visited = Vec::new();
        let result = book.read_family("Review", Budget::family(20)?, |row| {
            visited.push(row.index())
        });
        refused(result, &path)?;
        check!(eq; visited, Vec::<usize>::new());
    }
    Ok(())
}

#[test]
fn complete_continuation_headers_include_extra_and_missing_cells() -> TestResult {
    let directory = tempfile::tempdir()?;
    for mode in ["changed", "extra", "missing"] {
        let path = directory.path().join(format!("header-{mode}.xlsx"));
        workbook(
            &path,
            &[
                ("Review", &[("Alpha", 11.5)]),
                ("Review_002", &[("Beta", 12.25)]),
            ],
            Readback::InlineProfile,
        )?;
        alter_sheet(&path, "Review_002", |range| {
            match mode {
                "changed" => replace_cell(range, (0, 1), "Other mark"),
                "extra" => replace_cell(range, (0, 2), "Hidden header"),
                _ => range.set_value((0, 0), calamine::Data::Empty),
            }
            Ok(())
        })?;
        let mut book = Book::open(&path)?;
        let mut visited = Vec::new();
        let result = book.read_family("Review", Budget::family(2)?, |row| {
            visited.push(row.index())
        });
        refused(result, &path)?;
        check!(eq; visited, vec![0, 1]);
    }
    Ok(())
}

#[test]
fn global_budget_stops_before_the_first_excess_logical_row() -> TestResult {
    let directory = tempfile::tempdir()?;
    for lane in [Readback::InlineProfile, Readback::LegacySharedStrings] {
        let path = directory.path().join(format!("budget-{lane:?}.xlsx"));
        workbook(
            &path,
            &[
                ("Review", &[("Alpha", 11.5)]),
                ("Review_002", &[("Beta", 12.25), ("Gamma", 13.75)]),
                ("Review_003", &[("Delta", 14.0), ("Epsilon", 15.5)]),
            ],
            lane,
        )?;
        let mut book = Book::open(&path)?;
        let mut visited = Vec::new();
        let result = book.read_family("Review", Budget::family(4)?, |row| {
            visited.push(row.index())
        });
        refused(result, &path)?;
        check!(eq; visited, vec![0, 1, 2, 3, 4]);
    }
    Ok(())
}

#[test]
fn header_only_extra_page_is_not_an_ignorable_part() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("empty-part.xlsx");
    workbook(
        &path,
        &[("Review", &[("Alpha", 11.5)]), ("Review_002", &[])],
        Readback::InlineProfile,
    )?;
    let mut book = Book::open(&path)?;
    refused(
        book.read_family("Review", Budget::family(1)?, |_| {}),
        &path,
    )?;
    Ok(())
}

#[test]
fn only_logical_family_budgets_may_exceed_the_excel_row_envelope() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("envelope.xlsx");
    workbook(
        &path,
        &[("Review", &[("Alpha", 11.5)])],
        Readback::InlineProfile,
    )?;
    let mut book = Book::open(&path)?;
    let budget = Budget {
        rows: EXCEL_ROWS_PER_SHEET.checked_add(1).ok_or("row overflow")?,
        columns: 2,
    };
    let (shape, cells) = collect(&mut book, budget)?;
    check!(eq; shape, Some(2));
    check!(eq; cells.len(), 2);
    let mut visited = Vec::new();
    let error = book
        .read("Review", budget, |row| visited.push(row.index()))
        .err()
        .ok_or("physical reader accepted a logical budget")?;
    check!(matches!(error, ReportError::VerifierInput { .. }));
    check!(eq; visited, Vec::<usize>::new());
    Ok(())
}

#[test]
fn family_budget_rejects_zero_dimensions_and_excess_columns() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("invalid-budget.xlsx");
    workbook(
        &path,
        &[("Review", &[("Alpha", 11.5)])],
        Readback::InlineProfile,
    )?;
    let mut book = Book::open(&path)?;
    for budget in [
        Budget {
            rows: 0,
            columns: 2,
        },
        Budget {
            rows: 2,
            columns: 0,
        },
        Budget {
            rows: 2,
            columns: EXCEL_COLUMNS_PER_SHEET
                .checked_add(1)
                .ok_or("column overflow")?,
        },
    ] {
        let mut visited = Vec::new();
        let result: ReportResult<_> =
            book.read_family("Review", budget, |row| visited.push(row.index()));
        let error = result.err().ok_or("invalid budget accepted")?;
        check!(matches!(error, ReportError::VerifierInput { .. }));
        check!(eq; visited, Vec::<usize>::new());
    }
    let error = Budget::family(usize::MAX)
        .err()
        .ok_or("overflow budget accepted")?;
    check!(matches!(error, ReportError::VerifierInput { .. }));
    Ok(())
}
