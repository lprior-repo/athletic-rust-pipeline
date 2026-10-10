use super::classify;
use super::parse::metric_metres;
use super::parse::{
    clock_seconds, jurisdiction_of_url, list_filter, parse_list_page, parse_list_path,
    parse_team_page, parse_team_path, published_date, season_from_label, ParsedAthlete, ParsedList,
    ParsedMark, ParsedMeet, ParsedRow, ParsedSection, ParsedTeam, TeamPath, YearToken,
};
use crate::tfrrs::map::{Absorb, ListContext, Page};
use census_domain::model::{CentiMetres, Mark};
use census_domain::model::{ExactSeconds, Gender, Grade, SourceRef, Sport};
use census_domain::school_index::SchoolIndex;
use census_domain::UsJurisdiction;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

const LIST: &str = include_str!("../../tests/fixtures/tfrrs/indiana_list_5489_excerpt.html");
const ROSTER: &str = include_str!("../../tests/fixtures/tfrrs/nh_pembroke_xc_team.html");
const HOME: &str = include_str!("../../tests/fixtures/tfrrs/indiana_home_teams.html");

fn section<'a>(list: &'a ParsedList, label: &str) -> TestResult<&'a ParsedSection> {
    list.sections
        .iter()
        .find(|section| section.label == label)
        .ok_or_else(|| "the capture publishes this event".into())
}

fn row(section: &ParsedSection, index: usize) -> TestResult<&ParsedRow> {
    section
        .rows
        .get(index)
        .ok_or_else(|| "the capture publishes this row".into())
}

#[test]
fn a_fresh_store_binds_schools_without_a_consolidated_jsonl() -> TestResult {
    use crate::AdapterContext;
    use census_domain::model::{CanonicalSchool, SchoolYear};
    use census_store::Store;
    use std::collections::HashMap;
    use std::time::Duration;
    let dir = tempfile::tempdir()?;
    let store = Store::open(dir.path())?;
    let fetcher = crate::net::Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        Vec::new(),
    )?
    .with_source("tfrrs")
    .with_offline(true);
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Indiana, "Pembroke", "pembroke", None);
    store.append(census_store::Table::Schools, &school)?;
    let context = AdapterContext {
        fetcher: &fetcher,
        store: &store,
        refresh: false,
        school_year: SchoolYear::new(2026).ok_or("2026 school year")?,
        observed_on: "2026-09-19".into(),
        performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 19).ok_or("snapshot date")?,
        recording: None,
    };
    check!(
        !store.out_dir().join("schools.jsonl").exists(),
        "a fresh store carries no consolidated jsonl"
    );
    let schools = super::consolidated_schools(&context)?;
    check!(eq; schools.len(), 1, "the live store is the binding source");
    Ok(())
}

#[test]
fn sections_keep_the_hosts_labels_sides_and_handles() -> TestResult {
    let list = parse_list_page(LIST);
    let expected = [
        ("60 Meters", Gender::Boys, 46),
        ("3200 Meters", Gender::Boys, 61),
        ("4 x 200 Relay", Gender::Boys, 71),
        ("High Jump", Gender::Boys, 64),
        ("Pole Vault", Gender::Girls, 65),
        ("Long Jump", Gender::Boys, 66),
    ];
    check!(eq; list.sections.len(), expected.len());
    for (index, (label, gender, handle)) in expected.iter().enumerate() {
        let section = list.sections.get(index).ok_or("section")?;
        check!(eq; section.label, *label);
        check!(eq; section.gender, Some(*gender));
        check!(eq; section.event_hnd, Some(*handle));
        check!(!section.rows.is_empty());
    }
    Ok(())
}

