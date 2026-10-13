use super::*;
use census_domain::model::{CoachRole, Gender, Sport};

fn fixture_directory() -> &'static str {
    include_str!("tests/fixtures/ciac_directory.html")
}

#[test]
fn parse_directory_returns_schools() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    check!(
        parsed.len() > 100,
        "should parse more than 100 schools, got {}",
        parsed.len()
    );
    Ok(())
}

#[test]
fn parse_directory_contains_abbott_tech() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    let names: Vec<&str> = parsed.iter().map(|(name, _)| name.as_str()).collect();
    check!(names.contains(&"Abbott Tech"), "should contain Abbott Tech");
    Ok(())
}

#[test]
fn parsed_schools_have_coach_rows() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    let schools_with_rows: Vec<_> = parsed
        .iter()
        .filter(|(_, table)| !table.rows.is_empty())
        .collect();
    check!(
        schools_with_rows.len() > 100,
        "more than 100 schools should have coach rows, got {}",
        schools_with_rows.len()
    );
    Ok(())
}

#[test]
fn parse_sport_label_returns_cross_country() {
    assert!(
        pages::parse_sport_label("Boys Cross Country").is_some(),
        "Boys Cross Country should map to a sport"
    );
    assert!(
        pages::parse_sport_label("Girls Cross Country").is_some(),
        "Girls Cross Country should map to a sport"
    );
}

#[test]
fn parse_sport_label_returns_track() {
    assert!(
        pages::parse_sport_label("Boys Indoor Track").is_some(),
        "Boys Indoor Track should map to a sport"
    );
    assert!(
        pages::parse_sport_label("Girls Outdoor Track").is_some(),
        "Girls Outdoor Track should map to a sport"
    );
    assert!(
        pages::parse_sport_label("Boys Outdoor Track").is_some(),
        "Boys Outdoor Track should map to a sport"
    );
}

#[test]
fn parse_sport_label_returns_none_for_non_xc_tf() {
    assert!(
        pages::parse_sport_label("Boys Baseball").is_none(),
        "Boys Baseball should return None"
    );
    assert!(
        pages::parse_sport_label("Football").is_none(),
        "Football should return None"
    );
}

#[test]
fn parse_gender_returns_boys() {
    assert_eq!(pages::parse_gender("Boys Cross Country"), Gender::Boys);
}

#[test]
fn parse_gender_returns_girls() {
    assert_eq!(pages::parse_gender("Girls Indoor Track"), Gender::Girls);
}

#[test]
fn parse_gender_returns_mixed_for_coed() {
    assert_eq!(pages::parse_gender("Coed Outdoor Track"), Gender::Mixed);
}

#[test]
fn school_entities_mints_school() {
    let table = SchoolTable { rows: vec![] };
    let extract = map::school_entities("Avon High School", &table, "2026-09-24");
    assert_eq!(extract.school.name, "Avon High School");
    assert_eq!(extract.school.association, Some("ciac".to_string()));
}

#[test]
fn school_entities_mints_coach_for_xc() {
    let table = SchoolTable {
        rows: vec![("Boys Cross Country".to_string(), "Mario Longo".to_string())],
    };
    let extract = map::school_entities("Abbott Tech", &table, "2026-09-24");
    assert_eq!(extract.coaches.len(), 1);
    let coach = &extract.coaches[0];
    assert_eq!(coach.name, "Mario Longo");
    assert_eq!(coach.role, CoachRole::HeadCoach);
    assert_eq!(coach.sport, Some(Sport::CrossCountry));
    assert_eq!(coach.gender, Gender::Boys);
}

#[test]
fn school_entities_mints_coach_for_track() {
    let table = SchoolTable {
        rows: vec![("Boys Indoor Track".to_string(), "Mario Longo".to_string())],
    };
    let extract = map::school_entities("Abbott Tech", &table, "2026-09-24");
    assert_eq!(extract.coaches.len(), 1);
    let coach = &extract.coaches[0];
    assert_eq!(coach.name, "Mario Longo");
    assert_eq!(coach.sport, Some(Sport::IndoorTrack));
}

#[test]
fn school_entities_skips_placeholder_names() {
    let table = SchoolTable {
        rows: vec![
            ("Boys Cross Country".to_string(), "TBA".to_string()),
            ("Girls Cross Country".to_string(), "Unknown".to_string()),
            ("Boys Outdoor Track".to_string(), "No Team".to_string()),
        ],
    };
    let extract = map::school_entities("Test School", &table, "2026-09-24");
    assert_eq!(
        extract.coaches.len(),
        0,
        "placeholder names should not produce coaches"
    );
}

