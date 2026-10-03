#![forbid(unsafe_code)]

use census_crawl::wayzata::{schedule_rows, MeetRow};
use census_crawl::CrawlResult;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "wayzata_schedule_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "wayzata_schedule_properties/shapes.rs"]
mod shapes;
#[path = "wayzata_schedule_properties/spacing.rs"]
mod spacing;
#[path = "wayzata_schedule_properties/totalness.rs"]
mod totalness;

const TRACK_2026: &str =
    include_str!("../../census-crawl/tests/fixtures/wayzata/track_2026_schedule.html");

const XC_2026: &str =
    include_str!("../../census-crawl/tests/fixtures/wayzata/xc_2026_schedule.html");

const PAGES: [(&str, &str); 2] = [
    ("track schedule", TRACK_2026),
    ("cross-country schedule", XC_2026),
];

const SEASON: i16 = 2026;

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5741_595A_4154_4131),
        ..ProptestConfig::default()
    }
}

fn rows(body: &str, year: i16) -> CrawlResult<Vec<MeetRow>> {
    schedule_rows(body, year)
}

fn rendered_rows(rows: &[MeetRow]) -> Vec<String> {
    rows.iter().map(|row| format!("{row:?}")).collect()
}

fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("<table>".to_string()),
            Just("</table>".to_string()),
            Just("<tr class=\"month-title\">".to_string()),
            Just("<tr class=\"event-row\">".to_string()),
            Just("</tr>".to_string()),
            Just("<td class=\"date\">".to_string()),
            Just("<td class=\"awayteam\">".to_string()),
            Just("<td class=\"hometeam\">".to_string()),
            Just("</td>".to_string()),
            Just("<span title=\"USATF Minnesota All-Comers Meet #3\">".to_string()),
            Just("<a href=\"/links/7vqvs7\" aria-label=\"January 4 USATF Minnesota\">".to_string()),
            Just("January".to_string()),
            Just("August".to_string()),
            Just("University of Minnesota".to_string()),
            Just("4".to_string()),
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
            Just("<tr class=\"event-row\"><td class=\"date\"><span title=\"January 4\">4</span></td><td class=\"awayteam\"><span title=\"USATF Minnesota All-Comers Meet #3\">USATF Minnesota</span></td><td class=\"hometeam\"><span title=\"University of Minnesota\">Minnesota</span></td><td><a href=\"/links/7vqvs7\" aria-label=\"January 4 USATF Minnesota All-Comers Meet #3\">Details</a></td></tr>".to_string()),
            Just("<tr class=\"event-row\"><td class=\"date\">27</td><td class=\"awayteam\">River Falls Extreme Meet</td><td class=\"hometeam\">UW-River Falls</td><td><a href=\"/links/v70kmd\">Details</a></td></tr>".to_string()),
            Just("<tr class=\"event-row\"><td class=\"date\">12</td><td class=\"awayteam\"></td><td class=\"hometeam\">Milaca</td><td><a href=\"/links/\">Details</a></td></tr>".to_string()),
            (1u8..28).prop_map(|n| format!("<tr class=\"event-row\"><td class=\"date\">{n}</td><td class=\"awayteam\">Meet {n}</td><td class=\"hometeam\">Venue {n}</td><td><a href=\"/links/slug-{n}\" aria-label=\"Meet {n} at Venue {n}\">Details</a></td></tr>")),
        ],
        0..12,
    );
    let months = prop::collection::vec(
        prop_oneof![
            Just("<tr class=\"month-title\"><td>January</td></tr>".to_string()),
            Just("<tr class=\"month-title\"><td>August</td></tr>".to_string()),
            Just("<tr class=\"month-title\"><td>December</td></tr>".to_string()),
            Just("<tr class=\"month-title\"><td>Not A Month</td></tr>".to_string()),
        ],
        0..4,
    );
    (months, rows).prop_map(|(months, rows)| {
        let mut parts = months.clone();
        parts.extend(rows);
        parts.push("<tr class=\"event-row\"><td class=\"date\">1</td><td class=\"awayteam\">Meet</td><td class=\"hometeam\">Venue</td></tr>".to_string());
        format!("<table>\n{}\n</table>\n", parts.join("\n"))
    })
}