#[test]
fn a_sprint_row_reads_every_column_it_publishes() -> TestResult {
    let list = parse_list_page(LIST);
    let row = row(section(&list, "60 Meters")?, 1)?;
    check!(eq; row.mark, Some(ParsedMark::Time("6.98".to_string())));
    check!(eq; row.year, Some(YearToken::Sophomore));
    check!(eq; row.conv_metres, None);
    check!(eq; row.wind, None);
    check!(row
        .converted_note
        .as_deref()
        .is_some_and(|note| note.contains("6.49 (55)")));
    check!(row.place.is_some());
    check!(row
        .athlete
        .as_ref()
        .is_some_and(|athlete| athlete.id.is_some()));
    check!(row.team.as_ref().is_some_and(|team| !team.name.is_empty()));
    check!(row.meet.as_ref().is_some_and(|meet| meet.id.is_some()));
    check!(row.date.is_some());
    Ok(())
}

#[test]
fn a_track_size_row_keeps_the_hosts_conversion_note() -> TestResult {
    let list = parse_list_page(LIST);
    let row = row(section(&list, "3200 Meters")?, 0)?;
    check!(eq; row.mark, Some(ParsedMark::Time("9:15.75".to_string())));
    check!(eq; row.year, Some(YearToken::Junior));
    check!(row
        .converted_note
        .as_deref()
        .is_some_and(|note| note.contains("for Track Size")));
    Ok(())
}

#[test]
fn a_relay_row_lists_its_members_without_a_single_athlete() -> TestResult {
    let list = parse_list_page(LIST);
    let row = row(section(&list, "4 x 200 Relay")?, 1)?;
    check!(row.athlete.is_none());
    check!(row.relay_members.len() >= 2);
    check!(row
        .relay_members
        .iter()
        .all(|member| member.href_name.is_some()));
    check!(row.relay_members.iter().all(|member| member.id.is_some()));
    check!(row.team.as_ref().is_some_and(|team| !team.name.is_empty()));
    check!(matches!(row.mark, Some(ParsedMark::Time(_))));
    Ok(())
}

#[test]
fn a_field_row_reads_feet_inches_beside_the_hosts_metres() -> TestResult {
    let list = parse_list_page(LIST);
    let pole_vault = row(section(&list, "Pole Vault")?, 0)?;
    check!(pole_vault.year.is_none());
    check!(eq; pole_vault.conv_metres, Some(4.17));
    check!(matches!(
        pole_vault.mark.as_ref(),
        Some(ParsedMark::Field(token)) if token.contains('\'')
    ));
    check!(pole_vault
        .converted_note
        .as_deref()
        .is_some_and(|note| note.contains("Hand-Time")));
    let high_jump = row(section(&list, "High Jump")?, 0)?;
    check!(high_jump.year.is_none());
    check!(high_jump.conv_metres.is_some());
    check!(matches!(high_jump.mark, Some(ParsedMark::Field(_))));
    Ok(())
}

#[test]
fn a_team_page_publishes_its_roster_and_season() -> TestResult {
    let roster = parse_team_page(ROSTER);
    check!(!roster.athletes.is_empty());
    check!(roster
        .athletes
        .iter()
        .all(|athlete| athlete.href_name.is_some()));
    check!(roster
        .athletes
        .iter()
        .any(|athlete| athlete.full_name().is_some()));
    check!(roster.athletes.iter().any(|athlete| athlete.year.is_some()));
    check!(roster
        .athletes
        .iter()
        .any(|athlete| athlete.id == Some(9_264_598)));
    check!(eq; roster.school.as_deref(), Some("Pembroke"));
    let season = roster.season.ok_or("the page states its season")?;
    check!(eq; season.year, 2026);
    check!(eq; season.sport, Some(Sport::CrossCountry));
    Ok(())
}

