use std::collections::BTreeMap;

use anyhow::{Context, Result};
use census_crawl::{compiled, hytek};
use census_domain::model::{EventKind, Gender, Grade, Mark, SourceRef};
use serde::Serialize;

use super::{assert_rollup, golden_case, SEASON};

#[derive(Serialize)]
struct MeetFacts {
    name: String,
    date: String,
    end_date: Option<String>,
    timer: Option<String>,
    rows_parsed: usize,
    rows_skipped: usize,
    events: Vec<EventFacts>,
}

impl MeetFacts {
    fn of(meet: &compiled::ParsedMeet) -> Self {
        let compiled::ParsedMeet {
            name,
            date,
            end_date,
            timer,
            events,
            rows_parsed,
            rows_skipped,
        } = meet;
        Self {
            name: name.clone(),
            date: date.clone(),
            end_date: end_date.clone(),
            timer: timer.clone(),
            rows_parsed: *rows_parsed,
            rows_skipped: *rows_skipped,
            events: events.iter().map(EventFacts::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct EventFacts {
    label: String,
    kind: EventKind,
    gender: Gender,
    division: Option<String>,
    round: Option<String>,
    rows: Vec<RowFacts>,
}

impl EventFacts {
    fn of(event: &compiled::ParsedEvent) -> Self {
        let compiled::ParsedEvent {
            label,
            kind,
            gender,
            division,
            round,
            rows,
        } = event;
        Self {
            label: label.clone(),
            kind: kind.clone(),
            gender: *gender,
            division: division.clone(),
            round: round.clone(),
            rows: rows.iter().map(RowFacts::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct RowFacts {
    place: Option<u16>,
    name: String,
    grade: Option<Grade>,
    school: String,
    mark: Mark,
    wind_mps: Option<f64>,
    heat: Option<String>,
    points: Option<f64>,
    legs: Vec<LegFacts>,
}

impl RowFacts {
    fn of(row: &compiled::ParsedRow) -> Self {
        let compiled::ParsedRow {
            place,
            name,
            grade,
            school,
            mark,
            wind_mps,
            heat,
            points,
            legs,
            ..
        } = row;
        Self {
            place: *place,
            name: name.clone(),
            grade: *grade,
            school: school.clone(),
            mark: mark.clone(),
            wind_mps: *wind_mps,
            heat: heat.clone(),
            points: *points,
            legs: legs.iter().map(LegFacts::of).collect(),
        }
    }
}

#[derive(Serialize)]
struct LegFacts {
    position: u8,
    name: String,
    grade: Option<Grade>,
}

impl LegFacts {
    fn of(leg: &compiled::RelayLeg) -> Self {
        let compiled::RelayLeg {
            position,
            name,
            grade,
        } = leg;
        Self {
            position: *position,
            name: name.clone(),
            grade: *grade,
        }
    }
}

const REGIONAL_EXPORT: &str = r#"
05/26/2026, 09:56 PM                                  D1 Regional 8B - Appleton North
                                                        Appleton North HS  Tue, May 26, 2026
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
3      KIMBERLY                'A'          10:03.38           6            8    Olson, Denise           12    APPLETON EAST   13.55 q
"#;

const PAGE_STAMP_EXPORT: &str = "5/27/25, 8:35 PM                                              Manage D3 Regional 4B - Deerfield\n\
                          D3 Regional 4B - Deerfield\n\
                     Deerfield HS Track   Tue, May 27, 2025\n\
                                  Results\n";

const HEADERLESS_EXPORT: &str = "Girls' 100 Meters Division 1   Finals";

const COMPILED_EXPORTS: usize = 3;

#[test]
fn compiled_documented_exports_match_golden() -> Result<()> {
    let source = SourceRef::new("wiaa_results", None);
    let mut cases = BTreeMap::new();

    let regional = compiled::parse(
        &hytek::lines_from_pdf_text(REGIONAL_EXPORT),
        source.clone(),
        SEASON,
    );
    let regional = regional.context("the regional export has a meet header")?;
    anyhow::ensure!(
        regional.name == "D1 Regional 8B - Appleton North",
        "left={:?} right={:?}",
        &regional.name,
        &"D1 Regional 8B - Appleton North"
    );
    anyhow::ensure!(
        regional.date == "2026-05-26",
        "left={:?} right={:?}",
        &regional.date,
        &"2026-05-26"
    );
    anyhow::ensure!(
        regional.events.len() == 2,
        "one event per block — left={:?} right={:?}",
        &regional.events.len(),
        &2
    );
    let (name, digest) = golden_case("compiled__regional_export", &MeetFacts::of(&regional))?;
    cases.insert(name, digest);

    let page_stamp = compiled::parse(
        &hytek::lines_from_pdf_text(PAGE_STAMP_EXPORT),
        source.clone(),
        SEASON,
    );
    let (name, digest) = golden_case(
        "compiled__page_stamp_only_export",
        &page_stamp.as_ref().map(MeetFacts::of),
    )?;
    cases.insert(name, digest);

    let headerless = compiled::parse(
        &hytek::lines_from_pdf_text(HEADERLESS_EXPORT),
        source,
        SEASON,
    );
    let (name, digest) = golden_case(
        "compiled__headerless_export",
        &headerless.as_ref().map(MeetFacts::of),
    )?;
    cases.insert(name, digest);

    assert_rollup("compiled", cases, COMPILED_EXPORTS)
}
