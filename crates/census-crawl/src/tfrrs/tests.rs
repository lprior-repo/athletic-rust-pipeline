//! The TFRRS readers against captures of the pages the host actually served.
//!
//! | fixture | what it is | what it pins |
//! |---|---|---|
//! | `indiana_list_5489_excerpt.html` | byte-exact prefixes of six sections of `indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i` (1,880,743 B, HTTP 200), in document order | section labels/sides/event handles, the `Year` column, the `(55)`, `#` and `h` markers with their `title` conversions, the `Conv` column, the empty-`Year` row |
//! | `nh_pembroke_xc_team.html` | the whole `nh.tfrrs.org/teams/xc/Pembroke_Academy_m.html` capture (64,461 B) | the `ROSTER` table, the `YEAR` column, the school in the athlete routes, the page's own season control |
//! | `indiana_home_teams.html` | the whole `indiana.tfrrs.org/` capture (104,698 B) | the `/teams/tf/<School>_m.html` route family the instance publishes |
//!
//! Every assertion is on a row a page published, never on a string this adapter produced: the
//! fixture is the evidence, and a change that shifts a column is meant to break these tests.

use super::classify;
use super::parse::{
    clock_seconds, jurisdiction_of_url, list_filter, parse_list_page, parse_list_path,
    parse_team_page, parse_team_path, published_date, season_from_label, ParsedList, ParsedMark,
    ParsedRow, ParsedSection, TeamPath, YearToken,
};
use census_domain::model::{CentiSeconds, Gender, Grade, Sport};
use census_domain::UsJurisdiction;

const LIST: &str = include_str!("../../tests/fixtures/tfrrs/indiana_list_5489_excerpt.html");
const ROSTER: &str = include_str!("../../tests/fixtures/tfrrs/nh_pembroke_xc_team.html");
const HOME: &str = include_str!("../../tests/fixtures/tfrrs/indiana_home_teams.html");

/// The section a list page publishes for one event label.
fn section<'a>(list: &'a ParsedList, label: &str) -> &'a ParsedSection {
    list.sections
        .iter()
        .find(|section| section.label == label)
        .expect("the capture publishes this event")
}

/// One published row of a section, by position.
fn row(section: &ParsedSection, index: usize) -> &ParsedRow {
    section
        .rows
        .get(index)
        .expect("the capture publishes this row")
}

/// The fixture's sections keep the host's own labels, sides and event handles.
#[test]
fn sections_keep_the_hosts_labels_sides_and_handles() {
    let list = parse_list_page(LIST);
    let expected = [
        ("60 Meters", Gender::Boys, 46),
        ("3200 Meters", Gender::Boys, 61),
        ("4 x 200 Relay", Gender::Boys, 71),
        ("High Jump", Gender::Boys, 64),
        ("Pole Vault", Gender::Girls, 65),
        ("Long Jump", Gender::Boys, 66),
    ];
    assert_eq!(list.sections.len(), expected.len());
    for (index, (label, gender, handle)) in expected.iter().enumerate() {
        let section = list.sections.get(index).expect("section");
        assert_eq!(section.label, *label);
        assert_eq!(section.gender, Some(*gender));
        assert_eq!(section.event_hnd, Some(*handle));
        assert!(!section.rows.is_empty());
    }
}

/// A sprint row publishes place, athlete, grade, team, mark, meet and date — and its `(55)`
/// marker's note.
#[test]
fn a_sprint_row_reads_every_column_it_publishes() {
    let list = parse_list_page(LIST);
    let row = row(section(&list, "60 Meters"), 1);
    assert_eq!(row.mark, Some(ParsedMark::Time("6.98".to_string())));
    assert_eq!(row.year, Some(YearToken::Sophomore));
    assert_eq!(row.conv_metres, None);
    assert_eq!(row.wind, None);
    assert!(row
        .converted_note
        .as_deref()
        .is_some_and(|note| note.contains("6.49 (55)")));
    assert!(row.place.is_some());
    assert!(row
        .athlete
        .as_ref()
        .is_some_and(|athlete| athlete.id.is_some()));
    assert!(row.team.as_ref().is_some_and(|team| !team.name.is_empty()));
    assert!(row.meet.as_ref().is_some_and(|meet| meet.id.is_some()));
    assert!(row.date.is_some());
}

/// A `#` row publishes the host's own track-size conversion of its mark.
#[test]
fn a_track_size_row_keeps_the_hosts_conversion_note() {
    let list = parse_list_page(LIST);
    let row = row(section(&list, "3200 Meters"), 0);
    assert_eq!(row.mark, Some(ParsedMark::Time("9:15.75".to_string())));
    assert_eq!(row.year, Some(YearToken::Junior));
    assert!(row
        .converted_note
        .as_deref()
        .is_some_and(|note| note.contains("for Track Size")));
}