#[test]
fn routes_name_the_state_and_the_season_they_publish() -> TestResult {
    let list_url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let list_path = parse_list_path(list_url).ok_or("list route")?;
    check!(eq; list_path.id, "5489");
    check!(eq; list_path.slug, "HSR_All_School_Performance_List");
    check!(eq;
        list_path.season.and_then(|season| season.sport),
        Some(Sport::IndoorTrack)
    );
    check!(eq;
        jurisdiction_of_url(list_url).map(UsJurisdiction::code),
        Some("IN")
    );
    check!(eq;
        list_filter(&format!("{list_url}?year=SR")),
        Some(YearToken::Senior)
    );
    check!(eq; list_filter(&format!("{list_url}?year=2024")), None);
    let team_url = "https://indiana.tfrrs.org/teams/tf/Avon_m.html";
    let team_path = parse_team_path(team_url).ok_or("team route")?;
    check!(eq; team_path.route, "tf");
    check!(eq; team_path.slug, "Avon");
    check!(eq; team_path.gender, Some(Gender::Boys));
    check!(eq;
        jurisdiction_of_url("https://nh.tfrrs.org/teams/xc/Pembroke_m.html")
            .map(UsJurisdiction::code),
        Some("NH")
    );
    check!(eq;
        jurisdiction_of_url("https://www.tfrrs.org/teams/tf/Avon_m.html"),
        None
    );
    check!(classify(list_url).is_some());
    check!(classify("https://indiana.tfrrs.org/").is_none());
    Ok(())
}

#[test]
fn the_home_page_publishes_the_team_route_family() {
    let routes: Vec<&str> = HOME
        .split("href=\"")
        .skip(1)
        .filter_map(|piece| piece.split('"').next())
        .filter(|href| href.contains("/teams/tf/"))
        .collect();
    assert!(routes.len() >= 2, "the capture publishes the family");
    let parsed: Vec<TeamPath> = routes
        .iter()
        .filter_map(|href| parse_team_path(href))
        .collect();
    assert_eq!(parsed.len(), routes.len(), "every published route parses");
    assert!(parsed
        .iter()
        .all(|path| path.route == "tf" && !path.slug.is_empty()));
    assert!(parsed.iter().any(|path| path.gender == Some(Gender::Boys)));
    assert!(parsed.iter().any(|path| path.gender == Some(Gender::Girls)));
}

#[test]
fn the_published_vocabulary_reads_the_hosts_tokens() -> TestResult {
    let cross_country = season_from_label("2026 NHIAA DII Cross Country").ok_or("season label")?;
    check!(eq; cross_country.year, 2026);
    check!(eq; cross_country.sport, Some(Sport::CrossCountry));
    check!(!cross_country.school_year_label);
    let school_year_label = season_from_label("2022-23 Indoor").ok_or("school-year label")?;
    check!(school_year_label.school_year_label);
    check!(eq;
        YearToken::parse("SR").and_then(YearToken::grade),
        Grade::new(12)
    );
    check!(eq; YearToken::parse("8").and_then(YearToken::grade), None);
    check!(eq; YearToken::parse("wk"), None);
    check!(eq; clock_seconds("1:26.56"), Some(ExactSeconds::parse("86.56")?));
    let date = published_date("Mar 28, 2026").ok_or("published date")?;
    check!(eq; date.iso, "2026-03-28");
    check!(eq; date.month, 3);
    check!(eq; published_date(""), None);
    Ok(())
}
const LIVE_LIST: &str =
    include_str!("../../tests/fixtures/tfrrs/indiana_list_5489_live_shape.html");

#[test]
fn the_live_list_shape_presents_the_same_rows_as_the_fixture() -> TestResult {
    let list = parse_list_page(LIVE_LIST);
    check!(eq; list.sections.len(), 1);
    let section = list.sections.first().ok_or("list section")?;
    check!(eq; section.label, "60 Meters");
    check!(eq; section.rows.len(), 2);
    Ok(())
}

#[test]
fn live_list_urls_without_year_segment_parse() -> TestResult {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List";
    let path = parse_list_path(url).ok_or("list route")?;
    check!(eq; path.id, "5489");
    check!(eq; path.slug, "HSR_All_School_Performance_List");
    check!(eq; path.season, None);
    check!(classify(url).is_some());
    Ok(())
}

