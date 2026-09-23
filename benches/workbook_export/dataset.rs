//! The synthetic corpus: source workbook, manifest and rows, built once per run.
use super::*;
use super::verify::{export_once, verify_stats, verify_export_workbook, verify_source_workbook};

/// The synthetic corpus, its verified source workbook, and the manifest the export is built from.
pub(super) struct Dataset {
    pub(super) manifest: SourceManifest,
    pub(super) rows: Vec<ExportRow>,
    pub(super) extra_headers: Vec<String>,
    pub(super) directory: TempDir,
}

impl Dataset {
    pub(super) fn build() -> Result<Self> {
        let directory = tempfile::tempdir().context("creating the bench directory")?;
        let source = directory.path().join("source.xlsx");
        write_source_workbook(&source)?;
        let digest = digest_of(&source)?;
        let extra_headers = EXPORT_HEADERS
            .iter()
            .map(|header| (*header).to_owned())
            .collect::<Vec<_>>();
        let manifest = manifest_for(directory.path(), source, digest)?;
        let rows = build_rows()?;

        // The source workbook on disk must carry the shape the manifest declares.
        verify_source_workbook(&manifest.original)?;
        // One full export must round-trip to the same cardinality before anything is measured.
        let destination = directory.path().join("verified-export.xlsx");
        let stats = export_once(&manifest, &rows, &extra_headers, &destination)?;
        verify_stats(&stats, &manifest, rows.len())?;
        verify_export_workbook(&destination, extra_headers.len())?;

        let source_bytes = fs::metadata(&manifest.original)
            .context("reading the source workbook size")?
            .len();
        let export_bytes = fs::metadata(&destination)
            .context("reading the verified export size")?
            .len();
        println!("metric=bench_dataset_rows value={ROWS} unit=rows");
        println!("metric=bench_source_bytes value={source_bytes} unit=bytes");
        println!("metric=bench_export_bytes value={export_bytes} unit=bytes");
        println!(
            "metric=bench_columns value={} unit=columns",
            SOURCE_HEADERS.len() + extra_headers.len()
        );
        println!(
            "bench dataset verified: {ROWS} rows exported and re-read over one worksheet, \
             {} header columns",
            SOURCE_HEADERS.len() + extra_headers.len()
        );
        Ok(Self {
            manifest,
            rows,
            extra_headers,
            directory,
        })
    }
}

/// A fresh export holding every manifest row: the state `finish` starts from.
pub(super) fn written_export(dataset: &Dataset) -> WorkbookExport {
    let mut export = checked(WorkbookExport::new(
        &dataset.manifest,
        &dataset.extra_headers,
    ));
    for row in &dataset.rows {
        checked(export.write(row.clone()));
    }
    export
}

/// Write the synthetic source workbook: one header row and `ROWS` deterministic data rows.
pub(super) fn write_source_workbook(path: &Path) -> Result<()> {
    let mut workbook = Workbook::new();
    let worksheet = workbook
        .add_worksheet()
        .set_name(SHEET)
        .context("naming the source worksheet")?;
    SOURCE_HEADERS
        .iter()
        .enumerate()
        .try_for_each(|(column, header)| {
            let column = u16::try_from(column).context("header column conversion overflow")?;
            worksheet
                .write_string(0, column, *header)
                .map(|_| ())
                .context("writing a source header")
        })?;
    for row in 0..ROWS {
        let worksheet_row = u32::try_from(row).context("row conversion overflow")?;
        SOURCE_HEADERS
            .iter()
            .enumerate()
            .try_for_each(|(column, header)| {
                let column = u16::try_from(column).context("data column conversion overflow")?;
                worksheet
                    .write_string(worksheet_row + 1, column, cell_value(row, header))
                    .map(|_| ())
                    .context("writing a source value")
            })?;
    }
    workbook
        .save(path)
        .context("saving the synthetic source workbook")
}

