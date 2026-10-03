use super::pages::{parse_directory, parse_sport_label};
use census_domain::model::{CoachRole, Gender, Sport};

mod acquisition;
mod failures;
mod support;

const FIXTURE: &str = include_str!("fixture.html");

#[test]
fn directory_preserves_published_schools_and_target_appointments() -> anyhow::Result<()> {
    let tables = parse_directory(FIXTURE);
    let barrington = tables
        .iter()
        .find(|table| table.name == "Barrington HS")
        .ok_or_else(|| anyhow::anyhow!("Barrington HS not found"))?;
    let appointments: Vec<_> = barrington
        .coach_rows
        .iter()
        .map(|row| (row.sport_label.as_str(), row.coach_name.as_str()))
        .collect();
    check!(appointments.contains(&("Boys Cross Country", "Mike Katz")));
    check!(appointments.contains(&("Girls Cross Country", "Molly Lacher-Katz")));
    check!(appointments.contains(&("Boys Indoor Track", "Bill Barrass")));
    check!(barrington.coach_rows.iter().all(|row| matches!(
        row.sport,
        Sport::CrossCountry | Sport::IndoorTrack | Sport::OutdoorTrack
    )));
    Ok(())
}

#[test]
fn mapping_preserves_school_role_and_sides_without_inventing_contact_claims() -> anyhow::Result<()>
{
    let table = parse_directory(support::DIRECTORY)
        .into_iter()
        .next()
        .ok_or_else(|| anyhow::anyhow!("school not found"))?;
    let capture = support::capture(support::DIRECTORY);
    let extract = super::school_entities(&table, &capture);
    check!(eq; extract.school.name, "Example HS");
    check!(eq; extract.school.postal_addresses, Vec::new());
    check!(eq; extract.coaches.len(), 2);
    let appointments: Vec<_> = extract
        .coaches
        .iter()
        .map(|coach| -> anyhow::Result<_> {
            check!(eq; coach.school, extract.school.id);
            check!(eq; coach.role, CoachRole::HeadCoach);
            check!(eq; coach.professional_email, None);
            check!(eq; coach.personal_email, None);
            check!(eq; coach.source_identities, Vec::new());
            check!(eq; coach.tenure_evidence, Vec::new());
            Ok((
                coach.name.as_str(),
                coach.sport,
                coach.gender,
                coach.phone.as_deref(),
            ))
        })
        .collect::<anyhow::Result<_>>()?;
    check!(eq;
        appointments,
        vec![
            (
                "Alex Coach",
                Some(Sport::CrossCountry),
                Gender::Boys,
                Some("401-555-0100")
            ),
            (
                "Morgan Coach",
                Some(Sport::OutdoorTrack),
                Gender::Girls,
                None
            ),
        ]
    );
    Ok(())
}

#[test]
fn sport_labels_preserve_target_disciplines_and_exclude_other_sports() {
    for (label, expected) in [
        ("Boys Cross Country", Some(Sport::CrossCountry)),
        ("Girls Cross Country", Some(Sport::CrossCountry)),
        ("Coed Cross Country", Some(Sport::CrossCountry)),
        ("Boys Indoor Track", Some(Sport::IndoorTrack)),
        ("Girls Indoor", Some(Sport::IndoorTrack)),
        ("Coed Outdoor Track", Some(Sport::OutdoorTrack)),
        ("Girls Outdoor", Some(Sport::OutdoorTrack)),
        ("Boys Basketball", None),
        ("Girls Volleyball", None),
    ] {
        assert_eq!(parse_sport_label(label), expected);
    }
}
