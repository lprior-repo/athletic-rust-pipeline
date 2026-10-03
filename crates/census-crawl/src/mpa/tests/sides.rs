use super::fixture_directory;
use crate::mpa::{map::ParsedSchool, parse_directory, parse_staff_table, school_entities};
use census_domain::model::{CoachRole, Gender, SourceNamespace, Sport};

#[test]
fn school_entities_preserve_explicit_sides_for_each_supported_sport() -> anyhow::Result<()> {
    let school = parse_directory(fixture_directory())
        .into_iter()
        .find(|entry| entry.name == "Bonny Eagle High School")
        .ok_or_else(|| anyhow::anyhow!("fixture school missing"))?;
    let entry = ParsedSchool {
        name: school.name,
        school_id: school.school_id,
    };
    [
        ("Cross Country", Sport::CrossCountry),
        ("Indoor Track", Sport::IndoorTrack),
        ("Outdoor Track", Sport::OutdoorTrack),
        ("Indoor", Sport::IndoorTrack),
        ("Outdoor", Sport::OutdoorTrack),
    ]
    .into_iter()
    .try_for_each(|(label, sport)| assert_sides(&entry, label, sport))
}

fn assert_sides(entry: &ParsedSchool, label: &str, sport: Sport) -> anyhow::Result<()> {
    let cases = [
        ("Boys", Gender::Boys),
        ("Girls", Gender::Girls),
        ("Coed", Gender::Mixed),
    ];
    let rows = cases
        .iter()
        .map(|(side, _)| {
            format!("<tr><td>{side} {label}</td><td>Head Coach</td><td>{side} Coach</td></tr>")
        })
        .collect::<String>();
    let html = format!("<table class='DirectoryStaffTable'>{rows}</table>");
    let parsed = parse_staff_table(&html);
    let extract = school_entities(entry, &parsed, "2026-09-01T10:00:00Z");
    check!(eq;
        extract.school.evidence[0].observed_on,
        "2026-09-01T10:00:00Z"
    );
    check!(eq;
        extract.school.evidence[0].source.url.as_deref(),
        Some("https://www.mpa.cc/SchoolPages/School.aspx?SchoolID=3")
    );
    check!(eq; extract.coaches.len(), 3, "{label}");
    cases.into_iter().try_for_each(|(side, gender)| {
        let name = format!("{side} Coach");
        let coach = extract
            .coaches
            .iter()
            .find(|coach| coach.name == name)
            .ok_or_else(|| anyhow::anyhow!("missing {side} {label} coach"))?;
        check!(eq; coach.sport, Some(sport), "{side} {label}");
        check!(eq; coach.gender, gender, "{side} {label}");
        check!(eq; coach.role, CoachRole::HeadCoach);
        check!(eq; coach.school, extract.school_id);
        check!(eq; coach.professional_email, None);
        check!(eq;
            coach.source_identities[0].namespace,
            SourceNamespace::association_school("mpa")
        );
        check!(eq;
            coach.source_identities[0].id,
            format!("coach:3:{}:{}", sport.stable_key(), gender.stable_key())
        );
        check!(eq; coach.evidence[0].observed_on, "2026-09-01T10:00:00Z");
        check!(eq;
            coach.evidence[0].source.url.as_deref(),
            Some("https://www.mpa.cc/SchoolPages/School.aspx?SchoolID=3&tab=staff")
        );
        Ok(())
    })
}