#[test]
fn live_list_urls_with_query_filters_parse() -> TestResult {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List?year=SR";
    let path = parse_list_path(url).ok_or("list route")?;
    check!(eq; path.id, "5489");
    check!(eq; path.slug, "HSR_All_School_Performance_List");
    check!(eq; path.season, None);
    check!(eq; list_filter(url), Some(YearToken::Senior));
    check!(classify(url).is_some());
    Ok(())
}

#[test]
fn fixture_list_urls_with_year_segment_still_parse() -> TestResult {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let path = parse_list_path(url).ok_or("list route")?;
    check!(eq; path.id, "5489");
    check!(eq; path.slug, "HSR_All_School_Performance_List");
    check!(path.season.is_some());
    check!(classify(url).is_some());
    Ok(())
}

#[test]
fn metric_field_mark_without_conv_produces_distance_metres() -> TestResult {
    let mark = crate::tfrrs::map::row::mark_of(&ParsedMark::Field("6.50m".to_string()), None);
    check!(eq;
        mark,
        Mark::DistanceMetres(CentiMetres::new(650)),
        "6.50m is 650 cm"
    );
    Ok(())
}

#[test]
fn metric_metres_function_parses_metric_tokens() -> TestResult {
    check!(eq; metric_metres("6.50m"), Some(6.50));
    check!(eq; metric_metres("1.90M"), Some(1.90));
    check!(eq; metric_metres("40.1m"), Some(40.1));
    check!(eq; metric_metres("1.80"), None);
    check!(eq; metric_metres("6'5\""), None);
    Ok(())
}

#[test]
fn imperial_with_conv_still_works() -> TestResult {
    let mark = crate::tfrrs::map::row::mark_of(&ParsedMark::Field("7-2".to_string()), Some(2.18));
    match mark {
        Mark::FieldImperial { feet_mark, .. } => {
            check!(eq; feet_mark, "7-2");
        }
        other => return Err(format!("expected FieldImperial, got {other:?}").into()),
    }
    Ok(())
}

fn metric_row(
    name: &str,
    athlete_id: u64,
    mark: &str,
    conv_metres: Option<f64>,
    year: YearToken,
) -> ParsedRow {
    ParsedRow {
        place: Some(1),
        athlete: Some(ParsedAthlete {
            id: Some(athlete_id),
            name: name.to_string(),
            href_name: Some(name.to_string()),
        }),
        relay_members: Vec::new(),
        team: Some(ParsedTeam {
            name: "Kokomo".to_string(),
            path: "https://indiana.tfrrs.org/teams/tf/Kokomo_m.html".to_string(),
            gender: Some(Gender::Boys),
        }),
        year: Some(year),
        mark: Some(ParsedMark::Field(mark.to_string())),
        conv_metres,
        converted_note: None,
        meet: Some(ParsedMeet {
            name: "Hoosier State Relays".to_string(),
            id: Some(94159),
            path: None,
        }),
        date: published_date("Mar 28, 2026"),
        wind: None,
    }
}

fn metric_section(label: &str, handle: u32, rows: Vec<ParsedRow>) -> ParsedSection {
    ParsedSection {
        label: label.to_string(),
        gender: Some(Gender::Boys),
        event_hnd: Some(handle),
        rows,
    }
}

#[test]
fn metric_field_marks_keep_their_metres_without_a_conv_cell() -> TestResult {
    for (event, token, centimetres) in [
        ("High Jump", "1.90m", 190),
        ("Pole Vault", "5.00m", 500),
        ("Long Jump", "6.50m", 650),
        ("Triple Jump", "13.50m", 1350),
        ("Shot Put", "18.50m", 1850),
        ("Discus", "55.00m", 5500),
        ("Javelin", "60.00m", 6000),
        ("Hammer", "65.00m", 6500),
    ] {
        let mark = crate::tfrrs::map::row::mark_of(&ParsedMark::Field(token.to_string()), None);
        check!(eq;
            mark,
            Mark::DistanceMetres(CentiMetres::new(centimetres)),
            "{event} publishes {token}"
        );
    }
    Ok(())
}

