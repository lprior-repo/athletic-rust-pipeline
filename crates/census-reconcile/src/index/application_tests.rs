mod lineage_derive_tests;

use census_domain::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, CanonicalAthlete, CanonicalSchool, Evidence,
    Gender, GradYear, IdentityStatus, ReviewCase, ReviewState, ReviewVerdictRecord, SourceIdentity,
    SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn season() -> Result<census_domain::model::SchoolYear, Box<dyn std::error::Error>> {
    census_domain::model::SchoolYear::new(2026).ok_or_else(|| "invalid fixture season".into())
}

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
        check!(eq; super::derive(&store, "fixture", "2026-09-26", season()?)?.identity_applications,
        1);
        check!(eq; store
            .athlete_identity_projection()?
            .status(athlete.id.as_str())?,
        IdentityStatus::Verified);
        original = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
        check!(eq; super::derive(&store, "fixture", "2026-09-27", season()?)?.identity_applications,
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
        check!(eq; super::derive(&store, "fixture", "2026-09-27", season()?)?.identity_applications,
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
    check!(eq; super::derive(&store, "fixture", "2026-09-28", season()?)?.identity_applications,
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
    check!(eq; super::derive(&store, "fixture", "2026-09-26", season()?)?.identity_applications,
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

fn reviewed_case(
    store: &Store,
    second_native_id: &str,
    value: &str,
    detail: &str,
) -> Result<ReviewCase, Box<dyn std::error::Error>> {
    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, second_native_id);
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
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
        value: value.to_owned(),
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

fn resolved_transfer(store: &Store) -> Result<ReviewCase, Box<dyn std::error::Error>> {
    reviewed_case(
        store,
        "111",
        "same_person",
        "One primary provider identity observed across two schools",
    )
}

#[test]
fn an_accepted_different_person_verdict_resolves_its_case_and_persists() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let case = reviewed_case(
        &store,
        "222",
        "different_person",
        "Two primary provider identities observed across two schools",
    )?;
    check!(eq; super::apply::apply_decisions(&store, "2026-09-26")?, 1);
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let decision = decisions.first().ok_or("missing separation decision")?;
    check!(eq; decision.kind, AppliedIdentityKind::DifferentPerson);
    check!(eq; decision.canonical_id, None);
    check!(eq; decision.case_id.as_deref(), Some(case.id.as_str()));
    check!(eq; store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .iter()
        .find(|held| held.id == case.id)
        .map(|held| held.state),
    Some(ReviewState::Resolved));
    check!(eq; super::apply::apply_decisions(&store, "2026-09-27")?, 0);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?,
    decisions);
    let projection = store.athlete_identity_projection()?;
    for member in &case.member_ids {
        check!(eq; projection.status(member.as_str())?,
        IdentityStatus::Verified);
        check!(eq; projection.canonical_id(member.as_str()), member.as_str());
    }
    Ok(())
}

#[test]
fn a_different_person_verdict_cannot_separate_one_provider_object() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let case = reviewed_case(
        &store,
        "111",
        "different_person",
        "One primary provider identity observed across two schools",
    )?;
    check!(eq; super::apply::apply_decisions(&store, "2026-09-26")?, 0);
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    check!(eq; store
        .scan::<ReviewCase>(Table::ReviewCases)?
        .iter()
        .find(|held| held.id == case.id)
        .map(|held| held.state),
    Some(ReviewState::Retained));
    Ok(())
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

#[test]
fn supersede_obsolete_retained_identity_case_when_new_policy_resolves_cluster() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    // Create two athletes sharing the same provider identity
    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, "111");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    // File an old retained case (simulating the prior policy)
    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "old policy detail", &members)?;
    let old_id = format!("{}:{}:p1:{}", ATHLETE_IDENTITY_FAMILY, first.id, evidence.digest());
    let old_case = ReviewCase {
        id: old_id,
        family: ATHLETE_IDENTITY_FAMILY.to_string(),
        subject_id: first.id.to_string(),
        subject: subject.to_string(),
        detail: "old policy detail".to_string(),
        state: ReviewState::Retained,
        member_ids: members.clone(),
    };
    store.replace(Table::ReviewCases, &old_case)?;

    // File a new resolved case (simulating the new policy)
    let new_evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "new policy detail", &members)?;
    let mut new_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "new policy detail",
        new_evidence,
    );
    new_case.member_ids = members.clone();
    new_case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: new_case.id.clone(),
        case_id: new_case.id.clone(),
        subject_id: new_case.subject_id.clone(),
        family: new_case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "new policy resolves this cluster".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: new_case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;

    // Apply decisions - should supersede old retained case and apply new resolved case
    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 1, "new resolved case should be applied");

    // Verify the old retained case was superseded
    let old_case_state = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &old_case.id)?
        .ok_or("old case disappeared")?;
    check!(
        eq;
        old_case_state.state,
        ReviewState::Superseded,
        "old retained case should be superseded"
    );

    // Verify the new resolved case was applied
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; decisions.len(), 1);
    let decision = decisions.first().ok_or("missing decision")?;
    check!(eq; decision.kind, AppliedIdentityKind::SamePerson);
    check!(eq; decision.case_id.as_deref(), Some(new_case.id.as_str()));

    // Verify the projection shows verified status
    let projection = store.athlete_identity_projection()?;
    for member in &members {
        check!(eq; projection.status(member.as_str())?, IdentityStatus::Verified);
    }

    Ok(())
}

