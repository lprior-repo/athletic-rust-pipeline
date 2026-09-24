//! Kani proof harnesses for `Entity::merge` and `Entity::publish` on the canonical entities.
//!
//! The entities are built from concrete natural keys because minting an id runs SHA-256, and a
//! symbolic input would put the compression function's 64 rounds into the solver. The fields an
//! observation actually varies - names, cities, addresses - are overwritten with symbolic values
//! afterwards, so the properties below still quantify over arbitrary text: `String` arrives as a
//! bounded byte array through `String::from_utf8_lossy`, since `kani::any::<String>()` has no
//! `Arbitrary` impl.

use census_domain::UsJurisdiction;
use census_domain::model::{
    CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, SourceIdentity, SourceNamespace,
    SourceRef, Sport, professional_email,
};
use crate::Entity;

/// Bound on the symbolic text fields.
const TEXT_BYTES: usize = 6;

/// `sha2` picks its backend at runtime through `cpufeatures`, which probes the CPU with
/// `__cpuid_count` - inline asm, and `cargo kani` cannot model it:
///
/// ```text
/// Failed Checks: TerminatorKind::InlineAsm is not currently supported by Kani
///  File: ".../core_arch/src/x86/cpuid.rs", line 75, in std::arch::x86_64::__cpuid_count
/// ```
///
/// Minting an id runs SHA-256, so every harness here needs the probe stubbed to answer "no
/// features": the hash then runs on sha2's pure-Rust soft backend. The entity code under test is
/// untouched; the SHA-NI backend is an acceleration of the same function and stays out of reach.
fn cpuid_without_features(_leaf: u32, _sub_leaf: u32) -> core::arch::x86_64::CpuidResult {
    core::arch::x86_64::CpuidResult {
        eax: 0,
        ebx: 0,
        ecx: 0,
        edx: 0,
    }
}

fn any_text() -> String {
    let bytes: [u8; TEXT_BYTES] = kani::any();
    String::from_utf8_lossy(&bytes).into_owned()
}

fn any_opt_text() -> Option<String> {
    kani::any::<bool>().then(any_text)
}

fn any_aliases() -> Vec<String> {
    let count = kani::any::<u8>() % 3;
    (0..count).map(|i| format!("alias-number-{i}")).collect()
}

fn any_source_identities() -> Vec<SourceIdentity> {
    let count = kani::any::<u8>() % 3;
    (0..count)
        .map(|i| SourceIdentity::new(SourceNamespace::MilesplitSchool, format!("id-{i}")))
        .collect()
}

fn any_evidence() -> Vec<Evidence> {
    let count = kani::any::<u8>() % 3;
    (0..count)
        .map(|i| Evidence::fetched(SourceRef::new("fuzz", None), format!("2026-01-0{i}")))
        .collect()
}

fn any_school() -> CanonicalSchool {
    let (mut school, _id) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "Test High School", "test high school");
    school.name = any_text();
    school.normalized_name = any_text();
    school.city = any_opt_text();
    school.association = any_opt_text();
    school.classification = any_opt_text();
    school.enrollment = kani::any::<bool>().then(kani::any::<u32>);
    school.school_website = any_opt_text();
    school.athletics_website = any_opt_text();
    school.co_op = kani::any();
    school.aliases = any_aliases();
    school.source_identities = any_source_identities();
    school.evidence = any_evidence();
    school
}

fn any_coach() -> CanonicalCoach {
    let school_id = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Test High School", "test high school");
    let mut coach = CanonicalCoach::new(
        &school_id,
        "John Coach",
        Some(Sport::OutdoorTrack),
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    coach.name = any_text();
    coach.professional_email = any_opt_text();
    coach.phone = any_opt_text();
    coach.email_withheld = kani::any();
    coach.source_identities = any_source_identities();
    coach.evidence = any_evidence();
    coach
}

/// Merging an entity with a copy of itself changes nothing, whatever the observation carried.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_school_merge_idempotent() {
    let before = any_school();
    let mut after = before.clone();
    after.merge(before.clone());

    assert!(
        after == before,
        "CanonicalSchool::merge(itself) changed the entity"
    );

    kani::cover!(
        before.name.is_empty() || before.name.len() == TEXT_BYTES,
        "text boundaries (empty-like / full) are reachable"
    );
}

/// Same law for a coach, which also carries the mailbox the collection contract watches.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_coach_merge_idempotent() {
    let before = any_coach();
    let mut after = before.clone();
    after.merge(before.clone());

    assert!(
        after == before,
        "CanonicalCoach::merge(itself) changed the entity"
    );

    kani::cover!(
        before.name.is_empty() || before.name.len() == TEXT_BYTES,
        "text boundaries (empty-like / full) are reachable"
    );
}

/// Publishing twice is publishing once, for an arbitrary address and an arbitrary withheld flag.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_coach_publish_idempotent() {
    let mut coach = any_coach();

    coach.publish();
    let published_once = coach.clone();
    coach.publish();

    assert!(
        coach == published_once,
        "CanonicalCoach::publish is not idempotent"
    );

    kani::cover!(
        coach.professional_email.is_some(),
        "email present after first publish"
    );
    kani::cover!(
        coach.professional_email.is_none(),
        "email dropped after first publish"
    );
}

/// Publish never leaves an address the collection contract rejects behind: either the address it
/// kept is one the contract publishes, or the address was dropped and recorded as withheld.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_coach_publish_no_consumer_mailbox() {
    let mut coach = any_coach();
    coach.professional_email = Some(any_text());
    coach.email_withheld = false;

    coach.publish();

    match &coach.professional_email {
        Some(survived) => assert!(
            professional_email(survived) == Some(survived.clone()),
            "publish kept an address its own contract rejects"
        ),
        None => assert!(
            coach.email_withheld,
            "publish dropped the address without recording it as withheld"
        ),
    }

    kani::cover!(
        coach.professional_email.is_some(),
        "an address survives publish"
    );
    kani::cover!(
        coach.professional_email.is_none(),
        "an address is dropped by publish"
    );
}

/// The withheld count the report reads is the observable consequence of publishing: one for a
/// personal mailbox, none for a school one.
#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_coach_withheld_mailboxes_consistency() {
    let school_id = CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Test High School", "test high school");

    let mut personal = CanonicalCoach::new(
        &school_id,
        "Coach Personal",
        None,
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    personal.professional_email = Some("coach@gmail.com".to_string());
    personal.publish();
    assert_eq!(personal.professional_email, None);
    assert_eq!(
        personal.withheld_mailboxes(),
        1,
        "a dropped mailbox must be counted as withheld"
    );

    let mut professional = CanonicalCoach::new(
        &school_id,
        "Coach Professional",
        None,
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    professional.professional_email = Some("coach@school.edu".to_string());
    professional.publish();
    assert_eq!(
        professional.professional_email.as_deref(),
        Some("coach@school.edu")
    );
    assert_eq!(
        professional.withheld_mailboxes(),
        0,
        "a published mailbox must not be counted as withheld"
    );
}