#[test]
fn agreeing_conv_cells_keep_the_primary_published_mark() -> TestResult {
    let metric =
        crate::tfrrs::map::row::mark_of(&ParsedMark::Field("6.50m".to_string()), Some(6.50));
    check!(eq;
        metric,
        Mark::DistanceMetres(CentiMetres::new(650)),
        "an agreeing Conv confirms the metric primary"
    );
    let imperial =
        crate::tfrrs::map::row::mark_of(&ParsedMark::Field("23' 11.5\"".to_string()), Some(7.30));
    check!(eq;
        imperial,
        Mark::FieldImperial {
            feet_mark: "23' 11.5\"".to_string(),
            metres: CentiMetres::new(730),
        },
        "the captured long-jump imperial mark agrees with its Conv exactly"
    );
    Ok(())
}

#[test]
fn contradictory_conv_cells_withhold_the_numeric_mark() -> TestResult {
    let metric =
        crate::tfrrs::map::row::mark_of(&ParsedMark::Field("6.50m".to_string()), Some(7.30));
    check!(eq;
        metric,
        Mark::Raw("6.50m".to_string()),
        "a contradicting Conv must not mint a false numeric best"
    );
    let imperial =
        crate::tfrrs::map::row::mark_of(&ParsedMark::Field("7-2".to_string()), Some(9.99));
    check!(eq;
        imperial,
        Mark::Raw("7-2".to_string()),
        "a contradicting Conv must not mint a false imperial numeric either"
    );
    Ok(())
}

#[test]
fn conv_tolerance_boundary_agrees_at_two_centimetres() -> TestResult {
    let agree =
        crate::tfrrs::map::row::mark_of(&ParsedMark::Field("6.50m".to_string()), Some(6.52));
    check!(eq;
        agree,
        Mark::DistanceMetres(CentiMetres::new(650)),
        "a 2cm Conv difference still confirms the metric primary"
    );
    let contradict =
        crate::tfrrs::map::row::mark_of(&ParsedMark::Field("6.50m".to_string()), Some(6.53));
    check!(eq;
        contradict,
        Mark::Raw("6.50m".to_string()),
        "a 3cm Conv difference withholds the numeric mark"
    );
    Ok(())
}

#[test]
fn unparseable_time_tokens_are_retained_as_raw() -> TestResult {
    let kept = crate::tfrrs::map::row::mark_of(&ParsedMark::Time("NT".to_string()), None);
    check!(eq;
        kept,
        Mark::Raw("NT".to_string()),
        "a published but unparseable time token survives as Raw instead of dropping the row"
    );
    let timed = crate::tfrrs::map::row::mark_of(&ParsedMark::Time("10.56".to_string()), None);
    check!(eq;
        timed,
        Mark::TimeSeconds(ExactSeconds::parse("10.56")?),
        "a parseable time mints its exact seconds"
    );
    Ok(())
}

fn time_row(name: &str, athlete_id: u64, mark: &str, year: YearToken) -> ParsedRow {
    let mut row = metric_row(name, athlete_id, "6.50m", None, year);
    row.mark = Some(ParsedMark::Time(mark.to_string()));
    row
}

#[test]
fn unparseable_time_rows_are_admitted_without_numeric_marks() -> TestResult {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.to_string()));
    let route = parse_list_path(url).ok_or("list route")?;
    let context = ListContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
                .ok_or("snapshot date")?,
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: &route,
        filter: None,
    };
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    let page = ParsedList {
        sections: vec![metric_section(
            "60 Meters",
            46,
            vec![
                time_row("John Doe", 9_000_011, "NT", YearToken::Junior),
                time_row("Jane Roe", 9_000_012, "6.98", YearToken::Sophomore),
            ],
        )],
    };
    absorb.absorb_list(&context, &page)?;
    check!(eq; absorb.stats.rows_absorbed, 2, "both rows are admitted");
    check!(
        eq; absorb.stats.marks_unconverted, 1,
        "exactly the NT row lands in the unconverted bucket"
    );
    check!(
        eq; absorb.stats.rows_without_mark, 0,
        "no published mark is dropped as missing"
    );
    let marks: Vec<Mark> = absorb
        .accumulator
        .performances
        .values()
        .map(|performance| performance.mark.clone())
        .collect();
    check!(
        marks.contains(&Mark::Raw("NT".to_string())),
        "the NT row keeps its verbatim mark"
    );
    check!(eq;
        marks
            .iter()
            .filter(|mark| !matches!(mark, Mark::Raw(_)))
            .count(),
        1,
        "only the parseable row carries a numeric mark"
    );
    Ok(())
}

