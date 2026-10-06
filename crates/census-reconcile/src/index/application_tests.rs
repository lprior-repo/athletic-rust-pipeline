use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear,
    IdentityStatus, ReviewCase, ReviewState, ReviewVerdictRecord, SourceIdentity, SourceNamespace,
    SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn athlete(school: &CanonicalSchool, native_id: &str) -> CanonicalAthlete {
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, native_id),
    );
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some("https://example.test/results".to_owned())),
        "2026-09-26",
    ));
    athlete
}

#[test]
fn derive_applies_source_binding_once_and_preserves_history_after_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "School", "school", None);
    let mut athlete = athlete(&school, "111");
    let original;
    {
        let store = Store::open(directory.path())?;
        store.append(Table::Schools, &school)?;
        store.append(Table::Athletes, &athlete)?;
        check!(eq; super::derive(&store, "fixture", "2026-09-26")?.identity_applications,
        1);
        check!(eq; store
            .athlete_identity_projection()?
            .status(athlete.id.as_str())?,
        IdentityStatus::Verified);
        original = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
        check!(eq; super::derive(&store, "fixture", "2026-09-27")?.identity_applications,
        0);
        check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?,
        original);
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new("second", Some("https://example.test/second".to_owned())),
            "2026-09-27",
        ));
        store.append(Table::Athletes, &athlete)?;
        check!(eq; store
            .athlete_identity_projection()?
            .status(athlete.id.as_str())?,
        IdentityStatus::Unverified);
        check!(eq; super::derive(&store, "fixture", "2026-09-27")?.identity_applications,
        1);
        store.flush()?;
    }
    let store = Store::open(directory.path())?;
    let history = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; history.len(), 2);
    for record in original {
        check!(history.contains(&record));
    }
    check!(eq; store
        .athlete_identity_projection()?
        .status(athlete.id.as_str())?,
    IdentityStatus::Verified);
    check!(eq; super::derive(&store, "fixture", "2026-09-28")?.identity_applications,
    0);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?,
    history);
    Ok(())
}

#[test]
fn derive_does_not_promote_or_merge_provider_owned_homonyms() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "School", "school", None);
    let first = athlete(&school, "111");
    let second = athlete(&school, "222");
    store.append(Table::Schools, &school)?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    check!(eq; super::derive(&store, "fixture", "2026-09-26")?.identity_applications,
    0);
    let projection = store.athlete_identity_projection()?;
    for athlete in [&first, &second] {
        check!(ne; projection.status(athlete.id.as_str())?,
        IdentityStatus::Verified);
        check!(eq; projection.canonical_id(athlete.id.as_str()),
        athlete.id.as_str());
    }
    let subjects = store.scan::<CanonicalAthlete>(Table::Athletes)?;
    check!(eq; subjects.len(), 2);
    check!(subjects.contains(&first));
    check!(subjects.contains(&second));
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    Ok(())
}

fn resolved_transfer(store: &Store) -> Result<ReviewCase, Box<dyn std::error::Error>> {
    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, "111");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let detail = "One primary provider identity observed across two schools";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = members;
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: detail.to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;
    Ok(case)
}

#[test]
fn reviewed_membership_permutation_preserves_application_history() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let mut case = resolved_transfer(&store)?;
    check!(eq; super::apply::apply_decisions(&store, "2026-09-26")?, 1);
    let mut history = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    for decision in &mut history {
        decision
            .members
            .sort_unstable_by(|left, right| right.subject.cmp(&left.subject));
    }
    store.replace_many(Table::AthleteIdentityDecisions, &history)?;
    case.member_ids.sort_unstable();
    store.replace(Table::ReviewCases, &case)?;
    check!(eq; super::apply::apply_decisions(&store, "2026-09-27")?, 0);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?,
    history);
    let projection = store.athlete_identity_projection()?;
    let canonical = case
        .member_ids
        .iter()
        .min()
        .ok_or("missing fixture members")?;
    for member in &case.member_ids {
        check!(eq; projection.status(member.as_str())?,
        IdentityStatus::Verified);
        check!(eq; projection.canonical_id(member.as_str()), canonical.as_str());
    }
    Ok(())
}
