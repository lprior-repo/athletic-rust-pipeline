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

const CAPTURE: &str =
    include_str!("../../census-crawl/tests/fixtures/wiaa_results/racinesectionalb-finish-list.htm");

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

const NO_GRID: &str = r#"
<h3>WIAA D3 XC Sectionals - Girls Race</h3>
<p>Results posted after the meet.</p>
"#;

const ARCHIVE_YEAR: i16 = 2023;

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

fn parse_body(body: &str) -> CrawlResult<ParsedMeet> {
    parse(body, source(), ARCHIVE_YEAR)
}

fn rendered_reading(row: &ParsedRow) -> String {
    format!(
        "place={:?} name={:?} grade={:?} school={:?} mark={:?}",
        row.place, row.name, row.grade, row.school, row.mark
    )
}

fn rendered_rows(meet: &ParsedMeet) -> Vec<String> {
    meet.events
        .iter()
        .flat_map(|event| event.rows.iter())
        .map(rendered_reading)
        .collect()
}

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
