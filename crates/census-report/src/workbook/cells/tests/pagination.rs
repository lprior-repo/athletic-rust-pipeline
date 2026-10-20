use super::super::{write_paged_with_limit, Cell, SheetChrome, SheetLayout};
use crate::report::{ReportError, ReportResult};
use calamine::{open_workbook, Reader, Xlsx};

type TestResult = Result<(), Box<dyn std::error::Error>>;
const HEADER: [&str; 2] = ["Key", "Value"];
const ONE: [&str; 2] = ["one", "first"];
const TWO: [&str; 2] = ["two", "second"];
const THREE: [&str; 2] = ["three", "third"];
const FOUR: [&str; 2] = ["four", "fourth"];
const FIVE: [&str; 2] = ["five", "fifth"];
const INPUT: [[&str; 2]; 6] = [HEADER, ONE, TWO, THREE, FOUR, FIVE];

fn layout() -> SheetLayout<'static> {
    SheetLayout {
        name: "Review",
        widths: &[18, 20],
        chrome: SheetChrome::Filtered,
    }
}

fn cells(row: &[&str; 2]) -> Vec<Cell> {
    row.iter().map(|value| Cell::text(*value)).collect()
}

#[test]
fn exact_boundaries_preserve_every_row_and_header_without_empty_tail() -> TestResult {
    for (count, names, pages) in [
        (0, vec!["Review"], vec![vec![HEADER]]),
        (2, vec!["Review"], vec![vec![HEADER, ONE, TWO]]),
        (
            3,
            vec!["Review", "Review_002"],
            vec![vec![HEADER, ONE, TWO], vec![HEADER, THREE]],
        ),
        (
            4,
            vec!["Review", "Review_002"],
            vec![vec![HEADER, ONE, TWO], vec![HEADER, THREE, FOUR]],
        ),
        (
            5,
            vec!["Review", "Review_002", "Review_003"],
            vec![
                vec![HEADER, ONE, TWO],
                vec![HEADER, THREE, FOUR],
                vec![HEADER, FIVE],
            ],
        ),
    ] {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("book.xlsx");
        let mut output = rust_xlsxwriter::Workbook::new();
        write_paged_with_limit(
            &mut output,
            &path,
            layout(),
            INPUT.iter().take(count + 1).map(|row| Ok(cells(row))),
            2,
        )?;
        output.save(&path)?;
        let mut input: Xlsx<_> = open_workbook(&path)?;
        check!(eq; input.sheet_names(), names);
        for (name, expected) in names.iter().zip(pages) {
            let range = input.worksheet_range(name)?;
            let actual: Vec<Vec<String>> = range
                .rows()
                .map(|row| row.iter().map(ToString::to_string).collect())
                .collect();
            let expected: Vec<Vec<String>> = expected
                .into_iter()
                .map(|row| row.into_iter().map(str::to_owned).collect())
                .collect();
            check!(eq; actual, expected);
        }
    }
    Ok(())
}

#[test]
fn empty_input_preserves_one_empty_sheet() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("empty.xlsx");
    let mut output = rust_xlsxwriter::Workbook::new();
    write_paged_with_limit(
        &mut output,
        &path,
        layout(),
        std::iter::empty::<ReportResult<Vec<Cell>>>(),
        2,
    )?;
    output.save(&path)?;
    let mut input: Xlsx<_> = open_workbook(&path)?;
    check!(eq; input.sheet_names(), ["Review"]);
    check!(input.worksheet_range("Review")?.is_empty());
    Ok(())
}

#[test]
fn later_page_preserves_the_original_input_failure() -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("refused.xlsx");
    let mut output = rust_xlsxwriter::Workbook::new();
    let rows = INPUT
        .iter()
        .take(4)
        .map(|row| Ok(cells(row)))
        .chain(std::iter::once(Err(ReportError::Invariant {
            detail: "controlled row failure".into(),
        })));
    let error = write_paged_with_limit(&mut output, &path, layout(), rows, 2)
        .err()
        .ok_or("later input failure was swallowed")?;
    check!(
        matches!(error, ReportError::Invariant { detail } if detail == "controlled row failure")
    );
    Ok(())
}
