//! Property tests for the cross-country result seam.
//!
//! `census_crawl::xc::parse` reads the layouts WIAA's timers publish for cross-country —
//! Hy-Tek team blocks, the padded grade table and AccuRace's rule-lined table — and cross-country is
//! a class-of-2027 source rather than only a results source because every one of those layouts
//! prints the runner's grade. These properties pin what the census is allowed to assume about that
//! arm:
//!
//! * **Total-ness** — arbitrary text and text woven out of the tokens this parser keys on parse to a
//!   meet or to nothing, never a panic, and the same lines always give the same answer — see
//!   [`totalness`].
//! * **Accounting** — every row a file carries is counted and lands in the event of its own section:
//!   `rows_parsed` equals the rows the events hold, no event is empty, every row of these layouts
//!   carries a grade, and a section replayed after another keeps its own rows — see [`accounting`].
//! * **Append-only reading** — a truncated file parses to rows that are a prefix of the full file's
//!   rows, so a half-written page or a resumed walk can never invent a result — see [`prefix_laws`].
//! * **Page furniture** — the form feed of a PDF page break, a Windows carriage return and a timer's
//!   trailing column padding are the front end's business, never a row's: the lines a capture splits
//!   to, and the meet they are read into, do not move — see [`lines`].
//!
//! Deterministic by construction: [`seam_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x5843_5052_4F50_5331`, so a failing case is reproducible from the seed alone. The bodies below
//! are synthetic pages in the published layouts; the committed captures for the same layouts, and
//! the layout documentation they came from, live in `crates/census-crawl/src/xc/tests.rs`.

#![forbid(unsafe_code)]

use census_crawl::hytek::lines_from_text;
use census_crawl::result_file::{ParsedMeet, ParsedRow};
use census_crawl::xc::parse;
use census_domain::model::{Gender, SourceRef};
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "xc_parser_properties/accounting.rs"]
mod accounting;
#[path = "xc_parser_properties/lines.rs"]
mod lines;
#[path = "xc_parser_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "xc_parser_properties/totalness.rs"]
mod totalness;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// The season every body below was archived under: the padded table and the rule-lined table both
/// publish `October 25, 2025`, and the team blocks publish `11/1/2025`.
const ARCHIVE_YEAR: i16 = 2025;

/// Team-block layout: the state meet prints team score blocks whose scorers each carry a place,
/// grade and time. The file's first lines name the meet, the course and the date.
const STATE: &str = r#"
11/1/25, 1:38 PM                                                     WIAA State Cross Country Championships
                                                     WIAA State Cross Country Championships
                                                  The Ridges Golf Course, Wisconsin Rapids, WI
                                                                  11/1/2025
                                                      ========== BOYS TEAM SCORE ==========
                                                                  Division 1
    1.    69 SPASH                             (16:09.3 80:46.1 0:43.4)
  ===============================================
    1      6 Cooper Erickson                 12 15:50.2     5     28 Bennett Story               12   16:33.6
    2      9 Garrett Strong                  10 15:59.9     6   ( 32) Alex Dziak                 11   16:41.3
    3     10 Fisher Carroll                  9    16:03.1   7   ( 58) Donald Voetberg            12   17:06.6
"#;

/// Padded grade table: several sectionals publish one row per runner with a labelled grade column.
const TABLE: &str = r#"
WIAA D3 Sectional @ Sheboygan Lutheran
Overall Results
Place   Points   Bib   Name                        School                        Gender   Grade   Time      Pace
Boys Varsity
1       1        574   Wyatt See                   Poynette                      M        12      16:44.1   5:23
2       2        546   Nicholas Schubert           Ozaukee                       M        11      16:55.5   5:26
3       3        621   Eddy Giebler                Sheboygan Area Lutheran       M        10      17:00.7   5:28
4       4        573   Paceler Moll                Poynette                      M        10      17:18.1   5:34
"#;

/// Rule-lined table: AccuRace states its columns with a `====` rule rather than a labelled header,
/// and names the race in a `**** … ****` banner the walk re-anchors on.
const ACCURACE: &str = r#"
                           WIAA Division 3 Sectional Championship Meet
                   Baertschi & Keepers Property - Hosted by Albany High School
                                        Albany, Wisconsin
                                        October 25, 2025
                           Results provided by AccuRace Timing Services
                                      www.accuracetiming.com
                                  **** Boys' 5000 Meter Run ****
      Team Team                                                                         Avg   State
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    1    1 1/7 8223     Jonathan Simon        10   St. Ambrose/Abundant Life    16:21.6 5:16 t
    2    2 1/7 8156     Will Rzentkowski      11   Madison Country Day          16:45.7 5:24 t
    3    3 2/7 8222     David Simon           11   St. Ambrose/Abundant Life    16:55.4 5:27 t