/// The manifest the export is built from, matching the workbook just written.
pub(super) fn manifest_for(
    directory: &Path,
    source: PathBuf,
    workbook: WorkbookDigest,
) -> Result<SourceManifest> {
    let last_row = u32::try_from(ROWS)
        .context("row conversion overflow")?
        .checked_add(FIRST_DATA_ROW - 1)
        .context("last row overflow")?;
    let rows = u64::try_from(ROWS).context("row count conversion overflow")?;
    Ok(SourceManifest {
        original: source,
        frozen: directory.join("frozen.xlsx"),
        workbook,
        ingestion_revision: INGESTION_REVISION.to_owned(),
        stats: WorkbookStats {
            sheets: vec![SheetStats {
                name: SHEET.to_owned(),
                declared_dimension: Some(format!(
                    "A1:{}{last_row}",
                    column_letters(SOURCE_HEADERS.len())?
                )),
                xml_rows: u64::from(last_row),
                actual_data_rows: rows,
                last_actual_row: last_row,
                headers: SOURCE_HEADERS
                    .iter()
                    .map(|header| (*header).to_owned())
                    .collect(),
            }],
            actual_data_rows: rows,
            selected_prospects: rows,
        },
    })
}

/// The synthetic rows, in manifest order: Excel row `index + FIRST_DATA_ROW`.
pub(super) fn build_rows() -> Result<Vec<ExportRow>> {
    (0..ROWS)
        .map(|row| {
            let worksheet_row = u32::try_from(row).context("row conversion overflow")?;
            let excel_row = worksheet_row
                .checked_add(FIRST_DATA_ROW)
                .context("row overflow")?;
            let fields = SOURCE_HEADERS
                .iter()
                .map(|header| ((*header).to_owned(), cell_value(row, header)))
                .collect::<BTreeMap<_, _>>();
            let extra_fields = EXPORT_HEADERS
                .iter()
                .take(FILLED_EXTRA_COLUMNS)
                .enumerate()
                .map(|(slot, header)| ((*header).to_owned(), format!("bench-{slot}-{row}")))
                .collect::<BTreeMap<_, _>>();
            Ok(ExportRow {
                source: SourceRecord {
                    source_key: format!("{SHEET}:{excel_row}"),
                    sheet: SHEET.to_owned(),
                    excel_row,
                    fields,
                },
                extra_fields,
            })
        })
        .collect()
}

/// A deterministic value for one header of one row: the same corpus on every machine and run.
pub(super) fn cell_value(row: usize, header: &str) -> String {
    match header {
        "Person First" => format!("Bench{row}"),
        "Person Last" => "Runner".to_owned(),
        "Person Email" => format!("bench{row}@example.invalid"),
        "Address Mailing / Permanent City" => "Madison".to_owned(),
        "Address Mailing / Permanent Region" => "WI".to_owned(),
        "Sports Sport" => "Track & Field".to_owned(),
        "Origin Source" => "bench-harness".to_owned(),
        "Schools Name" => format!("Bench High School {}", row % 32),
        _ => format!("bench-{row}"),
    }
}

/// SHA-256 of a file, as the workbook digest the manifest carries.
pub(super) fn digest_of(path: &Path) -> Result<WorkbookDigest> {
    let bytes = fs::read(path).context("reading the source workbook for its digest")?;
    let digest = Sha256::digest(&bytes);
    WorkbookDigest::parse(&format!("{digest:x}")).context("parsing the source workbook digest")
}

/// Excel column letters for a one-based column count, for the declared dimension only.
pub(super) fn column_letters(count: usize) -> Result<String> {
    let offset = u8::try_from(count)
        .ok()
        .and_then(|count| count.checked_sub(1))
        .filter(|offset| *offset < 26)
        .context("declared dimension needs column letters beyond A..Z")?;
    let letter = b'A'.checked_add(offset).context("column letter overflow")?;
    Ok(char::from(letter).to_string())
}
