//! Unit tests for the `mpa` adapter: fixtures in, parsed rows out.

use super::*;
use census_domain::model::Sport;

// ── Fixture data ─────────────────────────────────────────────────────

fn fixture_directory() -> &'static str {
    include_str!("../../tests/fixtures/mpa/directory.html")
}

fn fixture_staff_bonny_eagle() -> &'static str {
    include_str!("../../tests/fixtures/mpa/staff_bonny_eagle_high.aspx")
}

// ── Directory parsing tests ───────────────────────────────────────────

#[test]
fn parse_directory_returns_schools() {
    let entries = pages::parse_directory(fixture_directory());
    assert!(
        entries.len() > 80,
        "should find many schools, got {}",
        entries.len()
    );
    // First school should be Ashland District School
    assert_eq!(entries[0].name, "Ashland District School");
    assert_eq!(entries[0].school_id, "25");
    // Second school should be Bangor Christian Schools
    assert_eq!(entries[1].name, "Bangor Christian Schools");
    // Third school should be Bangor High School
    assert_eq!(entries[2].name, "Bangor High School");
    assert_eq!(entries[2].school_id, "27");
}

#[test]
fn parse_directory_parses_many_schools() {
    let entries = pages::parse_directory(fixture_directory());
    // The directory has high schools + middle schools
    assert!(
        entries.len() > 100,
        "should find 100+ schools, got {}",
        entries.len()
    );
}

#[test]
fn parse_directory_no_empty_names() {
    let entries = pages::parse_directory(fixture_directory());
    for entry in &entries {
        assert!(
            !entry.name.is_empty(),
            "empty school name at index {}",
            entry.school_id
        );
    }
}

// ── Staff page parsing tests ──────────────────────────────────────────

#[test]
fn parse_staff_table_extracts_xc_coaches() {
    let rows = pages::parse_staff_table(fixture_staff_bonny_eagle());
    let xc_rows: Vec<_> = rows
        .iter()
        .filter(|r| r.sport.contains("Cross Country"))
        .collect();
    assert!(
        !xc_rows.is_empty(),
        "should find Cross Country rows, got {}",
        rows.len()
    );
    // Bonny Eagle High School's staff table carries a Boys Cross Country head coach
    let boys_xc = xc_rows.iter().find(|r| r.sport.contains("Boys"));
    assert!(boys_xc.is_some(), "should find Boys Cross Country row");
}

#[test]
fn parse_staff_table_extracts_track_coaches() {
    let rows = pages::parse_staff_table(fixture_staff_bonny_eagle());
    let track_rows: Vec<_> = rows
        .iter()
        .filter(|r| {
            r.sport.contains("Track") || r.sport.contains("Indoor") || r.sport.contains("Outdoor")
        })
        .collect();
    assert!(
        !track_rows.is_empty(),
        "should find Track rows, got {}",
        rows.len()
    );
}

#[test]
fn parse_staff_table_skips_non_head_coaches() {
    let rows = pages::parse_staff_table(fixture_staff_bonny_eagle());
    // All rows should be Head Coach entries (our filter)
    for row in &rows {
        assert!(
            !row.sport.contains("Principal") && !row.sport.contains("Athletic Director"),
            "should not include non-coach rows: {:?}",
            row
        );
    }
}

#[test]
fn parse_staff_table_captures_non_xc_sport() {
    let rows = pages::parse_staff_table(fixture_staff_bonny_eagle());
    // Verify that non-XC sports are still captured (e.g. Basketball, Football)
    let other_sports: Vec<_> = rows
        .iter()
        .filter(|r| {
            !r.sport.contains("Cross Country")
                && !r.sport.contains("Track")
                && !r.sport.contains("Indoor")
                && !r.sport.contains("Outdoor")
        })
        .collect();
    assert!(
        !other_sports.is_empty(),
        "should capture non-XC sports too, got {}",
        rows.len()
    );
}

#[test]
fn parse_staff_malformed_yields_empty() {
    let rows = pages::parse_staff_table("<html><body><p>No table here</p></body></html>");
    assert!(rows.is_empty(), "malformed HTML should yield 0 rows");
}

#[test]
fn parse_directory_malformed_yields_empty() {
    let entries = pages::parse_directory("<html><body><p>No schools</p></body></html>");
    assert!(entries.is_empty(), "malformed HTML should yield 0 entries");
}

// ── Sport label mapping tests ─────────────────────────────────────────

#[test]
fn parse_sport_label_maps_xc() {
    assert_eq!(
        map::parse_sport_label("Boys Cross Country"),
        Some(Sport::CrossCountry)
    );
    assert_eq!(
        map::parse_sport_label("Girls Cross Country"),
        Some(Sport::CrossCountry)
    );
}

