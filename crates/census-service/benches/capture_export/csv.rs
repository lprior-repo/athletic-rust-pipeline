use super::oracle::{Expected, Oracle};
use super::records::{self, Record};
use anyhow::Result;
use std::path::Path;

pub fn verify(directory: &Path, oracle: &Oracle) -> Result<()> {
    table(
        &directory.join("recruiting.csv"),
        oracle.all(),
        |row, expected| recruiting(row, expected, oracle),
    )
}

fn table<'a>(
    path: &Path,
    expected: impl Iterator<Item = &'a Expected>,
    validate: impl Fn(&Record<'_>, &Expected) -> Result<()>,
) -> Result<()> {
    let mut reader = ::csv::Reader::from_path(path)?;
    let headers: Vec<_> = reader.headers()?.iter().map(str::to_string).collect();
    let rows = reader
        .records()
        .map(|row| Ok(row?.iter().map(str::to_string).collect()));
    records::verify(&headers, rows, expected, "name", validate)
}

fn membership(row: &Record<'_>, expected: &Expected, oracle: &Oracle) -> Result<()> {
    row.equal("school", &expected.1)?;
    row.equal("state", oracle.event(expected)?.state.code())?;
    row.equal("grad_year", "2027")
}

fn recruiting(row: &Record<'_>, expected: &Expected, oracle: &Oracle) -> Result<()> {
    membership(row, expected, oracle)?;
    row.empty(&[
        "head_track_coach_email",
        "head_xc_coach_email",
        "athletic_director_email",
        "coach_id",
        "coach_source_url",
        "coach_capture_sha256",
        "coach_acquired_at",
        census_report::export::postal::ATHLETE_ADDRESS_CSV_HEADER,
    ])
}
