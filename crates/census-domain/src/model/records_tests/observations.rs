//! What an observation row says: the provider's own object, in the source's own words.

use super::super::*;
use crate::model::{Gender, GradYear, Grade, ObservedGrade, SchoolYear, SourceNamespace, SourceRef};

#[test]
fn an_athlete_observation_keeps_what_its_source_published() {
    let observation = SourceAthleteObservation::new(
        SourceNamespace::MilesplitAthlete,
        "14399169",
        "roster:madison-west:2025-26",
        "Jordan Smith",
        "2026-09-22",
    )
    .with_school(Some("Madison West".to_string()))
    .with_grade(Some(ObservedGrade {
        grade: Grade::new(10).expect("a grade in 9..=12"),
        school_year: SchoolYear::new(2025).expect("a season"),
        source: SourceRef::new("milesplit_roster", None),
    }))
    .with_gender(Gender::Girls)
    .with_profile_url(Some(
        "https://wi.milesplit.com/athletes/14399169".to_string(),
    ));

    assert_eq!(observation.id, "milesplit_athlete:14399169");
    assert_eq!(
        observation.observed_grade.as_ref().map(ObservedGrade::grad_year),
        Some(GradYear::new(2028).expect("a class")),
        "the class is what the source's own grade observation implies, not what this program decided"
    );
    assert_eq!(observation.gender, Gender::Girls);
    assert_eq!(
        SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            "14399169",
            "results:invite-2026",
            "J. Smith",
            "2026-09-22",
        )
        .id,
        observation.id,
        "one provider object is one observation row, whatever page it was read from"
    );
    assert_eq!(
        SourceAthleteObservation::new(
            SourceNamespace::MilesplitAthlete,
            "14399170",
            "roster:madison-west:2025-26",
            "Jordan Smith",
            "2026-09-22",
        )
        .gender,
        Gender::Unknown,
        "a source that published no gender states none rather than guessing one"
    );
}

#[test]
fn a_school_observation_keeps_the_sources_own_words_and_the_provider_ids() {
    let observation = SourceSchoolObservation::new(
        SourceNamespace::MilesplitSchool,
        "wi-madison-west",
        "schools/wi-madison-west",
        "Madison West High School",
        "2026-09-22",
    )
    .with_city(Some("Madison".to_string()))
    .with_state(Some(crate::UsJurisdiction::Wisconsin))
    .with_association_id(Some("WIAA-1234".to_string()))
    .with_district(Some("Big Eight".to_string()))
    .with_urls(
        Some("https://west.madison.k12.wi.us".to_string()),
        Some("998877".to_string()),
    );

    assert_eq!(observation.id, "milesplit_school:wi-madison-west");
    assert_eq!(observation.state, Some(crate::UsJurisdiction::Wisconsin));
    assert_eq!(observation.association_id.as_deref(), Some("WIAA-1234"));
    assert_eq!(observation.athletics_net_team_id.as_deref(), Some("998877"));
    assert_eq!(
        observation.observed_name, "Madison West High School",
        "the name is kept as the source spelled it: the canonical natural key is derived, not stored"
    );
    assert_eq!(
        SourceSchoolObservation::new(
            SourceNamespace::AssociationSchool {
                association: "wiaa".to_string(),
            },
            "1234",
            "schools/1234",
            "Madison West HS",
            "2026-09-22",
        )
        .id,
        "association_school:wiaa:1234",
        "the same school seen by another provider is another observation, not a rewrite of this one"
    );
}
