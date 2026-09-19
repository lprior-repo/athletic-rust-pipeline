use athletic_rust_pipeline::{domain::identity::WorkbookDigest, workbook_verify::verify_fields};
use rust_xlsxwriter::Workbook;
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, result::Result as StdResult};
use tempfile::tempdir;
use zip::{write::SimpleFileOptions, ZipWriter};

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
fn write_minimal_shared_workbook(path: &Path, dimension: &str, expanded: bool) -> TestResult {
    let options = SimpleFileOptions::default();
    let headers = [
        "Name",
        "Score",
        "Flag",
        "Literal",
        "Status",
        "Audit One",
        "Audit Two",
        "Audit Three",
    ];
    let status = if expanded {
        "x".repeat(2 * 1024 * 1024)
    } else {
        "PENDING".to_owned()
    };
    let shared_strings = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<sst xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\" count=\"14\" uniqueCount=\"14\">\
<si><t>Name</t></si><si><t>Score</t></si><si><t>Flag</t></si><si><t>Literal</t></si><si><t>Status</t></si>\
<si><t>Audit One</t></si><si><t>Audit Two</t></si><si><t>Audit Three</t></si>\
<si><t>Alice</t></si><si><t>42.5</t></si><si><t>1</t></si><si><t>=literal</t></si>\
<si><t>PENDING</t></si><si><t>{status}</t></si></sst>"
    );
    let header_cells = (b'A'..=b'H')
        .zip(0..headers.len())
        .map(|(column, index)| {
            let column = char::from(column);
            format!("<c r=\"{column}1\" t=\"s\"><v>{index}</v></c>")
        })
        .collect::<String>();
    let data_indexes = [
        8,
        9,
        10,
        11,
        if expanded { 13 } else { 12 },
        if expanded { 13 } else { 12 },
        if expanded { 13 } else { 12 },
        if expanded { 13 } else { 12 },
    ];
    let data_cells = (b'A'..=b'H')
        .zip(data_indexes)
        .map(|(column, shared_index)| {
            let column = char::from(column);
            format!("<c r=\"{column}2\" t=\"s\"><v>{shared_index}</v></c>")
        })
        .collect::<String>();
    let worksheet = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\
<worksheet xmlns=\"http://schemas.openxmlformats.org/spreadsheetml/2006/main\">\
<dimension ref=\"{dimension}\"/><sheetData><row r=\"1\">{header_cells}</row>\
<row r=\"2\">{data_cells}</row></sheetData></worksheet>"
    );
    let mut archive = ZipWriter::new(fs::File::create(path)?);
    archive.start_file("[Content_Types].xml", options)?;
    archive.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">
<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>
<Default Extension="xml" ContentType="application/xml"/>
<Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/>
<Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/>
<Override PartName="/xl/sharedStrings.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sharedStrings+xml"/>
</Types>"#,
    )?;
    archive.start_file("_rels/.rels", options)?;
    archive.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/>
</Relationships>"#,
    )?;
    archive.start_file("xl/workbook.xml", options)?;
    archive.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships">
<sheets><sheet name="Alpha" sheetId="1" r:id="rId1"/></sheets>
</workbook>"#,
    )?;
    archive.start_file("xl/_rels/workbook.xml.rels", options)?;
    archive.write_all(
        br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">
<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/>
<Relationship Id="rId2" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/sharedStrings" Target="sharedStrings.xml"/>
</Relationships>"#,
    )?;
    archive.start_file("xl/worksheets/sheet1.xml", options)?;
    archive.write_all(worksheet.as_bytes())?;
    archive.start_file("xl/sharedStrings.xml", options)?;
    archive.write_all(shared_strings.as_bytes())?;
    archive.finish()?;
    Ok(())
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

#[test]
fn rejects_shared_string_row_amplification_and_bad_dimensions() -> TestResult {
    let directory = tempdir()?;
    let original = directory.path().join("original.xlsx");
    let mut source = source_sheets().into_iter().take(1).collect::<Vec<_>>();
    source[0].1.truncate(2);
    write_fixture(&original, &source)?;

    let extra_headers = vec![
        "Status".to_owned(),
        "Audit One".to_owned(),
        "Audit Two".to_owned(),
        "Audit Three".to_owned(),
    ];
    let bounded = directory.path().join("bounded.xlsx");
    write_minimal_shared_workbook(&bounded, "A1:H2", false)?;
    let report = verify_fields(
        &original,
        &bounded,
        &expected_digest(&original)?,
        &extra_headers,
    )?;
    assert_eq!(report.matched_row_count, 1);

    let expanded = directory.path().join("expanded.xlsx");
    write_minimal_shared_workbook(&expanded, "A1:H2", true)?;

    assert!(verify_fields(
        &original,
        &expanded,
        &expected_digest(&original)?,
        &extra_headers
    )
    .is_err());

    let malformed = directory.path().join("malformed-dimension.xlsx");
    write_minimal_shared_workbook(&malformed, "A1:B", false)?;
    assert!(verify_fields(
        &original,
        &malformed,
        &expected_digest(&original)?,
        &extra_headers
    )
    .is_err());

    let out_of_bounds = directory.path().join("out-of-bounds-dimension.xlsx");
    write_minimal_shared_workbook(&out_of_bounds, "XFE1:XFE2", false)?;
    assert!(verify_fields(
        &original,
        &out_of_bounds,
        &expected_digest(&original)?,
        &extra_headers
    )
    .is_err());
    Ok(())
}