#[test]
fn parse_sport_label_maps_track() {
    assert_eq!(
        map::parse_sport_label("Boys Indoor Track"),
        Some(Sport::IndoorTrack)
    );
    assert_eq!(
        map::parse_sport_label("Girls Outdoor Track"),
        Some(Sport::OutdoorTrack)
    );
    assert_eq!(
        map::parse_sport_label("Boys Outdoor Track"),
        Some(Sport::OutdoorTrack)
    );
}

#[test]
fn parse_sport_label_skips_other_sports() {
    assert_eq!(map::parse_sport_label("Basketball"), None);
    assert_eq!(map::parse_sport_label("Football"), None);
    assert_eq!(map::parse_sport_label(""), None);
}

// ── Entity building tests ─────────────────────────────────────────────

#[test]
fn school_entities_parses_xc_coaches() {
    use super::map::ParsedSchool;
    use super::pages::CoachRow;

    let entry = ParsedSchool {
        name: "Bangor High School".to_string(),
        school_id: "3".to_string(),
    };
    let staff_rows = vec![
        CoachRow {
            sport: "Boys Cross Country".to_string(),
            coach: "Ben Davis".to_string(),
        },
        CoachRow {
            sport: "Girls Cross Country".to_string(),
            coach: "Tom Noonan".to_string(),
        },
        CoachRow {
            sport: "Boys Basketball".to_string(),
            coach: "Steve Sawyer".to_string(),
        },
    ];

    let extract = map::school_entities(&entry, &staff_rows, "2026-09-24");

    // School
    assert_eq!(extract.school.name, "Bangor High School");
    assert_eq!(extract.school.association, Some("mpa".to_string()));

    // Coaches: 2 XC + 1 Basketball = 3 (only XC/TF are kept by parse_sport_label)
    assert_eq!(
        extract.coaches.len(),
        2,
        "should have 2 XC coaches (Basketball filtered out)"
    );

    // Verify XC coaches
    let names: Vec<&str> = extract.coaches.iter().map(|c| c.name.as_str()).collect();
    assert!(names.contains(&"Ben Davis"));
    assert!(names.contains(&"Tom Noonan"));
}

#[test]
fn school_entities_deduplicates_duplicate_sport_rows() {
    use super::map::ParsedSchool;
    use super::pages::CoachRow;

    let entry = ParsedSchool {
        name: "Test School".to_string(),
        school_id: "99".to_string(),
    };
    // Source sometimes lists the same sport twice
    let staff_rows = vec![
        CoachRow {
            sport: "Boys Cross Country".to_string(),
            coach: "Alice Smith".to_string(),
        },
        CoachRow {
            sport: "Boys Cross Country".to_string(),
            coach: "Bob Jones".to_string(),
        },
    ];

    let extract = map::school_entities(&entry, &staff_rows, "2026-09-24");

    // Should deduplicate to 1 coach row for Boys Cross Country, keeping the first published
    assert_eq!(
        extract.coaches.len(),
        1,
        "duplicate sport rows should be deduplicated"
    );
    assert_eq!(extract.coaches[0].name, "Alice Smith");
}

#[test]
fn school_entities_strips_the_coach_honorific() {
    // The association publishes some rows as "Coach <name>". The honorific is not part of the
    // person's name, and it is the published Coach column that would otherwise carry it.
    use super::map::ParsedSchool;
    use super::pages::CoachRow;

    let entry = ParsedSchool {
        name: "Test School".to_string(),
        school_id: "98".to_string(),
    };
    let staff_rows = vec![CoachRow {
        sport: "Boys Cross Country".to_string(),
        coach: "Coach Jane Doe".to_string(),
    }];

    let extract = map::school_entities(&entry, &staff_rows, "2026-09-24");

    assert_eq!(extract.coaches.len(), 1);
    assert_eq!(extract.coaches[0].name, "Jane Doe");
}

#[test]
fn school_entities_no_email_invented() {
    use super::map::ParsedSchool;
    use super::pages::CoachRow;

    let entry = ParsedSchool {
        name: "Test School".to_string(),
        school_id: "99".to_string(),
    };
    let staff_rows = vec![CoachRow {
        sport: "Boys Cross Country".to_string(),
        coach: "Coach A".to_string(),
    }];

    let extract = map::school_entities(&entry, &staff_rows, "2026-09-24");

    assert_eq!(extract.coaches.len(), 1);
    assert!(
        extract.coaches[0].professional_email.is_none(),
        "should not invent email"
    );
}
