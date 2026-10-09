use super::oracle::{Expected, Oracle};
use super::records::{self, Record};
use anyhow::{Context, Result};
use calamine::{open_workbook, Reader, Xlsx};
use std::io::{Read, Seek};
use std::path::Path;

pub fn verify(path: &Path, oracle: &Oracle) -> Result<()> {
    let mut workbook: Xlsx<_> = open_workbook(path)?;
    table(
        &mut workbook,
        "Athletes",
        oracle.all(),
        "Name",
        |row, expected| athlete(row, expected, oracle),
    )?;
    table(
        &mut workbook,
        "Performances_001",
        oracle.all(),
        "Athlete",
        |row, expected| performance(row, expected, oracle),
    )?;
    Ok(())
}

fn table<'a, R: Read + Seek>(
    workbook: &mut Xlsx<R>,
    sheet: &str,
    expected: impl Iterator<Item = &'a Expected>,
    identity: &str,
    validate: impl Fn(&Record<'_>, &Expected) -> Result<()>,
) -> Result<()> {
    let range = workbook.worksheet_range(sheet)?;
    let headers: Vec<_> = range
        .rows()
        .next()
        .context("published sheet has no header")?
        .iter()
        .map(ToString::to_string)
        .collect();
    let rows = range
        .rows()
        .skip(1)
        .map(|row| Ok(row.iter().map(ToString::to_string).collect()));
    records::verify(&headers, rows, expected, identity, validate)
}

fn membership(row: &Record<'_>, expected: &Expected, oracle: &Oracle) -> Result<()> {
    row.equal("School", &expected.1)?;
    row.equal("State", oracle.event(expected)?.state.code())?;
    row.equal("Graduation Year", "2027")
}

fn athlete(row: &Record<'_>, expected: &Expected, oracle: &Oracle) -> Result<()> {
    membership(row, expected, oracle)?;
    row.equal("Observed School Year", "2025-26")?;
    row.empty(&[
        "Preferred Contact Email",
        "Preferred Contact Coach ID",
        "Preferred Contact Source URL",
        "Preferred Contact Capture SHA256",
        "Preferred Contact Acquired At",
        "Head TF Coach Email",
        "Head XC Coach Email",
        census_report::export::postal::ATHLETE_ADDRESS_HEADER,
    ])
}

fn performance(row: &Record<'_>, expected: &Expected, oracle: &Oracle) -> Result<()> {
    membership(row, expected, oracle)?;
    let event = oracle.event(expected)?;
    row.equal("Mark", &expected.3)?;
    row.equal("Date", event.date)?;
    row.equal("Meet", event.name)?;
    row.equal(
        "Place",
        &expected.5.map_or(String::new(), |place| place.to_string()),
    )?;
    row.equal(
        "Source URL",
        &census_crawl::athleticlive::event_doc_url(event.event),
    )
}
