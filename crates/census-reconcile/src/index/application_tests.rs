mod lineage_derive_tests;

use census_domain::model::{
    AppliedAthleteIdentity, AppliedIdentityKind, CanonicalAthlete, CanonicalSchool, Evidence,
    Gender, GradYear, IdentityApplication, IdentityProjectionBuilder, IdentityStatus, ReviewCase,
    ReviewState, ReviewVerdictRecord, SourceIdentity, SourceNamespace, SourceRef,
    ATHLETE_IDENTITY_FAMILY,
};
use census_domain::UsJurisdiction;
use census_store::{Store, Table};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn season() -> Result<census_domain::model::SchoolYear, Box<dyn std::error::Error>> {
    census_domain::model::SchoolYear::new(2026).ok_or_else(|| "invalid fixture season".into())
}

fn athlete(school: &CanonicalSchool, native_id: &str) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, native_id),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some("https://example.test/results".to_owned())),
        "2026-09-26",
    ));
    Ok(athlete)
}

#[test]
fn derive_applies_source_binding_once_and_preserves_history_after_reopen() -> TestResult {
    let directory = tempfile::tempdir()?;
    let (school, _) = CanonicalSchool::new(UsJurisdiction::Wisconsin, "School", "school", None);
    let mut athlete = athlete(&school, "111")?;
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
    let first = athlete(&school, "111")?;
    let second = athlete(&school, "222")?;
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
    let first = athlete(&school_a, "111")?;
    let second = athlete(&school_b, second_native_id)?;
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

fn publish_checked_retained_supersession(
    store: &Store,
    old: &ReviewCase,
    successor: &ReviewCase,
    observed_at: &str,
) -> TestResult {
    let snapshot = store.snapshot();
    let cases: Vec<ReviewCase> = snapshot.scan(Table::ReviewCases)?;
    let verdicts: Vec<ReviewVerdictRecord> = snapshot.scan(Table::IdentityVerdicts)?;
    let decisions: Vec<AppliedAthleteIdentity> = snapshot.scan(Table::AthleteIdentityDecisions)?;
    check!(eq; snapshot.get_merged::<ReviewCase>(Table::ReviewCases, &old.id)?, Some(old.clone()));
    check!(eq; old.state, ReviewState::Retained);
    check!(store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .is_none());
    let old_members: std::collections::BTreeSet<_> = old.member_ids.iter().collect();
    let new_members: std::collections::BTreeSet<_> = successor.member_ids.iter().collect();
    check!(!old_members.is_empty());
    check!(eq; old_members.len(), old.member_ids.len());
    check!(eq; new_members.len(), successor.member_ids.len());
    check!(eq; old_members, new_members);
    let mut builder =
        IdentityProjectionBuilder::new(snapshot.athlete_identity_index()?, &cases, &verdicts)?;
    let (_, application) = builder
        .reviewed_applications(observed_at)
        .find(|(case, _)| case.id == successor.id)
        .ok_or("current successor did not produce an application")?;
    let IdentityApplication::Accepted(expected) = application? else {
        return Err("current successor did not have accepted authority".into());
    };
    check!(eq; expected.record().kind, AppliedIdentityKind::SamePerson);
    check!(eq; expected.record().case_id.as_deref(), Some(successor.id.as_str()));
    check!(decisions.contains(expected.record()));
    for decision in &decisions {
        let issue = builder.consider(decision)?;
        if decision.id == expected.record().id {
            check!(eq; issue, None);
        }
    }
    let projection = builder.finish()?;
    for member in &old.member_ids {
        check!(eq; projection.status(member.as_str())?, IdentityStatus::Verified);
    }
    let prior = store.derived_generation();
    let receipts = store.receipt_count()?;
    let report = super::derive(store, "fixture", observed_at, season()?)?;
    check!(eq; report.superseded, 1);
    check!(eq; report.identity_applications, 0);
    check!(store.derived_generation() != prior);
    check!(eq; store.receipt_count()?, receipts.checked_add(1).ok_or("receipt count overflow")?);
    let mut closed = old.clone();
    closed.state = ReviewState::Superseded;
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &old.id)?, Some(closed));
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, decisions);
    check!(eq; store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?, verdicts);
    let recorded = store
        .journal_payload("identity_retained_supersession_v1", &old.id)?
        .ok_or("published supersession witness missing")?;
    let pair: (String, String) = serde_json::from_value(recorded)?;
    check!(eq; pair, (successor.id.clone(),
        "obsolete machine-generated identity finding replaced by the current-policy resolution for the same cluster".to_owned()));
    Ok(())
}

