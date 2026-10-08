use anyhow::{ensure, Context, Result};
use census_domain::UsJurisdiction;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct Capture {
    pub path: &'static str,
    pub bytes: u64,
    pub sha256: &'static str,
}

#[derive(Serialize)]
pub struct Event {
    pub capture: &'static Capture,
    pub event: u64,
    pub meet: u64,
    pub name: &'static str,
    pub state: UsJurisdiction,
    pub date: &'static str,
    pub acquired_at: &'static str,
    pub rows: usize,
    pub mapped: u64,
}

pub const XC: Capture = Capture {
    path: "crates/census-crawl/tests/fixtures/athleticlive_results/event-doc-2150205.json",
    bytes: 159_531,
    sha256: "d9a828d6ca14a531e93044b6f49ebcf84476e544c5d046cfc78c4a6521eb31e7",
};
pub const JUMP: Capture = Capture {
    path: "crates/census-crawl/tests/fixtures/athleticlive_results/event-doc-2254280.json",
    bytes: 17_175,
    sha256: "70fbe4248ff54a794b56c47a7fa450e283aa14ed75666884ce5d327c6ec423b4",
};
pub const TRACK: Capture = Capture {
    path: "research/sources/national-aggregators/samples/athleticlive/al-blob-ind-res-2254285.json",
    bytes: 30_436,
    sha256: "b40a93ad97c4e8d32cb3cdcd8be2fd3ab384777a254086b5fba5d558884de592",
};
pub const DIRECTORY: Capture = Capture {
    path: "crates/census-crawl/tests/fixtures/wiaa/directory_letter_a.html",
    bytes: 20_565,
    sha256: "0998d20dd8bc50f74cdd69036b19ca766a67ec79ae100a9c30f56d5ff69eb732",
};
pub const SCHOOL: Capture = Capture {
    path: "crates/census-crawl/tests/fixtures/wiaa/school_org1_abbotsford.html",
    bytes: 39_262,
    sha256: "34bab46fff0e4036cdfb7f2896938e9dd7e0ef7ad01b849a5c2c7a89308c04f2",
};

pub const EVENTS: [Event; 3] = [
    Event {
        capture: &XC,
        event: 2_150_205,
        meet: 58_504,
        name: "Iowa High School State Championships",
        state: UsJurisdiction::Iowa,
        date: "2025-10-31",
        acquired_at: "2026-09-22T03:59:19Z",
        rows: 136,
        mapped: 136,
    },
    Event {
        capture: &JUMP,
        event: 2_254_280,
        meet: 61_710,
        name: "MITS 4",
        state: UsJurisdiction::Michigan,
        date: "2026-02-14",
        acquired_at: "2026-09-22T04:01:13Z",
        rows: 17,
        mapped: 16,
    },
    Event {
        capture: &TRACK,
        event: 2_254_285,
        meet: 61_710,
        name: "MITS 4",
        state: UsJurisdiction::Michigan,
        date: "2026-02-14",
        acquired_at: "2026-09-22T03:55:22Z",
        rows: 43,
        mapped: 33,
    },
];

pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub fn body(capture: &Capture) -> Result<String> {
    let path = root().join(capture.path);
    ensure!(
        std::fs::metadata(&path)?.len() == capture.bytes,
        "capture size changed: {}",
        capture.path
    );
    let body =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", capture.path))?;
    Ok(body)
}

pub fn report() -> Result<()> {
    let captures = [&XC, &JUMP, &TRACK, &DIRECTORY, &SCHOOL];
    captures
        .iter()
        .try_for_each(|capture| verify_capture(capture))?;
    let oracle = include_str!("oracle.json");
    let metadata = serde_json::to_vec(&(&captures, &EVENTS))?;
    let mut digest = Sha256::new();
    digest.update(&metadata);
    digest.update(oracle.as_bytes());
    println!(
        "{}",
        serde_json::json!({
            "corpus_id": "captured-live-wiaa-co2027-v1", "corpus_sha256": format!("{:x}", digest.finalize()),
            "captures": captures, "oracle_sha256": format!("{:x}", Sha256::digest(oracle.as_bytes())),
            "events": EVENTS,
            "captured_bytes": 266969, "source_result_rows": 196, "expected_canonical_rows": 185,
            "expected_cohort": 60, "expected_prs": 14, "xc_prs_withheld": 45,
            "unknown_grade_rows": 1, "below_high_school_rows": 10,
            "school_labels": "structural team projection, not verified high-school membership",
            "contacts": "WIAA retained observations; no source-published tenure or capture timestamp",
            "timed": "parse, canonicalize, Fjall, consolidate, derive, XLSX and sidecar publication",
            "untimed": "independent frozen oracle, authoritative XLSX/CSV/JSONL readback, cleanup",
            "scale": "small captured qualification corpus; not a national-scale resource bound"
        })
    );
    Ok(())
}

fn verify_capture(capture: &Capture) -> Result<()> {
    let body = body(capture)?;
    ensure!(
        format!("{:x}", Sha256::digest(body.as_bytes())) == capture.sha256,
        "capture digest changed: {}",
        capture.path
    );
    Ok(())
}
