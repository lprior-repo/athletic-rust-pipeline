//! Property tests for the RaceDay Scoring export seam.
//!
//! `census_crawl::raceday::parse` reads the finish-list pages WIAA posts for
//! cross-country sectionals and state meets: one `<h3>` naming the race and one or more
//! `data-display` grids whose rows are the athletes. Cross-country feeds the class-of-2027 census
//! because every grid row prints the runner's `Year`, and the provider publishes no mark beyond the
//! finish time.
//!
//! These properties pin what the census may assume about that arm:
//!
//! * **Total-ness** — arbitrary markup and markup woven out of the tokens this reader keys on parse
//!   to a meet or to a typed refusal, never a panic, and the same body always gives the same answer
//!   ([`totalness`]).
//! * **Accounting** — `rows_parsed` is the rows the events carry, `rows_skipped` is the grid rows the
//!   layout could not read as a performance, and nothing without an athlete and a time is published
//!   ([`accounting`]).
//! * **Append-only reading** — a body cut short yields the readings the whole body starts with
//!   ([`prefix_laws`]).
//! * **Printed furniture** — the case a header label is printed in, the blanks a stylesheet pads a
//!   cell with, and the spelling a title gives its division decide nothing: the same grid reads to the
//!   same rows and the same race ([`casing`]).
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x5241_4345_4441_5931`. `CAPTURE` is the committed page this adapter was qualified against;
//! `DECLINED_ROW` is the shape of a grid row the layout cannot read.
#![forbid(unsafe_code)]

use census_crawl::raceday::parse;
use census_crawl::result_file::{ParsedMeet, ParsedRow};
use census_crawl::CrawlResult;
use census_domain::model::SourceRef;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "raceday_parser_properties/accounting.rs"]
mod accounting;
#[path = "raceday_parser_properties/casing.rs"]
mod casing;
#[path = "raceday_parser_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "raceday_parser_properties/totalness.rs"]
mod totalness;

/// Verbatim `<h3>` + finish-list table from
/// `https://www.wiaawi.org/Portals/0/PDF/Results/Cross_Country/2023/racinesectionalb.htm`
/// (WIAA Division 2 Racine sectional, boys race, 2023).
const CAPTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm");

/// A one-table export whose second row carries no athlete name: a grid row the layout cannot read as
/// a performance, beside one it can.
const DECLINED_ROW: &str = r#"
<h3>WIAA D3 XC Sectionals - Girls Race Team Finish List-XC</h3>
<table class="data-display">
<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Finish</th></tr></thead>
<tbody>
<tr><td>1</td><td>Ada Bell</td><td>11</td><td>Whitewater</td><td>19:02.10</td></tr>
<tr><td>2</td><td></td><td>12</td><td>East Troy</td><td>20:11.44</td></tr>
</tbody>
</table>
"#;

/// A titled page with no result table at all: the section index a collector can reach by mistake.
const NO_GRID: &str = r#"
<h3>WIAA D3 XC Sectionals - Girls Race</h3>
<p>Results posted after the meet.</p>
"#;

/// The season the file was archived under; RaceDay publishes no date of its own.
const ARCHIVE_YEAR: i16 = 2023;

/// The pages this seam's laws are stated over.
const LAYOUTS: [(&str, &str); 2] = [
    ("committed capture", CAPTURE),
    ("grid with a declined row", DECLINED_ROW),
];

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5241_4345_4441_5931),
        ..ProptestConfig::default()
    }
}

/// One body's parse, through the shipped entry point.
fn parse_body(body: &str) -> CrawlResult<ParsedMeet> {
    parse(body, source(), ARCHIVE_YEAR)
}

/// A row's reading, rendered for comparison.
fn rendered_reading(row: &ParsedRow) -> String {
    format!(
        "place={:?} name={:?} grade={:?} school={:?} mark={:?}",
        row.place, row.name, row.grade, row.school, row.mark
    )
}

/// Every event's rows, in file order, rendered for comparison.
fn rendered_rows(meet: &ParsedMeet) -> Vec<String> {
    meet.events
        .iter()
        .flat_map(|event| event.rows.iter())
        .map(rendered_reading)
        .collect()
}

/// Markup the arbitrary bodies are woven from: tag fragments, grid cells and times, so the reader
/// sees unbalanced tables and half-rows rather than prose.
fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("<h3>".to_string()),
            Just("</h3>".to_string()),
            Just("<table class=\"data-display\">".to_string()),
            Just("</table>".to_string()),
            Just("<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Finish</th></tr></thead>".to_string()),
            Just("<tr><td>".to_string()),
            Just("</td></tr>".to_string()),
            Just("WIAA D1 XC Sectionals - Boys Race".to_string()),
            Just("Team Finish List-XC".to_string()),
            Just("Ada Bell".to_string()),
            Just("Whitewater".to_string()),
            Just("19:02.10".to_string()),
            Just("11".to_string()),
            Just(" ".to_string()),
            Just("\n".to_string()),
            (0u8..10).prop_map(|n| n.to_string()),
            any::<char>().prop_map(|c| c.to_string()),
        ],
        0..64,
    )
    .prop_map(|parts| parts.concat())
}

/// Pages woven out of the tokens a finish list prints, so grids, rows and headings reach the reader
/// in orders no capture has.
fn shaped_body() -> impl Strategy<Value = String> {
    let rows = prop::collection::vec(
        prop_oneof![
            Just("<tr><td>1</td><td>Ada Bell</td><td>11</td><td>Whitewater</td><td>19:02.10</td></tr>".to_string()),
            Just("<tr><td>2</td><td>Jack Hefty</td><td>12</td><td>East Troy</td><td>17:13.69</td></tr>".to_string()),
            Just("<tr><td>3</td><td></td><td>12</td><td>East Troy</td><td>20:11.44</td></tr>".to_string()),
            Just("<tr class=\"team-summary\"><td>1</td><td>Whitewater</td><td></td><td>15</td><td>2:05:11</td></tr>".to_string()),
            (1u8..40).prop_map(|n| format!("<tr><td>{n}</td><td>Runner {n}</td><td>1{n}</td><td>Team {n}</td><td>1{n}:02.10</td></tr>")),
        ],
        0..12,
    );
    let heads = prop_oneof![
        Just("<h3>WIAA D1 XC Sectionals - Boys Race Team Finish List-XC</h3>".to_string()),
        Just("<h3>WIAA D2 XC Sectionals - Girls Race</h3>".to_string()),
        Just("<h3>Division 3 Boys Results</h3>".to_string()),
    ];
    (heads, any::<bool>(), rows).prop_map(|(head, table, rows)| {
        let body = rows.join("\n");
        if table {
            format!(
                "{head}\n<table class=\"data-display\">\n<thead><tr><th>Place</th><th>Name</th><th>Year</th><th>Team Name</th><th>Finish</th></tr></thead>\n<tbody>\n{body}\n</tbody>\n</table>\n"
            )
        } else {
            format!("{head}\n{body}\n")
        }
    })
}
