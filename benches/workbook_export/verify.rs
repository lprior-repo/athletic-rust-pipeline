//! The dataset gate: the assertions that refuse to hand the benches a corpus they have not round-tripped.
use super::*;

/// One complete export pass, used by the dataset gate.
pub(super) fn export_once(
    manifest: &SourceManifest,
    rows: &[ExportRow],
    extra_headers: &[String],
    destination: &Path,
) -> Result<ExportStats> {
    let mut export = WorkbookExport::new(manifest, extra_headers)?;
    for row in rows {
        export.write(row.clone())?;
    }
    export.finish(destination)
}

/// The export must report exactly the manifest's rows, sheets, and source digest.
pub(super) fn verify_stats(stats: &ExportStats, manifest: &SourceManifest, rows: usize) -> Result<()> {
    let rows = u64::try_from(rows).context("row count conversion overflow")?;
    ensure!(
        stats.aggregate_rows == rows,
        "export reported {} rows, expected {rows}",
        stats.aggregate_rows
    );
    ensure!(
        stats.sheets
            == [ExportSheetStats {
                name: SHEET.to_owned(),
                rows,
            }],
        "export reported sheets {:?}",
        stats.sheets
    );
    ensure!(
        stats.source_sha256 == manifest.workbook.as_str(),
        "export reported source digest {}",
        stats.source_sha256
    );
    Ok(())
}

/// The published XLSX must re-read to the source headers followed by the extra headers, with the
/// header row plus exactly `ROWS` data rows.
pub(super) fn verify_export_workbook(path: &Path, extra_columns: usize) -> Result<()> {
    let mut workbook: Xlsx<_> = open_workbook(path).context("opening the published export")?;
    let names = workbook.sheet_names().to_owned();
    ensure!(names == [SHEET.to_owned()], "export sheets are {names:?}");
    let range = workbook
        .worksheet_range(SHEET)
        .context("reading the published export")?;
    let expected = expected_headers(extra_columns);
    ensure!(
        header_row(&range)? == expected,
        "published export headers differ from the expected source plus extra headers"
    );
    ensure!(
        range.width() == expected.len(),
        "published export is {} columns wide, expected {}",
        range.width(),
        expected.len()
    );
    ensure!(
        range.height() == ROWS + 1,
        "published export has {} rows, expected {}",
        range.height(),
        ROWS + 1
    );
    Ok(())
}

/// The synthetic source workbook must carry the headers and row count the manifest declares.
pub(super) fn verify_source_workbook(path: &Path) -> Result<()> {
    let mut workbook: Xlsx<_> = open_workbook(path).context("opening the source workbook")?;
    let names = workbook.sheet_names().to_owned();
    ensure!(names == [SHEET.to_owned()], "source sheets are {names:?}");
    let range = workbook
        .worksheet_range(SHEET)
        .context("reading the source workbook")?;
    ensure!(
        header_row(&range)? == expected_headers(0),
        "source headers differ from SOURCE_HEADERS"
    );
    ensure!(
        range.height() == ROWS + 1,
        "source workbook has {} rows, expected {}",
        range.height(),
        ROWS + 1
    );
    Ok(())
}

/// The header row of a worksheet range, as text.
pub(super) fn header_row(range: &Range<Data>) -> Result<Vec<String>> {
    let row = range.rows().next().context("worksheet has no header row")?;
    row.iter()
        .map(|cell| match cell {
            Data::String(text) => Ok(text.clone()),
            other => Err(anyhow::anyhow!("header cell {other:?} is not text")),
        })
        .collect()
}

/// `SOURCE_HEADERS` followed by `count` extra headers, as one worksheet's header row.
pub(super) fn expected_headers(count: usize) -> Vec<String> {
    SOURCE_HEADERS
        .iter()
        .map(|header| (*header).to_owned())
        .chain(
            EXPORT_HEADERS
                .iter()
                .take(count)
                .map(|header| (*header).to_owned()),
        )
        .collect()
}
