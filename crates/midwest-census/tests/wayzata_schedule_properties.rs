//! Property tests for the Wayzata Results schedule seam.
//!
//! `midwest_census::sources::wayzata::schedule_rows` reads the provider's published schedules — the
//! page a weekly collector walks to find meets it has not seen. The only date evidence the provider
//! states is a season page URL carrying the year plus a month heading and a day cell per row, and the
//! only key a row carries is the provider's own `/links/<slug>` target, so both facts matter to the
//! incremental design: a mistaken date creates a meet that never happened, and a mistaken slug points
//! every later request at the wrong event.
//!
//! These properties pin what the census may assume:
//!
//! * **Total-ness** — arbitrary markup and markup woven out of the schedule's own tokens parse to
//!   rows or to a typed refusal, never a panic, and the same body always gives the same answer
//!   ([`totalness`]).
//! * **Shape** — every row is stamped with the season it was read for and carries a well-formed date,
//!   a label that is text rather than markup, and a provider key that is a slug ([`shapes`]).
//! * **Append-only reading** — a body cut short yields rows the whole body starts with
//!   ([`prefix_laws`]).
//! * **Printed spacing** — the whitespace runs a stylesheet leaves in a label and the case a month
//!   heading is printed in decide nothing: the same table publishes the same rows, dated in the same
//!   month ([`spacing`]).
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x5741_595A_4154_4131`. The pages below are the committed captures the walk was qualified
//! against; the collector's own tests exercise them end to end in `src/sources/wayzata/tests.rs`.
#![forbid(unsafe_code)]

use midwest_census::sources::wayzata::{schedule_rows, MeetRow};
use midwest_census::sources::CrawlResult;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "wayzata_schedule_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "wayzata_schedule_properties/shapes.rs"]
mod shapes;
#[path = "wayzata_schedule_properties/spacing.rs"]
mod spacing;
#[path = "wayzata_schedule_properties/totalness.rs"]
mod totalness;

/// The 2026 track schedule: opens in the indoor season and runs into the outdoor one.
const TRACK_2026: &str = include_str!("fixtures/wayzata/track_2026_schedule.html");

/// The 2026 cross-country schedule.
const XC_2026: &str = include_str!("fixtures/wayzata/xc_2026_schedule.html");

/// The pages this seam's laws are stated over.
const PAGES: [(&str, &str); 2] = [
    ("track schedule", TRACK_2026),
    ("cross-country schedule", XC_2026),
];

/// The season the captures were published for.
const SEASON: i16 = 2026;

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5741_595A_4154_4131),
        ..ProptestConfig::default()
    }
}

/// One page's rows for one season, through the shipped entry point.
fn rows(body: &str, year: i16) -> CrawlResult<Vec<MeetRow>> {
    schedule_rows(body, year)
}

/// The rows rendered for comparison.
fn rendered_rows(rows: &[MeetRow]) -> Vec<String> {
    rows.iter().map(|row| format!("{row:?}")).collect()
}

/// Markup the arbitrary bodies are woven from: table furniture, month headings, cells and links, so
/// the reader sees half-rows and stray anchors rather than prose.
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

/// Pages woven out of the tokens a schedule prints, so month headings, rows and links reach the
/// reader in orders no capture has.
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