"#;

/// The rule-lined layout with the boys race replayed after the girls race: the same file shape one
/// meet produces when a race is resumed later in the page. The names are distinct per block so a
/// row that lands in the wrong event is named in the failure.
const REPLAYED: &str = r#"
                           WIAA Division 3 Sectional Championship Meet
                                        Albany, Wisconsin
                                        October 25, 2025
                                  **** Boys' 5000 Meter Run ****
      Team Team                                                                         Avg   State
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    1    1 1/7 8223     Aaron Alpha           10   St. Ambrose/Abundant Life    16:21.6 5:16 t
    2    2 1/7 8156     Bram Beta             11   Madison Country Day          16:45.7 5:24 t
                                  **** Girls' 5000 Meter Run ****
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    1    1 1/7 8224     Cara Gamma            10   St. Ambrose/Abundant Life    18:21.6 5:56 t
    2    2 1/7 8157     Dana Delta            11   Madison Country Day          18:45.7 6:04 t
                                  **** Boys' 5000 Meter Run ****
Place Pts Place Bib#    Name                  Gr   Team                         Time    Mile Qual
===== ==== ===== ====   ===================== ==   ============================ ======= ===== =====
    3    3 1/7 8225     Ethan Epsilon         10   St. Ambrose/Abundant Life    16:30.0 5:19 t
    4    4 1/7 8158     Finn Zeta             11   Madison Country Day          16:55.0 5:27 t
"#;

/// The committed captures of the same three layouts, as the census parses them: the state meet's
/// team blocks, the sectional's padded table and the rule-lined sectional.
const LAYOUTS: [(&str, &str); 3] = [
    ("team blocks (state meet)", STATE),
    ("padded grade table (sectional)", TABLE),
    ("rule-lined table (AccuRace sectional)", ACCURACE),
];

/// The source every fixture is parsed as evidence for.
fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

// ---------------------------------------------------------------------------
// Plumbing
// ---------------------------------------------------------------------------

/// One body's lines, split the way the collector splits them.
fn lines(body: &str) -> Vec<String> {
    lines_from_text(body)
}

/// One line stream's parse, through the shipped entry point.
fn parse_lines(lines: &[String]) -> Option<ParsedMeet> {
    parse(lines, source(), ARCHIVE_YEAR)
}

/// One body's parse, through the shipped entry point.
fn parse_body(body: &str) -> Option<ParsedMeet> {
    parse_lines(&lines(body))
}

/// Every row of a meet in file order, rendered so two parses can be compared as values.
fn rendered_rows(meet: &ParsedMeet) -> Vec<String> {
    meet.events
        .iter()
        .flat_map(|event| event.rows.iter())
        .map(|row| format!("{row:?}"))
        .collect()
}

/// Every row of every event of one gender, in file order.
fn rows_of(meet: &ParsedMeet, gender: Gender) -> Vec<&ParsedRow> {
    meet.events
        .iter()
        .filter(|event| event.gender == gender)
        .flat_map(|event| event.rows.iter())
        .collect()
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

/// The lane's generators: fixed seed, fixed case count, one algorithm — a failure is reproducible
/// from the seed alone.
fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5843_5052_4F50_5331),
        ..ProptestConfig::default()
    }
}

/// Arbitrary text: every `char`, controls included, up to a few hundred of them.
fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..256).prop_map(|chars| chars.into_iter().collect())
}

/// Text built out of the tokens this parser keys on, woven in arbitrary order: a body of pure noise
/// never reaches the branches a malformed real page does.
fn shaped_body() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("\n".to_string()),
            Just("========== BOYS TEAM SCORE ==========".to_string()),
            Just("Boys' 5000 Meter Run".to_string()),
            Just("**** Girls' 5000 Meter Run ****".to_string()),
            Just("Division 1".to_string()),
            Just("Boys Varsity".to_string()),
            Just("Place   Points   Bib   Name".to_string()),
            Just("===== ==== ===== ====".to_string()),
            Just("1.    69 SPASH".to_string()),
            Just("1       1        574   Wyatt See".to_string()),
            Just("1      6 Cooper Erickson                 12 15:50.2".to_string()),
            Just("October 25, 2025".to_string()),
            Just("11/1/2025".to_string()),
            prop::num::u16::ANY.prop_map(|n| format!("{n:>4}")),
            any::<char>().prop_map(|c| c.to_string()),
        ],
        0..64,
    )
    .prop_map(|parts| parts.concat())
}