fn seed_retained_supersession_case(store: &Store, detail: &str) -> TestResult<ReviewCase> {
    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete(&school_a, "111")?;
    let second = athlete(&school_b, "111")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store
        .athlete_identity_index()?
        .case_evidence(subject, detail, &members)?;
    Ok(ReviewCase {
        id: format!(
            "{}:{}:p1:{}",
            ATHLETE_IDENTITY_FAMILY,
            first.id,
            evidence.digest()
        ),
        family: ATHLETE_IDENTITY_FAMILY.to_owned(),
        subject_id: first.id.to_string(),
        subject: subject.to_owned(),
        detail: detail.to_owned(),
        state: ReviewState::Retained,
        member_ids: members,
    })
}

fn resolved_supersession_case(
    store: &Store,
    retained: &ReviewCase,
    detail: &str,
    rationale: &str,
) -> TestResult<(ReviewCase, ReviewVerdictRecord)> {
    let evidence = store.athlete_identity_index()?.case_evidence(
        &retained.subject,
        detail,
        &retained.member_ids,
    )?;
    let mut resolved = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        &retained.subject_id,
        &retained.subject,
        detail,
        evidence,
    );
    resolved.member_ids = retained.member_ids.clone();
    resolved.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: resolved.id.clone(),
        case_id: resolved.id.clone(),
        subject_id: resolved.subject_id.clone(),
        family: resolved.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: rationale.to_owned(),
        reviewer: "deterministic fixture".to_owned(),
        observed_at: "2026-09-26".to_owned(),
        member_ids: resolved.member_ids.clone(),
    };
    Ok((resolved, verdict))
}

#[test]
fn supersede_obsolete_retained_identity_case_when_new_policy_resolves_cluster() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let old_case = seed_retained_supersession_case(&store, "old policy detail")?;
    store.replace(Table::ReviewCases, &old_case)?;
    let (new_case, verdict) = resolved_supersession_case(
        &store,
        &old_case,
        "new policy detail",
        "new policy resolves this cluster",
    )?;
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;

    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 1, "new resolved case should be applied");
    publish_checked_retained_supersession(&store, &old_case, &new_case, "2026-09-26")?;

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

    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; decisions.len(), 1);
    let decision = decisions.first().ok_or("missing decision")?;
    check!(eq; decision.kind, AppliedIdentityKind::SamePerson);
    check!(eq; decision.case_id.as_deref(), Some(new_case.id.as_str()));

    let projection = store.athlete_identity_projection()?;
    for member in &old_case.member_ids {
        check!(eq; projection.status(member.as_str())?, IdentityStatus::Verified);
    }

    Ok(())
}

#[test]
fn supersession_is_idempotent_across_repeated_apply() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let old_case = seed_retained_supersession_case(&store, "old policy detail")?;
    store.replace(Table::ReviewCases, &old_case)?;
    let (new_case, verdict) = resolved_supersession_case(
        &store,
        &old_case,
        "new policy detail",
        "new policy resolves this cluster",
    )?;
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;
    let applied1 = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied1, 1);
    let applied2 = super::apply::apply_decisions(&store, "2026-09-27")?;
    check!(eq; applied2, 0, "second apply should be idempotent");
    let decisions = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    check!(eq; decisions.len(), 1);
    publish_checked_retained_supersession(&store, &old_case, &new_case, "2026-09-26")?;
    let pointer = store.derived_generation();
    let receipts = store.receipt_count()?;
    let repeated = super::derive(&store, "fixture", "2026-09-27", season()?)?;
    check!(eq; repeated.superseded, 0);
    check!(eq; store.derived_generation(), pointer);
    check!(eq; store.receipt_count()?, receipts);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, decisions);
    drop(store);
    let store = Store::open(directory.path())?;
    let repeated = super::derive(&store, "fixture", "2026-09-28", season()?)?;
    check!(eq; repeated.superseded, 0);
    check!(eq; store.derived_generation(), pointer);
    check!(eq; store.receipt_count()?, receipts);
    check!(eq; store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?, decisions);
    Ok(())
}

