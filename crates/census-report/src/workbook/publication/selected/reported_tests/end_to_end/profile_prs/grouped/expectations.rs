use super::super::TestResult;
use calamine::{open_workbook, DataType, Reader, Xlsx};
use census_crawl::participation_evidence::provider_target::{
    profile_pr::{self, SourceProfilePrV1},
    SourceTargetPacketV1,
};
use census_domain::model::SourceIdentity;
use census_store::RetainedCaptureRootV1;
use std::collections::BTreeSet;
use std::path::Path;

pub(in super::super) struct Capture {
    pub(in super::super) root: RetainedCaptureRootV1,
    pub(super) header: String,
    pub(super) rows: Vec<String>,
    detailed: Vec<String>,
    records: Vec<Vec<String>>,
}

pub(in super::super) fn expectation(
    packet: &SourceTargetPacketV1,
    root: RetainedCaptureRootV1,
    athlete: &str,
) -> TestResult<Capture> {
    let checked = packet.read_profile_prs()?;
    check!(eq; checked.rows().len(), 46);
    let capture = packet.profile_capture();
    let header = format!(
        "Source-declared PR capture (unverified); Profile: {}; Capture: {}; Root: {}; Body interpretation: {}",
        capture.requested_url(), capture.acquired_at().to_rfc3339(), root.key,
        profile_pr::INTERPRETATION,
    );
    let mut result = Capture {
        root,
        header,
        rows: Vec::new(),
        detailed: Vec::new(),
        records: Vec::new(),
    };
    for row in checked.rows() {
        let (compact, detailed) = literals(packet, &result.root, row);
        result.rows.push(compact);
        result.detailed.push(detailed);
        result.records.push(record(
            packet,
            &result.root,
            checked.athlete(),
            row,
            athlete,
        )?);
    }
    Ok(result)
}

fn literals(
    packet: &SourceTargetPacketV1,
    root: &RetainedCaptureRootV1,
    row: &SourceProfilePrV1,
) -> (String, String) {
    let location = row.locator();
    let capture = packet.profile_capture();
    let compact =
        format!(
        "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Locator: {}:{}:{}",
        row.event(), row.published_mark(), row.source_mode(), row.source_level(),
        row.published_date(), location.ordinal(), location.byte_offset(), location.byte_length(),
    );
    let detailed = format!(
        "{}: {}; Source-declared PR; Mode: {}; Level: {}; Published date: {}; Profile: {}; Capture: {}; Root: {}; Body interpretation: {}; Locator: {}:{}:{}",
        row.event(), row.published_mark(), row.source_mode(), row.source_level(),
        row.published_date(), capture.requested_url(), capture.acquired_at().to_rfc3339(),
        root.key, profile_pr::INTERPRETATION,
        location.ordinal(), location.byte_offset(), location.byte_length(),
    );
    (compact, detailed)
}

fn record(
    packet: &SourceTargetPacketV1,
    root: &RetainedCaptureRootV1,
    source: &SourceIdentity,
    row: &SourceProfilePrV1,
    athlete: &str,
) -> TestResult<Vec<String>> {
    let capture = packet.profile_capture();
    let location = row.locator();
    Ok(vec![
        athlete.to_owned(),
        serde_json::to_string(&source.namespace)?,
        source.id.clone(),
        row.event().to_owned(),
        row.published_mark().to_owned(),
        row.source_mode().to_owned(),
        row.source_level().to_owned(),
        row.published_date().to_owned(),
        capture.requested_url().to_owned(),
        capture.acquired_at().to_rfc3339(),
        capture.body_sha256().to_owned(),
        serde_json::to_string(root)?,
        root.payload.digest.as_str().to_owned(),
        root.payload.bytes.to_string(),
        profile_pr::INTERPRETATION.to_owned(),
        location.ordinal().to_string(),
        location.byte_offset().to_string(),
        location.byte_length().to_string(),
        "source_reported".to_owned(),
        "source_declared_pr_unverified".to_owned(),
    ])
}

pub(in super::super) fn texts(captures: &[Capture]) -> (String, String) {
    let detailed = captures
        .iter()
        .flat_map(|capture| capture.detailed.iter().map(String::as_str))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join("\n");
    let grouped = captures
        .iter()
        .flat_map(|capture| {
            std::iter::once(capture.header.as_str()).chain(capture.rows.iter().map(String::as_str))
        })
        .collect::<Vec<_>>()
        .join("\n");
    (detailed, grouped)
}

pub(in super::super) fn cell(workbook: &Path, athlete: &str) -> TestResult<String> {
    let mut book: Xlsx<_> = open_workbook(workbook)?;
    let sheet = book.worksheet_range("Athletes")?;
    let headers = sheet.rows().next().ok_or("worksheet header")?;
    let id = headers
        .iter()
        .position(|cell| cell.get_string().is_some_and(|text| text == "Athlete ID"))
        .ok_or("athlete ID")?;
    let prs = headers
        .iter()
        .position(|cell| {
            cell.get_string()
                .is_some_and(|text| text == "Source-declared PRs")
        })
        .ok_or("source PR column")?;
    let row = sheet
        .rows()
        .skip(1)
        .find(|row| {
            row.get(id)
                .is_some_and(|cell| cell.get_string().is_some_and(|text| text == athlete))
        })
        .ok_or("profile athlete row")?;
    Ok(row.get(prs).ok_or("source PR cell")?.to_string())
}

pub(in super::super) fn csv_records(generation: &Path, expected: &[Capture]) -> TestResult {
    let mut csv = csv::Reader::from_path(generation.join("source_profile_prs.csv"))?;
    check!(eq; csv.headers()?.iter().collect::<Vec<_>>(), vec![
        "canonical_athlete_id", "source_namespace", "source_athlete_id", "event",
        "source_declared_pr", "source_mode", "source_level", "published_date",
        "source_profile_url", "capture_acquired_at", "capture_body_sha256", "retained_root",
        "retained_payload_sha256", "retained_payload_bytes", "interpretation", "row_ordinal",
        "byte_offset", "byte_length", "record_basis", "verification_status",
    ]);
    let actual = csv
        .records()
        .map(|record| Ok(record?.iter().map(str::to_owned).collect::<Vec<_>>()))
        .collect::<TestResult<Vec<_>>>()?;
    let records = expected
        .iter()
        .flat_map(|capture| capture.records.iter().cloned())
        .collect::<Vec<_>>();
    check!(eq; actual, records);
    for capture in expected {
        check!(eq; capture.records.len(), 46);
    }
    Ok(())
}
