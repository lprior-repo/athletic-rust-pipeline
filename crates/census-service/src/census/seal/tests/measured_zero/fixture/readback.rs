use super::TestResult;
use calamine::{open_workbook, Data, Reader, Xlsx};
use std::path::Path;

pub(super) fn assert_readback(path: &Path) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    for name in ["Athletes", "PRs"] {
        let range = book.worksheet_range(name)?;
        check!(eq; range.height(), 1, "zero-result sheet {name} contains only its header");
        check!(range.width() > 0);
    }
    let schools = book.worksheet_range("Schools")?;
    check!(eq; schools.height(), 2);
    let headers = schools.rows().next().ok_or("schools header")?;
    let column = headers
        .iter()
        .position(|cell| cell == "School")
        .ok_or("school column")?;
    check!(eq; schools.get((1, column)), Some(&Data::String("Measured Zero High".into())));
    let generation = path.parent().ok_or("generation directory")?;
    let mut reader = csv::Reader::from_path(generation.join("recruiting.csv"))?;
    check!(reader
        .headers()?
        .iter()
        .any(|header| header == "athlete_id"));
    check!(reader.records().next().transpose()?.is_none());
    Ok(())
}
