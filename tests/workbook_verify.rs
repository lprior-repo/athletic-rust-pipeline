use athletic_rust_pipeline::{domain::identity::WorkbookDigest, workbook_verify::verify_fields};
use rust_xlsxwriter::Workbook;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, result::Result as StdResult};
use tempfile::tempdir;

type TestResult = StdResult<(), Box<dyn std::error::Error>>;

#[derive(Clone)]
enum Cell {
    Text(&'static str),
    Number(f64),
    Bool(bool),
}

type Rows = Vec<(u32, Vec<Cell>)>;
type Sheet = (&'static str, Rows);

fn write_fixture(path: &Path, sheets: &[Sheet]) -> TestResult {
    let mut workbook = Workbook::new();
    sheets.iter().try_for_each(|(name, rows)| -> TestResult {
        let worksheet = workbook.add_worksheet().set_name(*name)?;
        rows.iter().try_for_each(|(row, cells)| {
            cells.iter().enumerate().try_for_each(|(column, cell)| {
                let column = u16::try_from(column)?;
                match cell {
                    Cell::Text(value) => {
                        worksheet.write_string(*row, column, *value).map(|_| ())?
                    }
                    Cell::Number(value) => {
                        worksheet.write_number(*row, column, *value).map(|_| ())?
                    }
                    Cell::Bool(value) => {
                        worksheet.write_boolean(*row, column, *value).map(|_| ())?
                    }
                }
                Ok::<(), Box<dyn std::error::Error>>(())
            })
        })
    })?;
    workbook.save(path)?;
    Ok(())
}

fn expected_digest(path: &Path) -> StdResult<WorkbookDigest, Box<dyn std::error::Error>> {
    let bytes = fs::read(path)?;
    Ok(WorkbookDigest::parse(&format!(
        "{:x}",
        Sha256::digest(bytes)
    ))?)
}

fn source_sheets() -> Vec<Sheet> {
    vec![
        (
            "Alpha",
            vec![
                (
                    0,
                    vec![
                        Cell::Text("Name"),
                        Cell::Text("Score"),
                        Cell::Text("Flag"),
                        Cell::Text("Literal"),
                    ],
                ),
                (
                    1,
                    vec![
                        Cell::Text("Alice"),
                        Cell::Number(42.5),
                        Cell::Bool(true),
                        Cell::Text("=literal"),
                    ],
                ),
                (
                    3,
                    vec![
                        Cell::Text("Bob"),
                        Cell::Number(7.0),
                        Cell::Bool(false),
                        Cell::Text("plain"),
                    ],
                ),
            ],
        ),
        (
            "Beta",
            vec![
                (
                    0,
                    vec![
                        Cell::Text("Name"),
                        Cell::Text("Score"),
                        Cell::Text("Flag"),
                        Cell::Text("Literal"),
                    ],
                ),
                (
                    1,
                    vec![
                        Cell::Text("Cara"),
                        Cell::Number(9.0),
                        Cell::Bool(true),
                        Cell::Text("=keep"),
                    ],
                ),
            ],
        ),
    ]
}

fn output_sheets(rows: Rows, second: Rows) -> Vec<Sheet> {
    vec![("Alpha", rows), ("Beta", second)]
}

fn output_rows() -> (Rows, Rows) {
    (
        vec![
            (
                0,
                vec![
                    Cell::Text("Name"),
                    Cell::Text("Score"),
                    Cell::Text("Flag"),
                    Cell::Text("Literal"),
                    Cell::Text("Status"),
                ],
            ),
            (
                1,
                vec![
                    Cell::Text("Alice"),
                    Cell::Text("42.5"),
                    Cell::Text("1"),
                    Cell::Text("=literal"),
                    Cell::Text("PENDING"),
                ],
            ),
            (
                3,
                vec![
                    Cell::Text("Bob"),
                    Cell::Text("7"),
                    Cell::Text("0"),
                    Cell::Text("plain"),
                    Cell::Text("PENDING"),
                ],
            ),
        ],
        vec![
            (
                0,
                vec![
                    Cell::Text("Name"),
                    Cell::Text("Score"),
                    Cell::Text("Flag"),
                    Cell::Text("Literal"),
                    Cell::Text("Status"),
                ],
            ),
            (
                1,
                vec![
                    Cell::Text("Cara"),
                    Cell::Text("9"),
                    Cell::Text("1"),
                    Cell::Text("=keep"),
                    Cell::Text("PENDING"),
                ],
            ),
        ],
    )
}

#[test]
fn verifies_sparse_rows_and_scalar_cell_semantics() -> TestResult {
    let directory = tempdir()?;
    let original = directory.path().join("original.xlsx");
    let output = directory.path().join("output.xlsx");
    write_fixture(&original, &source_sheets())?;
    let (alpha, beta) = output_rows();
    write_fixture(&output, &output_sheets(alpha, beta))?;
    let report = verify_fields(
        &original,
        &output,
        &expected_digest(&original)?,
        &["Status".to_owned()],
    )?;
    assert_eq!(report.source_sheet_count, 2);
    assert_eq!(report.matched_row_count, 3);
    assert_eq!(report.matched_field_count, 12);
    assert_eq!(report.appended_header_count, 1);
    assert!(report.source_hash_before_matches);
    assert!(report.source_hash_after_matches);
    Ok(())
}

#[test]
fn rejects_missing_duplicated_or_moved_rows() -> TestResult {
    let directory = tempdir()?;
    let original = directory.path().join("original.xlsx");
    write_fixture(&original, &source_sheets())?;
    let (mut alpha, beta) = output_rows();
    alpha.retain(|(row, _)| *row != 3);
    let missing = directory.path().join("missing.xlsx");
    write_fixture(&missing, &output_sheets(alpha, beta.clone()))?;
    assert!(verify_fields(
        &original,
        &missing,
        &expected_digest(&original)?,
        &["Status".to_owned()]
    )
    .is_err());

    let (mut alpha, _) = output_rows();
    alpha.push((4, alpha[2].1.clone()));
    let duplicated = directory.path().join("duplicated.xlsx");
    write_fixture(&duplicated, &output_sheets(alpha, beta.clone()))?;
    assert!(verify_fields(
        &original,
        &duplicated,
        &expected_digest(&original)?,
        &["Status".to_owned()]
    )
    .is_err());

    let (mut alpha, _) = output_rows();
    alpha[2].0 = 2;
    let moved = directory.path().join("moved.xlsx");
    write_fixture(&moved, &output_sheets(alpha, beta))?;
    assert!(verify_fields(
        &original,
        &moved,
        &expected_digest(&original)?,
        &["Status".to_owned()]
    )
    .is_err());
    Ok(())
}

#[test]
fn rejects_altered_cells_and_extra_header_collisions() -> TestResult {
    let directory = tempdir()?;
    let original = directory.path().join("original.xlsx");
    write_fixture(&original, &source_sheets())?;
    let (mut alpha, beta) = output_rows();
    alpha[1].1[1] = Cell::Text("43.5");
    let altered = directory.path().join("altered.xlsx");
    write_fixture(&altered, &output_sheets(alpha, beta.clone()))?;
    assert!(verify_fields(
        &original,
        &altered,
        &expected_digest(&original)?,
        &["Status".to_owned()]
    )
    .is_err());
    assert!(verify_fields(
        &original,
        &altered,
        &expected_digest(&original)?,
        &["Score".to_owned()]
    )
    .is_err());
    Ok(())
}

#[test]
fn rejects_malformed_output_workbook() -> TestResult {
    let directory = tempdir()?;
    let original = directory.path().join("original.xlsx");
    let malformed = directory.path().join("malformed.xlsx");
    write_fixture(&original, &source_sheets())?;
    write_fixture(
        &malformed,
        &[("Alpha", vec![(0, vec![Cell::Text("Name")])])],
    )?;
    assert!(verify_fields(
        &original,
        &malformed,
        &expected_digest(&original)?,
        &["Status".to_owned()]
    )
    .is_err());
    Ok(())
}
