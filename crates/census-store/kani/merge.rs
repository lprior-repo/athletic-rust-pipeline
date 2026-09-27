
use census_domain::UsJurisdiction;
use census_domain::model::{
    published_email, CanonicalCoach, CanonicalSchool, CoachRole, Evidence, Gender, MailboxKind,
    SourceIdentity, SourceNamespace, SourceRef, Sport,
};
use crate::Entity;

const TEXT_BYTES: usize = 6;

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
    coach.personal_email = any_opt_text();
    coach.phone = any_opt_text();
    coach.source_identities = any_source_identities();
    coach.evidence = any_evidence();
    coach
}

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
}

#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_coach_publish_routes_arbitrary_address() {
    let mut coach = any_coach();
    coach.personal_email = None;
    let Some(address) = coach.professional_email.clone() else {
        return;
    };

    coach.publish();

    match published_email(&address) {
        Some((expected, MailboxKind::Professional)) => {
            assert_eq!(coach.professional_email.as_deref(), Some(expected.as_str()));
            assert_eq!(coach.personal_email, None);
        }
        Some((expected, MailboxKind::Personal)) => {
            assert_eq!(coach.personal_email.as_deref(), Some(expected.as_str()));
            assert_eq!(coach.professional_email, None);
        }
        None => {
            assert_eq!(coach.professional_email, None);
            assert_eq!(coach.personal_email, None);
        }
    }
}

#[kani::proof]
#[kani::unwind(64)]
#[kani::stub(core::arch::x86_64::__cpuid_count, cpuid_without_features)]
fn check_coach_publish_routes_known_addresses() {
    let school_id =
        CanonicalSchool::mint(UsJurisdiction::Wisconsin, "Test High School", "test high school");
    let mut coach = CanonicalCoach::new(
        &school_id,
        "Coach",
        None,
        Gender::Boys,
        CoachRole::HeadCoach,
    );
    coach.professional_email = Some("coach@gmail.com".to_string());
    coach.personal_email = Some("coach@school.edu".to_string());

    coach.publish();

    assert_eq!(coach.personal_email.as_deref(), Some("coach@gmail.com"));
    assert_eq!(coach.professional_email.as_deref(), Some("coach@school.edu"));
}
