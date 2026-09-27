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

const ARCHIVE_YEAR: i16 = 2025;

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

const LAYOUTS: [(&str, &str); 3] = [
    ("team blocks (state meet)", STATE),
    ("padded grade table (sectional)", TABLE),
    ("rule-lined table (AccuRace sectional)", ACCURACE),
];

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

fn lines(body: &str) -> Vec<String> {
    lines_from_text(body)
}

fn parse_lines(lines: &[String]) -> Option<ParsedMeet> {
    parse(lines, source(), ARCHIVE_YEAR)
}

fn parse_body(body: &str) -> Option<ParsedMeet> {
    parse_lines(&lines(body))
}

fn rendered_rows(meet: &ParsedMeet) -> Vec<String> {
    meet.events
        .iter()
        .flat_map(|event| event.rows.iter())
        .map(|row| format!("{row:?}"))
        .collect()
}

fn rows_of(meet: &ParsedMeet, gender: Gender) -> Vec<&ParsedRow> {
    meet.events
        .iter()
        .filter(|event| event.gender == gender)
        .flat_map(|event| event.rows.iter())
        .collect()
}

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x5843_5052_4F50_5331),
        ..ProptestConfig::default()
    }
}

fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(any::<char>(), 0..256).prop_map(|chars| chars.into_iter().collect())
}

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
