use super::classify;
use super::parse::{
    clock_seconds, jurisdiction_of_url, list_filter, parse_list_page, parse_list_path,
    parse_team_page, parse_team_path, published_date, season_from_label, ParsedList, ParsedMark,
    ParsedRow, ParsedSection, TeamPath, YearToken,
};
use census_domain::model::{ExactSeconds, Gender, Grade, Sport};
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

mod cohort;