#[test]
fn metric_marks_without_conv_mint_exact_canonical_distance() -> TestResult {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.to_string()));
    let route = parse_list_path(url).ok_or("list route")?;
    let context = ListContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
                .ok_or("snapshot date")?,
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: &route,
        filter: None,
    };
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    let page = ParsedList {
        sections: vec![
            metric_section(
                "Long Jump",
                66,
                vec![metric_row(
                    "John Doe",
                    9_000_001,
                    "6.50m",
                    None,
                    YearToken::Junior,
                )],
            ),
            metric_section(
                "High Jump",
                64,
                vec![metric_row(
                    "Jane Roe",
                    9_000_002,
                    "1.90m",
                    None,
                    YearToken::Sophomore,
                )],
            ),
            metric_section(
                "Pole Vault",
                65,
                vec![metric_row(
                    "Jim Poe",
                    9_000_003,
                    "5.00m",
                    None,
                    YearToken::Senior,
                )],
            ),
        ],
    };
    absorb.absorb_list(&context, &page)?;
    check!(eq; absorb.stats.rows_absorbed, 3);
    check!(eq; absorb.accumulator.performances.len(), 3);
    for (token, centimetres) in [("6.50m", 650), ("1.90m", 190), ("5.00m", 500)] {
        check!(
            absorb
                .accumulator
                .performances
                .values()
                .any(|performance| performance.mark
                    == Mark::DistanceMetres(CentiMetres::new(centimetres))),
            "the {token} row carries an exact canonical numeric mark"
        );
    }
    let mut grades: Vec<u8> = absorb
        .accumulator
        .athletes
        .values()
        .filter_map(|athlete| {
            athlete
                .observed_grades
                .iter()
                .find_map(|grade| Some(grade.grade.get()))
        })
        .collect();
    grades.sort();
    check!(eq; grades, vec![10, 11, 12], "published grades stay published");
    Ok(())
}

#[test]
fn contradictory_conv_rows_keep_the_row_but_mint_no_numeric_best() -> TestResult {
    let url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let source = SourceRef::new("tfrrs_in", Some(url.to_string()));
    let route = parse_list_path(url).ok_or("list route")?;
    let context = ListContext {
        page: Page {
            source: &source,
            observed_on: "2026-09-30",
            performance_as_of: chrono::NaiveDate::from_ymd_opt(2026, 9, 30)
                .ok_or("snapshot date")?,
            jurisdiction: UsJurisdiction::Indiana,
        },
        list: &route,
        filter: None,
    };
    let index = SchoolIndex::from_schools(&[]);
    let mut absorb = Absorb::new(&index);
    let page = ParsedList {
        sections: vec![metric_section(
            "Long Jump",
            66,
            vec![metric_row(
                "John Doe",
                9_000_001,
                "6.50m",
                Some(7.30),
                YearToken::Junior,
            )],
        )],
    };
    absorb.absorb_list(&context, &page)?;
    check!(eq; absorb.accumulator.performances.len(), 1);
    let performance = absorb
        .accumulator
        .performances
        .values()
        .next()
        .ok_or("the contradictory row is retained")?;
    check!(eq;
        performance.mark,
        Mark::Raw("6.50m".to_string()),
        "contradiction withholds the numeric mark instead of minting a false best"
    );
    check!(eq; absorb.stats.marks_unconverted, 1);
    Ok(())
}

mod cohort;
mod grade_filter;
