use super::cells::{Cell, SheetChrome, SheetLayout};
use crate::report::{
    io_error, Derivation, EventAdmission, RelayProjection, RelaySelection, ReportError,
    ReportResult,
};
use rust_xlsxwriter::Workbook;
use std::io::{BufWriter, Write};
use std::path::Path;

pub(super) const TITLE: &str = "Relays";
pub(super) const HEADERS: [&str; 12] = [
    "Relay ID",
    "Team ID",
    "Event ID",
    "Event",
    "Meet ID",
    "Published Mark",
    "Place",
    "Round",
    "Timing",
    "Published Members",
    "Source Key",
    "Source Team",
];
const WIDTHS: [u16; 12] = [22, 22, 22, 20, 22, 18, 8, 14, 12, 70, 35, 30];

pub(super) fn write_sheet(
    book: &mut Workbook,
    path: &Path,
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
) -> ReportResult<()> {
    let projection = RelayProjection::of(derivation, admission)?;
    let header = Ok(HEADERS.into_iter().map(Cell::text).collect());
    let rows = std::iter::once(header).chain(
        projection
            .selections
            .iter()
            .map(|row| Ok(values(row)?.into_iter().map(Cell::text).collect())),
    );
    super::cells::write_sheet_rows(
        book,
        path,
        SheetLayout {
            name: TITLE,
            widths: &WIDTHS,
            chrome: SheetChrome::Filtered,
        },
        rows,
    )
}

pub(super) fn values(row: &RelaySelection<'_>) -> ReportResult<[String; 12]> {
    let result = row.result;
    Ok([
        result.id.to_string(),
        result.team.to_string(),
        result.event.to_string(),
        row.event_kind.stable_key().into_owned(),
        result.meet.to_string(),
        crate::bests::mark_text(&result.mark),
        result
            .place
            .map_or_else(String::new, |value| value.to_string()),
        result.round.clone().map_or_else(String::new, |value| value),
        result
            .timing
            .map_or_else(String::new, |value| value.stable_key().to_owned()),
        encode(&result.members)?,
        result.source_key.clone(),
        encode(&result.source_team)?,
    ])
}

pub(super) fn write_sidecars(
    directory: &Path,
    derivation: &Derivation<'_>,
    admission: &EventAdmission<'_>,
) -> ReportResult<()> {
    let projection = RelayProjection::of(derivation, admission)?;
    super::publication::write_json(
        &directory.join("relay-dispositions.json"),
        &projection.admissions,
    )?;
    let path = directory.join("relays.jsonl");
    let file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|source| io_error(&path, source))?;
    let mut writer = BufWriter::new(file);
    for row in &projection.selections {
        serde_json::to_writer(&mut writer, row).map_err(|source| ReportError::Decode {
            path: path.clone(),
            line: 0,
            source,
        })?;
        writer
            .write_all(b"\n")
            .map_err(|source| io_error(&path, source))?;
    }
    writer.flush().map_err(|source| io_error(&path, source))?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|source| io_error(&path, source))
}

fn encode(value: &impl serde::Serialize) -> ReportResult<String> {
    census_domain::model::serialized_bytes(value)
        .map_err(|source| ReportError::Invariant {
            detail: format!("encoding published relay: {source}"),
        })
        .and_then(|bytes| {
            String::from_utf8(bytes).map_err(|source| ReportError::Invariant {
                detail: format!("published relay JSON is not UTF-8: {source}"),
            })
        })
}
