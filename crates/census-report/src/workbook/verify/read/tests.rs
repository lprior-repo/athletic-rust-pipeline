mod archive;
mod fixture;

pub(in crate::workbook::verify) use archive::{alter_sheet, copy_bundle, recapture, rejected_by_all, remove_row, replace_cell};
pub(in crate::workbook::verify) use fixture::publication;
pub(in crate::workbook::verify) type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn declared_dimensions_cannot_hide_trailing_pr_or_metadata_rows() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    for sheet in ["PRs", "Schools", "Meets", "Sources", "Coverage", "Conflicts", "Review", "Run Metrics"] {
        let path = copy_bundle(&good, &directory.path().join(sheet))?;
        recapture(&path, &dataset, &options)?;
        crate::workbook::publication::verify_published(&path)?;
        alter_sheet(&path, sheet, |range| {
            let (row, _) = range.end().ok_or("missing last row")?;
            check!(row > 0, "{sheet} needs a represented data row, not only a header");
            remove_row(range, row)
        })?;
        recapture(&path, &dataset, &options)?;
        rejected_by_all(&path, &dataset, &options, sheet)?;
    }
    Ok(())
}

#[test]
fn absent_headers_short_dimensions_and_interior_gaps_are_rejected() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (good, dataset, options) = publication(directory.path())?;
    for mode in ["headers", "empty", "short", "interior"] {
        let path = copy_bundle(&good, &directory.path().join(mode))?;
        alter_sheet(&path, "Run Metrics", |range| {
            match mode {
                "headers" => remove_row(range, 0)?,
                "interior" => remove_row(range, 1)?,
                "empty" => {
                    let (last_row, _) = range.end().ok_or("metadata rows")?;
                    for row in 0..=last_row {
                        remove_row(range, row)?;
                    }
                }
                _ => *range = calamine::Range::empty(),
            }
            Ok(())
        })?;
        recapture(&path, &dataset, &options)?;
        rejected_by_all(&path, &dataset, &options, "Run Metrics")?;
    }
    Ok(())
}
