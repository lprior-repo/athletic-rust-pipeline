use super::*;
use crate::athlete_packet::single_packet;
use census_domain::model::ATHLETE_GENDER_UNVERIFIED_FAMILY;

#[test]
fn quarantine_packet_names_the_question_field_and_values() -> TestResult {
    let athlete = athlete("Casey Runner", Gender::Unknown, GradYear::CO2027)?;
    let case = ReviewCase::pending(
        ATHLETE_GENDER_UNVERIFIED_FAMILY,
        athlete.id.as_str(),
        "Casey Runner (Madison West High School)",
        "no source supplied a boys or girls competition category",
    );
    let packet = single_packet(
        &case,
        &athlete,
        "what is the athlete competition gender?",
        "gender",
        "boys | girls",
    )?;
    check!(eq; stated(&packet, "census", "question"), Some("what is the athlete competition gender?".to_string()));
    check!(eq; stated(&packet, "census", "answer_field"), Some("gender".to_string()));
    check!(eq; stated(&packet, "census", "answer_values"), Some("boys | girls".to_string()));
    check!(eq; stated(&packet, "census", "side_a_gender"), Some("Unknown".to_string()));
    check!(eq; stated(&packet, "census", "side_a_name"), Some("Casey Runner".to_string()));
    Ok(())
}