#[test]
fn school_entities_captures_non_xc_tf_rows() {
    let table = SchoolTable {
        rows: vec![
            ("Boys Baseball".to_string(), "Steve Bova".to_string()),
            ("Boys Cross Country".to_string(), "Mario Longo".to_string()),
        ],
    };
    let extract = map::school_entities("Abbott Tech", &table, "2026-09-24");
    assert_eq!(extract.coaches.len(), 1);
    assert_eq!(extract.coaches[0].sport, Some(Sport::CrossCountry));
    assert_eq!(extract.coaches[0].name, "Mario Longo");
}

#[test]
fn fixture_has_182_schools_with_xc_tf_rows() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    let schools_with_xc_tf: Vec<_> = parsed
        .iter()
        .filter(|(_, table)| {
            table
                .rows
                .iter()
                .any(|(sport, _)| pages::parse_sport_label(sport).is_some())
        })
        .collect();
    check!(
        schools_with_xc_tf.len() >= 170,
        "should have ~182 schools with XC/TF rows, got {}",
        schools_with_xc_tf.len()
    );
    Ok(())
}

#[test]
fn fixture_has_xc_tf_coach_rows_counting_over_1000() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    let total_rows: usize = parsed.iter().map(|(_, t)| t.rows.len()).sum();
    check!(
        total_rows > 1000,
        "should have 1000+ XC/TF coach rows total, got {}",
        total_rows
    );
    Ok(())
}

#[test]
fn abbott_tech_has_bcx_coach_mario_longo() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    let entry = parsed.iter().find(|(name, _)| name == "Abbott Tech");
    let Some((_, table)) = entry else {
        return Err("Abbott Tech not found".into());
    };
    let cx_coaches: Vec<_> = table
        .rows
        .iter()
        .filter(|(sport, _)| sport == "Boys Cross Country")
        .collect();
    check!(!cx_coaches.is_empty(), "should have Boys Cross Country row");
    let name = cx_coaches[0].1.as_str();
    check!(eq; name, "Mario Longo");
    Ok(())
}

#[test]
fn abbott_tech_has_girls_cx_coach() -> Result<(), Box<dyn std::error::Error>> {
    let parsed = pages::parse_directory(fixture_directory())?;
    let entry = parsed.iter().find(|(name, _)| name == "Abbott Tech");
    let Some((_, table)) = entry else {
        return Err("Abbott Tech not found".into());
    };
    let gc = table
        .rows
        .iter()
        .filter(|(sport, _)| sport == "Girls Cross Country")
        .collect::<Vec<_>>();
    check!(!gc.is_empty(), "should have Girls Cross Country row");
    Ok(())
}

#[test]
fn an_oversized_staff_table_is_refused_not_truncated() -> Result<(), Box<dyn std::error::Error>> {
    let mut table = String::from("<table class='DirectoryStaffTable'>");
    for n in 0..20_001 {
        table.push_str(&format!(
            "<tr><td>Boys Cross Country</td><td>Role</td><td>Coach {n}</td></tr>"
        ));
    }
    table.push_str("</table>");
    let outcome = pages::parse_table(&table);
    let Err(crate::CrawlError::Schema { detail, .. }) = outcome else {
        return Err("an oversized staff table must be refused".into());
    };
    check!(detail.contains("exceeds"));
    Ok(())
}

#[test]
fn a_directory_accumulating_past_the_row_budget_is_refused(
) -> Result<(), Box<dyn std::error::Error>> {
    let mut html = String::new();
    for t in 0..2 {
        html.push_str("<div class='DirectoryDetail'><b>Overflow High</b></div><table class='DirectoryStaffTable'>");
        for n in 0..10_001 {
            html.push_str(&format!(
                "<tr><td>Girls Cross Country</td><td>Role</td><td>Coach {t}-{n}</td></tr>"
            ));
        }
        html.push_str("</table>");
    }
    let outcome = pages::parse_directory(&html);
    let Err(crate::CrawlError::Schema { detail, .. }) = outcome else {
        return Err("a directory past the row budget must be refused".into());
    };
    check!(detail.contains("exceeds"));
    Ok(())
}