#[test]
fn supersession_is_idempotent_across_repeated_apply() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, "111");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "old policy detail", &members)?;
    let old_id = format!("{}:{}:p1:{}", ATHLETE_IDENTITY_FAMILY, first.id, evidence.digest());
    let old_case = ReviewCase {
        id: old_id,
        family: ATHLETE_IDENTITY_FAMILY.to_string(),
        subject_id: first.id.to_string(),
        subject: subject.to_string(),
        detail: "old policy detail".to_string(),
        state: ReviewState::Retained,
        member_ids: members.clone(),
    };
    store.replace(Table::ReviewCases, &old_case)?;

    let new_evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "new policy detail", &members)?;
    let mut new_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "new policy detail",
        new_evidence,
    );
    new_case.member_ids = members.clone();
    new_case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: new_case.id.clone(),
        case_id: new_case.id.clone(),
        subject_id: new_case.subject_id.clone(),
        family: new_case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "new policy resolves this cluster".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: new_case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;

    // First apply
    let applied1 = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied1, 1);

    // Second apply (idempotent)
    let applied2 = super::apply::apply_decisions(&store, "2026-09-27")?;
    check!(eq; applied2, 0, "second apply should be idempotent");

    // Verify only one decision was created
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; decisions.len(), 1);

    Ok(())
}

fn supersession_case_in_order(reversed: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, "111");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "detail", &members)?;
    let old_id = format!("{}:{}:p1:{}", ATHLETE_IDENTITY_FAMILY, first.id, evidence.digest());
    let retained_case = ReviewCase {
        id: old_id,
        family: ATHLETE_IDENTITY_FAMILY.to_string(),
        subject_id: first.id.to_string(),
        subject: subject.to_string(),
        detail: "detail".to_string(),
        state: ReviewState::Retained,
        member_ids: members.clone(),
    };

    let new_evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "new detail", &members)?;
    let mut resolved_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "new detail",
        new_evidence,
    );
    resolved_case.member_ids = members.clone();
    resolved_case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: resolved_case.id.clone(),
        case_id: resolved_case.id.clone(),
        subject_id: resolved_case.subject_id.clone(),
        family: resolved_case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "resolves cluster".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: resolved_case.member_ids.clone(),
    };

    if reversed {
        store.replace(Table::ReviewCases, &resolved_case)?;
        store.replace(Table::ReviewCases, &retained_case)?;
    } else {
        store.replace(Table::ReviewCases, &retained_case)?;
        store.replace(Table::ReviewCases, &resolved_case)?;
    }
    store.replace(Table::IdentityVerdicts, &verdict)?;
    let stored = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &retained_case.id)?
        .ok_or("retained case disappeared")?;
    check!(
        eq;
        stored.state,
        ReviewState::Retained,
        "seeded retained state must survive seeding regardless of order"
    );

    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 1);

    let old_case_state = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &retained_case.id)?
        .ok_or("old case disappeared")?;
    check!(
        eq;
        old_case_state.state,
        ReviewState::Superseded,
        "retained case should be superseded regardless of row order"
    );

    Ok(())
}

#[test]
fn supersession_order_is_independent_of_row_order() -> TestResult {
    supersession_case_in_order(false)
}

#[test]
fn supersession_order_is_independent_with_the_verdict_recorded_first() -> TestResult {
    supersession_case_in_order(true)
}

