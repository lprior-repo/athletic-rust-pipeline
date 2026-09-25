//! Property tests for the read-time merge algebra (`Entity::merge` in `store/entities.rs`).
//!
//! Laws pinned here, each one read off the merge bodies they constrain:
//!
//! * **Idempotency** — absorbing a second identical observation changes nothing.
//! * **Union commutativity** — `source_identities`, `evidence`, `aliases`, `known_names`, `sports`,
//!   `source_urls` and `source_labels` are sets, so merge order cannot leak into a row.
//! * **Source identity reversibility** — those unions are *exact*: the `source_identities` a merge
//!   leaves are the two sides' union and nothing else, `identity_in` answers from the merged row for
//!   every namespace either side named, and a refused merge absorbs no identity while its finding
//!   names both sides' provider objects.
//! * **First-writer-wins** — an option a row already carries is never replaced (`level`, `city`,
//!   `enrollment`, …). Every law is checked in both directions, so it is the *first* writer that
//!   survives rather than one fixed side. The meet `level` is the documented exception: `Unknown`
//!   is a hole the other side fills. A coach's contacts are first-writer-wins *per published field*:
//!   a merge reads each side's raw address through `publish`, so the first writer of a kind keeps
//!   that field and the second fills only the kind it left empty.
//! * **Identity preservation** — merge never rewrites the name a canonical record was minted from
//!   (`school.name`, `school.normalized_name`, `athlete.canonical_name`); variants land in
//!   `aliases` / `known_names` instead.
//! * **Coach contact policy** — `publish` keeps each valid address and routes it by domain through
//!   `census_domain::model::published_email`, placing consumer mailboxes in `personal_email`.
//! * **Athlete cohort rule** — an observation that disagrees with `grad_year` lowers
//!   `identity_confidence` to `LOW`, agreement raises it to `HIGH`, and no observation leaves it
//!   alone.
//!
//! Deterministic by construction: [`law_config`] pins 64 cases on ChaCha with the fixed seed
//! `0x004D_4552_475F_4944`, so a failing case is reproducible from the seed alone. The laws live in
//! [`laws_unions`], [`laws_source_identity`], [`laws_writers`] and [`contact_policy`].

#![forbid(unsafe_code)]

use census_domain::model::{
    published_email, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalSchool, CanonicalTeam, CoachRole, CompetitionLevel, Confidence, EventKind, Evidence,
    Gender, GradYear, Grade, MailboxKind, ObservedGrade, SchoolYear, SourceEventLabel,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
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

// ---------------------------------------------------------------------------
// Deterministic runner configuration
// ---------------------------------------------------------------------------

fn law_config() -> ProptestConfig {
    ProptestConfig {
        cases: 64,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(0x004D_4552_475F_4944),
        ..ProptestConfig::default()
    }
}

// ---------------------------------------------------------------------------
// Strategies: scalars
// ---------------------------------------------------------------------------

/// A lowercase word. The values only need to be printable and distinct.
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

/// An external id in `namespace`, distinct from the identities the base value already carries.
fn peer_identity(namespace: SourceNamespace) -> impl Strategy<Value = SourceIdentity> {
    word(8).prop_map(move |suffix| SourceIdentity::new(namespace.clone(), format!("peer-{suffix}")))
}

fn evidence() -> impl Strategy<Value = Evidence> {
    word(8).prop_map(|id| Evidence::parsed(SourceRef::new(format!("src-{id}"), None), "2026-01-01"))
}

/// A published mailbox, sometimes padded, sometimes not a mailbox at all.
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

// ---------------------------------------------------------------------------
// Strategies: canonical entities, built through their own constructors
// ---------------------------------------------------------------------------

fn school() -> impl Strategy<Value = CanonicalSchool> {
    (state(), word(20), word(20))
        .prop_map(|(state, name, normalized)| CanonicalSchool::new(state, name, normalized).0)
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

/// The fields a coach row publishes for one source's raw address: the address trimmed, in the field
/// its own domain kind names, and nothing at all when the source text is not a mailbox.
fn published_slots(address: &str) -> (Option<String>, Option<String>) {
    match published_email(address) {
        Some((address, MailboxKind::Professional)) => (Some(address), None),
        Some((address, MailboxKind::Personal)) => (None, Some(address)),
        None => (None, None),
    }
}

/// The fields two raw addresses leave behind, the first side winning the field it fills.
fn merged_slots(
    first: &(Option<String>, Option<String>),
    second: &(Option<String>, Option<String>),
) -> (Option<String>, Option<String>) {
    (
        first.0.clone().or_else(|| second.0.clone()),
        first.1.clone().or_else(|| second.1.clone()),
    )
}

/// The same coach, carrying an address exactly as a source published it.
fn coach_with_email(address: String) -> CanonicalCoach {
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "Madison", "madison");
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
            let mut athlete = CanonicalAthlete::new(&school.id, name, grad_year, gender);
            // A row the store wrote carries set-shaped vectors: `Sport` is `Ord`, so sorting and
            // de-duplicating is how a set is held here.
            sports.sort();
            sports.dedup();
            athlete.sports = sports;
            athlete
        })
}

fn meet() -> impl Strategy<Value = CanonicalMeet> {
    (state(), word(20), word(8), level())
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
        .prop_map(|(meet, kind, gender)| CanonicalEvent::new(&meet.id, kind, gender, None, None))
}

/// Set equality for the de-duplicated vectors `union_vec` produces: equal length plus mutual
/// containment is enough, because neither side can repeat an element.
fn same_members<T: PartialEq + std::fmt::Debug>(left: &[T], right: &[T]) -> bool {
    left.len() == right.len() && left.iter().all(|item| right.contains(item))
}
