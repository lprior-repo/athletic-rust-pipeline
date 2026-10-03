use anyhow::{Context, Result};
use census_crawl::milesplit;

#[path = "milesplit_fixtures/provenance.rs"]
mod provenance;
#[path = "milesplit_fixtures/raw.rs"]
mod raw;
#[path = "milesplit_fixtures/replay.rs"]
mod replay;

pub use replay::replay_owned_captures;

pub const PROVENANCE: &str = "troy_725218_raw_projection_provenance.json";
pub const RELAYS: &str = "troy_725218_team_relays.json";
pub const TROY_URL: &str = "https://al.milesplit.com/meets/725218-troy-invitational-2-2026/results";
pub const RAW_FILES: [&str; 2] = [
    "troy_725218_rs1266814_raw_projection.html",
    "troy_725218_rs1266815_raw_projection.html",
];

pub fn validate_result_fixture(file: &str, body: &str) -> Result<bool> {
    match file {
        PROVENANCE => provenance::validate(body)?,
        RELAYS => validate_relays(body)?,
        "troy_725218_rs1266814_raw_projection.html" => raw::validate_projection(body, false)?,
        "troy_725218_rs1266815_raw_projection.html" => raw::validate_projection(body, true)?,
        "oh_meet_770621_results.html" => validate_listing(body, false)?,
        "dc_meet_735841_results_legacy.html" => {
            validate_listing(body, true)?;
            raw::validate_capture(file, body)?;
        }
        "dc_meet_764735_results_inline.html" => {
            validate_inline(body)?;
            raw::validate_capture(file, body)?;
        }
        "oh_meet_770621_rs1321880_raw.html" | "nc_meet_684812_rs1283641_raw.html" => {
            raw::validate_capture(file, body)?;
        }
        _ => return Ok(false),
    }
    Ok(true)
}

fn validate_listing(body: &str, legacy: bool) -> Result<()> {
    let (url, expected) = if legacy {
        (
            "https://www.milesplit.com/meets/735841-stancs-home-meet-1-2026/results",
            vec![
                (1257095, "Varsity Boys Results", 0, false),
                (1257096, "Varsity Girls Results", 0, false),
            ],
        )
    } else {
        (
            "https://oh.milesplit.com/meets/770621-beaver-eastern-invite-2026/results",
            vec![(1321880, "Results", 0, false)],
        )
    };
    let files = milesplit::parse_meet_result_files(url, body)?;
    let actual: Vec<_> = files
        .iter()
        .map(|row| (row.id, row.name.as_str(), row.is_meet_pro, row.inline))
        .collect();
    check!(eq; actual, expected);
    Ok(())
}

fn validate_inline(body: &str) -> Result<()> {
    let url = "https://www.milesplit.com/meets/764735-dc10-track-fest-hosted-by-light-horse-track-club-2026/results";
    let files = milesplit::parse_meet_result_files(url, body)?;
    check!(eq; files.len(), 1);
    let file = files
        .first()
        .context("inline capture carries no result file")?;
    check!(eq; (file.id, file.inline, file.is_meet_pro), (0, true, 0));
    check!(eq; file.raw_url(url), url);
    Ok(())
}

fn validate_relays(body: &str) -> Result<()> {
    let document: serde_json::Value = serde_json::from_str(body)?;
    let url = format!(
        "https://al.milesplit.com/api/v1/meets/725218/performances?isMeetPro=0&fields={}",
        milesplit::OWNED_FIELDS
    );
    check!(eq; document["provenance"]["source_url"], url);
    check!(eq; document["provenance"]["capture_sha256"],
    "8db9804ebc2d36b6f7ec1e2a289b2b8ab28610e0063f0ee9f54245eb128071ed");
    check!(eq; document["provenance"]["fetched_at"], "2026-10-01T23:44:16Z");
    provenance::validate_relay_origin(&document)?;
    let page = owned_page(body.as_bytes())?;
    check!(eq; page.completeness, milesplit::OwnedCompleteness::Unknown);
    check!(eq; page.published_rows, 3);
    check!(eq; page.rows, Vec::new());
    check!(eq; page.rejected
        .iter()
        .map(|row| (row.locator.as_str(), row.kind))
        .collect::<Vec<_>>(),
    vec![
        ("data[0]", milesplit::OwnedRejectionKind::TeamRelay),
        ("data[1]", milesplit::OwnedRejectionKind::TeamRelay),
        ("data[2]", milesplit::OwnedRejectionKind::TeamRelay),
    ]);
    check!(page.individual_parse_complete());
    check!(!page.ownership_complete());
    Ok(())
}

fn owned_page(body: &[u8]) -> Result<milesplit::OwnedMeetPage> {
    match milesplit::parse_owned_meet(body, 725218) {
        milesplit::OwnedMeetVerdict::Parsed(page) => Ok(page),
        verdict => anyhow::bail!("authentic Troy owned capture was not parsed: {verdict:?}"),
    }
}
