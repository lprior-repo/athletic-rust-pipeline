use census_domain::model::{
    normalize_name, CanonicalAthlete, CanonicalSchool, Gender, GradYear, Grade, ObservedGrade,
    ReviewCase, ReviewPacket, SchoolId, SchoolYear, SourceIdentity, SourceNamespace, SourceRef,
    ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;

#[path = "athlete_packet_tests/flags.rs"]
mod flags;
#[path = "athlete_packet_tests/group.rs"]
mod group;

fn school() -> SchoolId {
    CanonicalSchool::new(
        UsJurisdiction::Wisconsin,
        "Madison West High School",
        normalize_name("Madison West High School"),
    )
    .0
    .id
}

fn athlete(name: &str, gender: Gender, grad_year: GradYear) -> CanonicalAthlete {
    let source = SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399169");
    CanonicalAthlete::new(&school(), name, grad_year, gender, source)
}

fn observed(athlete: &mut CanonicalAthlete, grade: u8, school_year: i16) {
    athlete.observed_grades.push(ObservedGrade {
        grade: Grade::new(grade).expect("a grade in 9..=12"),
        school_year: SchoolYear::new(school_year)
            .expect("a school year inside the accepted window"),
        source: SourceRef::new("milesplit_roster", None),
    });
}

fn known_as(
    athlete: &mut CanonicalAthlete,
    namespace: SourceNamespace,
    id: &str,
    url: Option<&str>,
) {
    let identity = SourceIdentity::new(namespace, id);
    athlete.add_identity(match url {
        Some(url) => identity.with_url(url),
        None => identity,
    });
}

fn rows() -> (CanonicalAthlete, CanonicalAthlete, CanonicalAthlete) {
    let mut boys = athlete("Jordan Smith", Gender::Boys, GradYear::CO2027);
    observed(&mut boys, 11, 2025);
    known_as(
        &mut boys,
        SourceNamespace::MilesplitAthlete,
        "14399169",
        Some("https://wi.milesplit.com/athletes/14399169/jordan-smith"),
    );
    known_as(&mut boys, SourceNamespace::TfrrsAthlete, "77", None);

    let mut girls = athlete("Jordan Smith", Gender::Girls, GradYear::CO2027);
    observed(&mut girls, 10, 2025);
    known_as(
        &mut girls,
        SourceNamespace::MilesplitAthlete,
        "14399169",
        Some("https://wi.milesplit.com/athletes/14399169/jordan-smith"),
    );
    known_as(
        &mut girls,
        SourceNamespace::AthleticNet {
            kind: "athlete".to_string(),
        },
        "998877",
        Some("https://www.athletic.net/athlete/998877/track-and-field"),
    );

    let mut other = athlete("Sam Rivers", Gender::Girls, GradYear::CO2027);
    observed(&mut other, 11, 2025);
    (boys, girls, other)
}

fn case_for(row: &CanonicalAthlete) -> ReviewCase {
    ReviewCase::pending(
        ATHLETE_IDENTITY_FAMILY,
        row.id.as_str(),
        "Jordan Smith (Madison West High School)",
        "same school, name and cohort as every id here: the pair",
    )
}

fn stated(packet: &ReviewPacket, source: &str, field: &str) -> Option<String> {
    packet
        .evidence
        .iter()
        .find(|fact| fact.source == source && fact.field == field)
        .map(|fact| fact.value.clone())
}