fn supersession_case_in_order(reversed: bool) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let retained_case = seed_retained_supersession_case(&store, "detail")?;
    let (resolved_case, verdict) = resolved_supersession_case(
        &store,
        &retained_case,
        "new detail",
        "resolves cluster",
    )?;

    if reversed {
        store.replace(Table::IdentityVerdicts, &verdict)?;
        store.replace(Table::ReviewCases, &resolved_case)?;
        store.replace(Table::ReviewCases, &retained_case)?;
    } else {
        store.replace(Table::ReviewCases, &retained_case)?;
        store.replace(Table::ReviewCases, &resolved_case)?;
    }
    if !reversed {
        store.replace(Table::IdentityVerdicts, &verdict)?;
    }
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

    publish_checked_retained_supersession(&store, &retained_case, &resolved_case, "2026-09-26")?;
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

    let mut first = athlete(&school_a, "111")?;
    first.gender = Gender::Boys;
    let mut second = athlete(&school_b, "111")?;
    second.gender = Gender::Girls;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store.athlete_identity_index()?.case_evidence(
        subject,
        "gender conflict detail",
        &members,
    )?;
    let mut retained_case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        "gender conflict detail",
        evidence,
    );
    retained_case.member_ids = members.clone();
    retained_case.state = ReviewState::Retained;
    let (binding, digest) = retained_case
        .id
        .rsplit_once(':')
        .ok_or("case digest missing")?;
    let (subject_binding, _) = binding.rsplit_once(":p").ok_or("case policy missing")?;
    retained_case.id = format!("{subject_binding}:p1:{digest}");
    store.replace(Table::ReviewCases, &retained_case)?;

    let new_evidence = store.athlete_identity_index()?.case_evidence(
        subject,
        "gender conflict detail",
        &members,
    )?;
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
    check!(retained_case.id != resolved_case.id);
    store.replace(Table::ReviewCases, &resolved_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;
    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 0, "gender conflict should block application");
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

    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    let report = super::derive(&store, "fixture", "2026-09-26", season()?)?;
    check!(eq; report.superseded, 0);
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &retained_case.id)?, Some(retained_case.clone()));
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &resolved_case.id)?.ok_or("resolved case missing")?.state, ReviewState::Retained);
    check!(eq; store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?, vec![verdict]);
    check!(store
        .journal_payload("identity_retained_supersession_v1", &retained_case.id)?
        .is_none());
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    let projection = store.athlete_identity_projection()?;
    for member in &members {
        check!(projection.status(member.as_str())? != IdentityStatus::Verified);
        check!(eq; projection.canonical_id(member.as_str()), member.as_str());
    }
    Ok(())
}

#[test]
fn supersession_records_durable_transition_provenance() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;

    let old_case = seed_retained_supersession_case(&store, "old policy detail")?;
    store.replace(Table::ReviewCases, &old_case)?;
    let (new_case, verdict) = resolved_supersession_case(
        &store,
        &old_case,
        "new detail",
        "new policy",
    )?;
    store.replace(Table::ReviewCases, &new_case)?;
    store.replace(Table::IdentityVerdicts, &verdict)?;

    check!(eq; super::apply::apply_decisions(&store, "2026-09-26")?, 1);
    publish_checked_retained_supersession(&store, &old_case, &new_case, "2026-09-26")?;
    drop(store);
    let store = Store::open(directory.path())?;

    let recorded = store
        .journal_payload("identity_retained_supersession_v1", &old_case.id)?
        .ok_or("supersession provenance should be journaled")?;
    let pair: (String, String) = serde_json::from_value(recorded)?;
    check!(eq; pair, (new_case.id.clone(),
        "obsolete machine-generated identity finding replaced by the current-policy resolution for the same cluster".to_owned()));
    let old_state = store
        .snapshot()
        .get_merged::<ReviewCase>(Table::ReviewCases, &old_case.id)?
        .ok_or("published closure missing after reopen")?;
    check!(eq; old_state.state, ReviewState::Superseded);

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
    let first = athlete(&school_a, "111")?;
    let second = athlete(&school_b, "111")?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;

    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Synthetic Runner";
    let evidence = store.athlete_identity_index()?.case_evidence(
        subject,
        "operator retained detail",
        &members,
    )?;
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

    let new_evidence =
        store
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
    let applied = super::apply::apply_decisions(&store, "2026-09-26")?;
    check!(eq; applied, 0, "operator retained case should block application");
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

    let verdicts = store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?;
    let report = super::derive(&store, "fixture", "2026-09-26", season()?)?;
    check!(eq; report.superseded, 0);
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &operator_case.id)?, Some(operator_case.clone()));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &operator_case.id)?
        .is_none());
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    check!(eq; store.scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?, verdicts);
    Ok(())
}