#[test]
fn genuine_gender_conflict_still_blocks_supersession() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);

    let mut first = athlete(&school_a, "111");
    first.gender = Gender::Boys;
    let mut second = athlete(&school_b, "111");
    second.gender = Gender::Girls;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "gender conflict detail", &members)?;
    let mut retained_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "gender conflict detail",
        evidence,
    );
    retained_case.member_ids = members.clone();
    retained_case.state = ReviewState::Retained;
    store.replace(Table::ReviewCases, &retained_case)?;

    // File a resolved case (but the gender conflict is real)
    let new_evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "gender conflict detail", &members)?;
    let mut resolved_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "gender conflict detail",
        new_evidence,
    );
    resolved_case.member_ids = members.clone();
    resolved_case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: resolved_case.id.clone(),
        case_id: resolved_case.id.clone(),
        subject_id: resolved_case.subject_id.clone(),
        family: resolved_case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "claims to resolve".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: resolved_case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &resolved_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;

    // Apply - the resolved case should be rejected due to gender conflict
    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 0, "gender conflict should block application");

    // The resolved case should be written back as retained
    let resolved_state = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &resolved_case.id)?
        .ok_or("resolved case disappeared")?;
    check!(
        eq;
        resolved_state.state,
        ReviewState::Retained,
        "resolved case with gender conflict should be retained"
    );

    Ok(())
}

#[test]
fn supersession_records_durable_transition_provenance() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, "111");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "old policy detail", &members)?;
    let old_id = format!("{}:{}:p1:{}", ATHLETE_IDENTITY_FAMILY, first.id, evidence.digest());
    let old_case = ReviewCase {
        id: old_id,
        family: ATHLETE_IDENTITY_FAMILY.to_string(),
        subject_id: first.id.to_string(),
        subject: subject.to_string(),
        detail: "old policy detail".to_string(),
        state: ReviewState::Retained,
        member_ids: members.clone(),
    };
    store.replace(Table::ReviewCases, &old_case)?;

    let new_evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "new detail", &members)?;
    let mut new_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "new detail",
        new_evidence,
    );
    new_case.member_ids = members.clone();
    new_case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: new_case.id.clone(),
        case_id: new_case.id.clone(),
        subject_id: new_case.subject_id.clone(),
        family: new_case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "new policy".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: new_case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;

    super::apply::apply_decisions(&store, "2026-09-26")?;

    let recorded = store
        .journal_payload("identity_retained_supersession_v1", &old_case.id)?
        .ok_or("supersession provenance should be journaled")?;
    let inner = recorded.get("payload").cloned().unwrap_or(recorded);
    let pair = inner.as_array().ok_or("provenance payload is not an array")?;
    check!(
        eq;
        pair.first().and_then(|value| value.as_str()),
        Some(new_case.id.as_str()),
        "supersession provenance should name the superseding case"
    );

    Ok(())
}

#[test]
fn operator_retained_case_in_same_cluster_is_not_superseded() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111");
    let second = athlete(&school_b, "111");
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "operator retained detail", &members)?;
    let mut operator_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "operator retained detail",
        evidence,
    );
    operator_case.member_ids = members.clone();
    operator_case.state = ReviewState::Retained;
    store.replace(Table::ReviewCases, &operator_case)?;

    let operator_verdict = ReviewVerdictRecord {
        id: operator_case.id.clone(),
        case_id: operator_case.id.clone(),
        subject_id: operator_case.subject_id.clone(),
        family: operator_case.family.clone(),
        kind: "insufficient_evidence".to_owned(),
        field: "".to_owned(),
        value: "".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "operator decided to retain this case".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-25".to_owned(),
        member_ids: operator_case.member_ids.clone(),
    };
    store.replace(Table::IdentityVerdicts, &operator_verdict)?;

    // File a new resolved case for the same cluster
    let new_evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, "new policy detail", &members)?;
    let mut new_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "new policy detail",
        new_evidence,
    );
    new_case.member_ids = members;
    new_case.state = ReviewState::Resolved;
    let new_verdict = ReviewVerdictRecord {
        id: new_case.id.clone(),
        case_id: new_case.id.clone(),
        subject_id: new_case.subject_id.clone(),
        family: new_case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "new policy resolves this cluster".to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: new_case.member_ids.clone(),
    };
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &new_verdict)?;

    // Apply decisions - the new resolved case should be rejected because the operator case is still retained
    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 0, "operator retained case should block application");

    // Verify the operator case is still retained (not superseded)
    let operator_state = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &operator_case.id)?
        .ok_or("operator case disappeared")?;
    check!(
        eq;
        operator_state.state,
        ReviewState::Retained,
        "operator retained case should not be superseded"
    );

    // Verify the new resolved case was written back as retained
    let new_state = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &new_case.id)?
        .ok_or("new case disappeared")?;
    check!(
        eq;
        new_state.state,
        ReviewState::Retained,
        "new resolved case should be retained when operator case blocks it"
    );

    Ok(())
}
