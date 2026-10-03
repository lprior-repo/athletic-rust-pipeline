#![forbid(unsafe_code)]

use census_crawl::compiled::{parse, ParsedMeet, ParsedRow};
use census_crawl::hytek::lines_from_pdf_text;
use census_domain::model::SourceRef;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[macro_use]
#[path = "../../../tools/fallible_checks.rs"]
mod fallible_checks;

#[path = "compiled_parser_properties/accounting.rs"]
mod accounting;
#[path = "compiled_parser_properties/lines.rs"]
mod lines;
#[path = "compiled_parser_properties/prefix_laws.rs"]
mod prefix_laws;
#[path = "compiled_parser_properties/totalness.rs"]
mod totalness;

const REGIONAL: &str = r#"
05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North
                                                       Appleton North HS  Tue, May 26, 2026
                                                                     Results
Girls' 4x800 Relay Division 1                     Finals                    Girls' 100 Meters Division 1              Prelims
       Team                    Relay        Finals             Pts               Athlete                 Yr Team              Prelims
1      HORTONVILLE             'A'          9:55.11            10           1    Parrish, Ashley         11   APPLETON NOR…   12.30 Q
    1) Wloszczynski, Lexi 10         2) Young, Ellie 9                      2    Thompson, Emily         12   APPLETON NOR…   12.74 q
    3) Falbo, Hailey 12              4) Huza, Hannah 12                     3    Rades, Jayla            11   HORTONVILLE     12.83 Q
                                                                            4    Rezash, Johannah        12   WEST DE PERE    13.12 q
2      APPLETON NORTH          'A'          9:56.50            8
                                                                            5    Lopez, Eilianyz         10   WEST DE PERE    13.38 q
    1) Dehlinger, Audry 11           2) Brazzale, Elise 10                  6    Hammen, Allie           9    APPLETON WEST   13.39 q
    3) Busch, Sophia 12              4) Helmbrecht, Ava 12
                                                                            7    Josephson, Sydney       9    KAUKAUNA        13.44 q
3      KIMBERLY                'A'          10:03.38           6            8    Olson, Denise           12   APPLETON EAST   13.55 q
"#;

const HEAT: &str = r#"
05/27/2026, 08:12 PM                                  D2 Regional 3A - Seymour
                                                       Seymour HS  Wed, May 27, 2026
                                                                     Results
Girls' 100 Meters Division 2              Prelims
       Athlete                 Yr Team              Prelims
1      Parrish, Ashley         11   SEYMOUR         12.30 Q
2      Thompson, Emily         12   SEYMOUR         12.74 q
"#;

const ARCHIVE_YEAR: i16 = 2026;

const LAYOUTS: [(&str, &str); 2] = [
    ("two-block regional page", REGIONAL),
    ("single heat page", HEAT),
];

fn source() -> SourceRef {
    SourceRef::new("wiaa_results", None)
}

fn seam_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x434F_4D50_494C_4544),
        ..ProptestConfig::default()
    }
}

fn lines(body: &str) -> Vec<String> {
    lines_from_pdf_text(body)
}

fn parse_lines(lines: &[String]) -> Option<ParsedMeet> {
    parse(lines, source(), ARCHIVE_YEAR)
}

fn parse_body(body: &str) -> Option<ParsedMeet> {
    parse_lines(&lines(body))
}

fn rendered_reading(row: &ParsedRow) -> String {
    format!(
        "place={:?} name={:?} grade={:?} school={:?} mark={:?} points={:?}",
        row.place, row.name, row.grade, row.school, row.mark, row.points
    )
}

fn arbitrary_body() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just("<td>".to_string()),
            Just("</td>".to_string()),
            Just("<tr>".to_string()),
            Just("Results".to_string()),
            Just("Girls".to_string()),
            Just("'".to_string()),
            Just("4x800 Relay".to_string()),
            Just("Division 1".to_string()),
            Just("Finals".to_string()),
            Just("Pts".to_string()),
            Just("Yr".to_string()),
            Just("Athlete".to_string()),
            Just("Team".to_string()),
            Just("12.30".to_string()),
            Just("Q".to_string()),
            Just("1)".to_string()),
            Just("05/26/2026".to_string()),
            Just(",".to_string()),
            Just(" ".to_string()),
            Just("\n".to_string()),
            (0u8..10).prop_map(|n| n.to_string()),
            (any::<char>().prop_map(|c| c.to_string())),
        ],
        0..64,
    )
    .prop_map(|parts| parts.concat())
}

fn shaped_body() -> impl Strategy<Value = String> {
    let tokens = prop::collection::vec(
        prop_oneof![
            Just("Girls' 4x800 Relay Division 1                     Finals".to_string()),
            Just("Girls' 100 Meters Division 1              Prelims".to_string()),
            Just("       Team                    Relay        Finals             Pts".to_string()),
            Just("               Athlete                 Yr Team              Prelims".to_string()),
            Just("1      HORTONVILLE             'A'          9:55.11            10".to_string()),
            Just("           1    Parrish, Ashley         11   APPLETON NOR…   12.30 Q".to_string()),
            Just("    1) Wloszczynski, Lexi 10         2) Young, Ellie 9".to_string()),
            Just("05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North".to_string()),
            Just("                                                       Appleton North HS  Tue, May 26, 2026".to_string()),
            Just("                                                                     Results".to_string()),
            Just("3      KIMBERLY                'A'          10:03.38           6".to_string()),
            (1u8..9).prop_map(|n| format!("{n}      TEAM                 'A'          9:5{n}.11            1{n}")),
        ],
        0..24,
    );
    (any::<bool>(), tokens).prop_map(|(separated, parts)| {
        if separated {
            parts.join("\n")
        } else {
            parts.concat()
        }
    })
}
