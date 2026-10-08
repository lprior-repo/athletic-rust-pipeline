#![forbid(unsafe_code)]

use census_domain::model::{
    published_email, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel, Confidence, EventIdentity,
    EventKind, EventSpecification, Evidence, Gender, GradYear, Grade, MailboxKind, ObservedGrade,
    SchoolYear, SourceEventLabel, SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use census_domain::UsJurisdiction;
use census_store::Entity;
use proptest::prelude::*;
use proptest::test_runner::{RngAlgorithm, RngSeed};

#[path = "merge_properties/contact_policy.rs"]
mod contact_policy;
#[path = "merge_properties/laws_source_identity.rs"]
mod laws_source_identity;
#[path = "merge_properties/laws_unions.rs"]
mod laws_unions;
#[path = "merge_properties/laws_writers.rs"]
mod laws_writers;

fn law_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x004D_4552_475F_4944),
        ..ProptestConfig::default()
    }
}

fn word(max: usize) -> impl Strategy<Value = String> {
    prop::collection::vec(b'a'..=b'z', 1..=max)
        .prop_map(|bytes| bytes.into_iter().map(char::from).collect())
}

fn state() -> impl Strategy<Value = UsJurisdiction> {
    prop_oneof![
        Just(UsJurisdiction::Wisconsin),
        Just(UsJurisdiction::Ohio),
        Just(UsJurisdiction::Illinois),
        Just(UsJurisdiction::Kansas),
        Just(UsJurisdiction::Iowa),
        Just(UsJurisdiction::Minnesota)
    ]
}

fn sport() -> impl Strategy<Value = Sport> {
    prop_oneof![
        Just(Sport::OutdoorTrack),
        Just(Sport::IndoorTrack),
        Just(Sport::CrossCountry),
    ]
}

fn gender() -> impl Strategy<Value = Gender> {
    prop_oneof![Just(Gender::Boys), Just(Gender::Girls), Just(Gender::Mixed),]
}

fn grade() -> impl Strategy<Value = Grade> {
    (9u8..=12u8).prop_filter_map("a published grade is 9..=12", Grade::new)
}

fn school_year() -> impl Strategy<Value = SchoolYear> {
    (2015i16..=2030).prop_filter_map("a school year starts in 2015..=2030", SchoolYear::new)
}

fn grad_year() -> impl Strategy<Value = GradYear> {
    (2020i16..=2040).prop_filter_map("a graduation year is 2020..=2040", GradYear::new)
}

fn level() -> impl Strategy<Value = CompetitionLevel> {
    prop_oneof![
        Just(CompetitionLevel::Invitational),
        Just(CompetitionLevel::State),
        Just(CompetitionLevel::Unknown),
    ]
}

fn peer_identity(namespace: SourceNamespace) -> impl Strategy<Value = SourceIdentity> {
    word(8).prop_map(move |suffix| SourceIdentity::new(namespace.clone(), format!("peer-{suffix}")))
}

fn evidence() -> impl Strategy<Value = Evidence> {
    word(8).prop_map(|id| Evidence::parsed(SourceRef::new(format!("src-{id}"), None), "2026-01-01"))
}

fn mailbox() -> impl Strategy<Value = String> {
    let domain = prop_oneof![
        Just("school.wi.us"),
        Just("district.k12.mn.us"),
        Just("coach.example.org"),
        Just("gmail.com"),
        Just("yahoo.com"),
    ];
    prop_oneof![
        (word(8), domain.clone()).prop_map(|(local, domain)| format!("{local}@{domain}")),
        (word(8), domain).prop_map(|(local, domain)| format!("  {local}@{domain} ")),
        Just("no-mailbox".to_string()),
        Just("@school.wi.us".to_string()),
    ]
}

fn school() -> impl Strategy<Value = CanonicalSchool> {
    (state(), word(20), word(20))
        .prop_map(|(state, name, normalized)| CanonicalSchool::new(state, name, normalized, None).0)
}

fn team() -> impl Strategy<Value = CanonicalTeam> {
    (
        school(),
        sport(),
        gender(),
        school_year(),
        prop::option::of(word(10)),
    )
        .prop_map(|(school, sport, gender, year, level)| CanonicalTeam {
            id: CanonicalTeam::mint(&school.id, sport, gender, year),
            school: school.id,
            sport,
            gender,
            school_year: year,
            level,
            source_identities: Vec::new(),
            evidence: Vec::new(),
            retained_conflicts: Vec::new(),
        })
}

fn coach() -> impl Strategy<Value = CanonicalCoach> {
    (
        school(),
        word(20),
        prop::option::of(sport()),
        gender(),
        prop_oneof![Just(CoachRole::HeadCoach), Just(CoachRole::AssistantCoach)],
    )
        .prop_map(|(school, name, sport, gender, role)| {
            CanonicalCoach::new(&school.id, name, sport, gender, role)
        })
}

fn published_slots(address: &str) -> (Option<String>, Option<String>) {
    match published_email(address) {
        Some((address, MailboxKind::Professional)) => (Some(address), None),
        Some((address, MailboxKind::Personal)) => (None, Some(address)),
        None => (None, None),
    }
}

fn merged_slots(
    first: &(Option<String>, Option<String>),
    second: &(Option<String>, Option<String>),
) -> (Option<String>, Option<String>) {
    (
        first.0.clone().or_else(|| second.0.clone()),
        first.1.clone().or_else(|| second.1.clone()),
    )
}

fn coach_with_email(address: String) -> CanonicalCoach {
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison", "madison", None);
    let mut coach = CanonicalCoach::new(
        &school.id,
        "Coach Smith",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some(address);
    coach
}

fn athlete() -> impl Strategy<Value = CanonicalAthlete> {
    (
        school(),
        word(20),
        grad_year(),
        gender(),
        prop::collection::vec(sport(), 0..=2),
    )
        .prop_map(|(school, name, grad_year, gender, mut sports)| {
            let source = SourceIdentity::new(
                SourceNamespace::Other("fixture".to_string()),
                format!("athlete-{name}"),
            );
            let mut athlete = CanonicalAthlete::new(&school.id, name, grad_year, gender, source);
            sports.sort();
            sports.dedup();
            athlete.sports = sports;
            athlete
        })
}

fn meet_date() -> impl Strategy<Value = String> {
    prop_oneof![
        (1900i16..=2100).prop_map(|year| format!("{year:04}")),
        (2000i16..=2030, 1u8..=12, 1u8..=28)
            .prop_map(|(year, month, day)| format!("{year:04}-{month:02}-{day:02}")),
    ]
}

fn meet() -> impl Strategy<Value = CanonicalMeet> {
    (state(), word(20), meet_date(), level())
        .prop_map(|(state, name, date, level)| CanonicalMeet::new(Some(state), name, date, level))
}

fn event() -> impl Strategy<Value = CanonicalEvent> {
    (
        meet(),
        prop_oneof![
            Just(EventKind::Track100m),
            Just(EventKind::Track200m),
            Just(EventKind::CrossCountry),
        ],
        gender(),
    )
        .prop_filter_map(
            "a valid qualified event identity",
            |(meet, kind, gender)| {
                CanonicalEvent::new(
                    EventIdentity {
                        meet: &meet.id,
                        kind,
                        gender,
                        division: None,
                        round: None,
                    },
                    EventSpecification::default(),
                )
                .ok()
            },
        )
}

fn same_members<T: PartialEq + std::fmt::Debug>(left: &[T], right: &[T]) -> bool {
    left.len() == right.len() && left.iter().all(|item| right.contains(item))
}