/// A relay row lists its members and mints no single athlete.
#[test]
fn a_relay_row_lists_its_members_without_a_single_athlete() {
    let list = parse_list_page(LIST);
    let row = row(section(&list, "4 x 200 Relay"), 1);
    assert!(row.athlete.is_none());
    assert!(row.relay_members.len() >= 2);
    assert!(row
        .relay_members
        .iter()
        .all(|member| member.href_name.is_some()));
    assert!(row.relay_members.iter().all(|member| member.id.is_some()));
    assert!(row.team.as_ref().is_some_and(|team| !team.name.is_empty()));
    assert!(matches!(row.mark, Some(ParsedMark::Time(_))));
}

/// A field row publishes feet–inches beside the host's own metric conversion, and a row the host
/// tracks without a class year publishes an empty `Year` cell.
#[test]
fn a_field_row_reads_feet_inches_beside_the_hosts_metres() {
    let list = parse_list_page(LIST);
    let pole_vault = row(section(&list, "Pole Vault"), 0);
    assert!(pole_vault.year.is_none());
    assert_eq!(pole_vault.conv_metres, Some(4.17));
    assert!(matches!(
        pole_vault.mark.as_ref(),
        Some(ParsedMark::Field(token)) if token.contains('\'')
    ));
    assert!(pole_vault
        .converted_note
        .as_deref()
        .is_some_and(|note| note.contains("Hand-Time")));
    let high_jump = row(section(&list, "High Jump"), 0);
    assert!(high_jump.year.is_none());
    assert!(high_jump.conv_metres.is_some());
    assert!(matches!(high_jump.mark, Some(ParsedMark::Field(_))));
}

/// A team page publishes its roster, the school its athlete routes name, and the season its own
/// control states.
#[test]
fn a_team_page_publishes_its_roster_and_season() {
    let roster = parse_team_page(ROSTER);
    assert!(!roster.athletes.is_empty());
    assert!(roster
        .athletes
        .iter()
        .all(|athlete| athlete.href_name.is_some()));
    assert!(roster
        .athletes
        .iter()
        .any(|athlete| athlete.full_name().is_some()));
    assert!(roster.athletes.iter().any(|athlete| athlete.year.is_some()));
    assert!(roster
        .athletes
        .iter()
        .any(|athlete| athlete.id == Some(9_264_598)));
    assert_eq!(roster.school.as_deref(), Some("Pembroke"));
    let season = roster.season.expect("the page states its season");
    assert_eq!(season.year, 2026);
    assert_eq!(season.sport, Some(Sport::CrossCountry));
}

/// The routes name the state the instance serves, the season a list path states and the `?year=`
/// view it was requested with — and a host that names no state is refused rather than placed.
#[test]
fn routes_name_the_state_and_the_season_they_publish() {
    let list_url = "https://indiana.tfrrs.org/lists/5489/HSR_All_School_Performance_List/2026/i";
    let list_path = parse_list_path(list_url).expect("list route");
    assert_eq!(list_path.id, "5489");
    assert_eq!(list_path.slug, "HSR_All_School_Performance_List");
    assert_eq!(
        list_path.season.and_then(|season| season.sport),
        Some(Sport::IndoorTrack)
    );
    assert_eq!(
        jurisdiction_of_url(list_url).map(UsJurisdiction::code),
        Some("IN")
    );
    assert_eq!(
        list_filter(&format!("{list_url}?year=SR")),
        Some(YearToken::Senior)
    );
    assert_eq!(list_filter(&format!("{list_url}?year=2024")), None);
    let team_url = "https://indiana.tfrrs.org/teams/tf/Avon_m.html";
    let team_path = parse_team_path(team_url).expect("team route");
    assert_eq!(team_path.route, "tf");
    assert_eq!(team_path.slug, "Avon");
    assert_eq!(team_path.gender, Some(Gender::Boys));
    assert_eq!(
        jurisdiction_of_url("https://nh.tfrrs.org/teams/xc/Pembroke_m.html")
            .map(UsJurisdiction::code),
        Some("NH")
    );
    assert_eq!(
        jurisdiction_of_url("https://www.tfrrs.org/teams/tf/Avon_m.html"),
        None
    );
    assert!(classify(list_url).is_some());
    assert!(classify("https://indiana.tfrrs.org/").is_none());
}

/// The Indiana home page publishes the same team-route family the list rows link, on both sides.
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

/// The season label, the grade vocabulary and the published date read the host's own tokens.
#[test]
fn the_published_vocabulary_reads_the_hosts_tokens() {
    let cross_country = season_from_label("2026 NHIAA DII Cross Country").expect("season label");
    assert_eq!(cross_country.year, 2026);
    assert_eq!(cross_country.sport, Some(Sport::CrossCountry));
    assert!(!cross_country.school_year_label);
    let school_year_label = season_from_label("2022-23 Indoor").expect("school-year label");
    assert!(school_year_label.school_year_label);
    assert_eq!(
        YearToken::parse("SR").and_then(YearToken::grade),
        Grade::new(12)
    );
    assert_eq!(YearToken::parse("8").and_then(YearToken::grade), None);
    assert_eq!(YearToken::parse("wk"), None);
    assert_eq!(clock_seconds("1:26.56"), Some(CentiSeconds(8656)));
    let date = published_date("Mar 28, 2026").expect("published date");
    assert_eq!(date.iso, "2026-03-28");
    assert_eq!(date.month, 3);
    assert_eq!(published_date(""), None);
}
