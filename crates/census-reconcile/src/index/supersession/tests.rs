use super::*;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, Evidence, Gender, GradYear, SourceIdentity, SourceNamespace,
    SourceRef,
};
use census_domain::UsJurisdiction;
use census_store::Store;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const AT: &str = "2026-10-13";

fn fixture(store: &Store) -> TestResult<Vec<AthleteCandidateId>> {
    let mut members = Vec::new();
    for name in ["School A", "School B", "School C"] {
        let school = CanonicalSchool::new(UsJurisdiction::Wisconsin, name, name, None).0;
        let mut athlete = CanonicalAthlete::new(
            &school.id,
            "Synthetic Runner",
            GradYear::CO2027,
            Gender::Girls,
            SourceIdentity::new(SourceNamespace::MilesplitAthlete, "111"),
        )?;
        athlete.evidence.push(Evidence::parsed(
            SourceRef::new("fixture", Some("https://example.test/results".to_owned())),
            AT,
        ));
        store.append(Table::Schools, &school)?;
        store.append(Table::Athletes, &athlete)?;
        members.push(athlete.id.cast());
    }
    Ok(members)
}

fn subset(
    members: &[AthleteCandidateId],
    positions: &[usize],
) -> TestResult<Vec<AthleteCandidateId>> {
    positions
        .iter()
        .map(|position| {
            members
                .get(*position)
                .cloned()
                .ok_or_else(|| "fixture member absent".into())
        })
        .collect()
}

fn case(store: &Store, members: Vec<AthleteCandidateId>, detail: &str) -> TestResult<ReviewCase> {
    let first = members.first().ok_or("fixture has no members")?;
    let evidence =
        store
            .athlete_identity_index()?
            .case_evidence("Synthetic Runner", detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.as_str(),
        "Synthetic Runner",
        detail,
        evidence,
    );
    case.member_ids = members;
    Ok(case)
}

fn old(store: &Store, members: Vec<AthleteCandidateId>) -> TestResult<ReviewCase> {
    let mut case = case(store, members, "obsolete machine finding")?;
    make_stale(&mut case)?;
    case.state = ReviewState::Retained;
    store.replace(Table::ReviewCases, &case)?;
    Ok(case)
}

fn make_stale(case: &mut ReviewCase) -> TestResult {
    let (binding, digest) = case.id.rsplit_once(':').ok_or("fixture case digest")?;
    let (subject, _) = binding.rsplit_once(":p").ok_or("fixture case policy")?;
    case.id = format!("{subject}:p1:{digest}");
    Ok(())
}

fn verdict(case: &ReviewCase) -> ReviewVerdictRecord {
    ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".to_owned(),
        field: "identity".to_owned(),
        value: "same_person".to_owned(),
        accepted: true,
        confidence: 100,
        rationale: "parsed shared provider object".to_owned(),
        reviewer: "deterministic:shared-provider-object".to_owned(),
        observed_at: AT.to_owned(),
        member_ids: case.member_ids.clone(),
    }
}

fn file_resolved(store: &Store, case: &ReviewCase) -> TestResult {
    let mut batch = store.write_batch();
    batch.replace_many(Table::ReviewCases, std::slice::from_ref(case))?;
    batch.replace_many(Table::IdentityVerdicts, &[verdict(case)])?;
    batch.commit()?;
    Ok(())
}

fn resolved(
    store: &Store,
    members: Vec<AthleteCandidateId>,
    detail: &str,
) -> TestResult<ReviewCase> {
    let mut case = case(store, members, detail)?;
    case.state = ReviewState::Resolved;
    file_resolved(store, &case)?;
    Ok(case)
}

fn planned(store: &Store) -> TestResult<(Vec<ReviewCase>, Vec<SupersessionRecord>)> {
    let snapshot = store.snapshot();
    let stored = crate::index::stored_cases(&snapshot)?;
    Ok(plan(&snapshot, &stored, &[], AT)?)
}

fn preserved(store: &Store, old: &ReviewCase) -> TestResult {
    let (closed, provenance) = planned(store)?;
    if closed.iter().any(|case| case.id == old.id) || !provenance.is_empty() {
        return Err("unproven or incomplete successor closed historical finding".into());
    }
    if !store.scan::<ReviewCase>(Table::ReviewCases)?.contains(old) {
        return Err("plan changed historical evidence".into());
    }
    Ok(())
}

fn athlete_with(
    school: &CanonicalSchool,
    native_id: &str,
    grad_year: GradYear,
) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &school.id,
        "Synthetic Runner",
        grad_year,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, native_id),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new("fixture", Some("https://example.test/results".to_owned())),
        AT,
    ));
    Ok(athlete)
}

fn contradicted_pair(second_native_id: &str, second_year: GradYear) -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let (school_a, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School A", "school a", None);
    let (school_b, _) =
        CanonicalSchool::new(UsJurisdiction::Wisconsin, "School B", "school b", None);
    let first = athlete_with(&school_a, "111", GradYear::CO2027)?;
    let second = athlete_with(&school_b, second_native_id, second_year)?;
    store.append(Table::Schools, &school_a)?;
    store.append(Table::Schools, &school_b)?;
    store.append_many(Table::Athletes, &[first.clone(), second.clone()])?;
    let members = vec![first.id.cast(), second.id.cast()];
    let kept = old(&store, subset(&members, &[0, 1])?)?;
    let candidate = resolved(
        &store,
        subset(&members, &[0, 1])?,
        "contradicted resolution",
    )?;
    check!(eq; crate::index::apply::apply_decisions(&store, AT)?, 0);
    let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    let report = crate::index::derive(&store, "fixture", AT, year)?;
    let mut retained_candidate = candidate.clone();
    retained_candidate.state = ReviewState::Retained;
    check!(eq; report.superseded, 0, "a contradicted cluster must not supersede");
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &kept.id)?, Some(kept.clone()));
    check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &candidate.id)?, Some(retained_candidate));
    check!(store
        .journal_payload("identity_retained_supersession_v1", &kept.id)?
        .is_none());
    check!(store
        .scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?
        .is_empty());
    preserved(&store, &kept)
}

#[test]
fn exact_unique_current_cluster_closes_old_fields_losslessly() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    let old = old(&store, subset(&members, &[1, 0])?)?;
    let successor = resolved(&store, subset(&members, &[0, 1])?, "current resolution")?;
    if crate::index::apply::apply_decisions(&store, AT)? != 1 {
        return Err("positive control not applied".into());
    }
    let (closed, provenance) = planned(&store)?;
    let mut expected = old.clone();
    expected.state = ReviewState::Superseded;
    let record = provenance.first().ok_or("provenance missing")?;
    if !closed.contains(&expected)
        || provenance.len() != 1
        || record.old_id != old.id
        || record.new_id != successor.id
    {
        return Err("historical fields or exact successor were lost".into());
    }
    Ok(())
}

#[test]
fn subset_or_one_member_overlap_never_closes_the_other_cluster() -> TestResult {
    for (old_positions, new_positions) in
        [(&[0, 1, 2][..], &[0, 1][..]), (&[0, 1][..], &[0, 2][..])]
    {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let members = fixture(&store)?;
        let old = old(&store, subset(&members, old_positions)?)?;
        resolved(
            &store,
            subset(&members, new_positions)?,
            "partial resolution",
        )?;
        if crate::index::apply::apply_decisions(&store, AT)? != 1 {
            return Err("partial control not valid".into());
        }
        preserved(&store, &old)?;
    }
    Ok(())
}

#[test]
fn stale_resolved_labels_do_not_prove_a_current_resolution() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    let old = old(&store, subset(&members, &[0, 1])?)?;
    let mut stale = case(&store, subset(&members, &[0, 1])?, "historical resolution")?;
    make_stale(&mut stale)?;
    stale.state = ReviewState::Resolved;
    file_resolved(&store, &stale)?;
    if crate::index::apply::apply_decisions(&store, AT)? != 0 {
        return Err("stale binding applied".into());
    }
    preserved(&store, &old)
}

#[test]
fn current_retained_findings_and_historical_verdicts_are_preserved() -> TestResult {
    for historical_verdict in [false, true] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let members = fixture(&store)?;
        let mut old = case(
            &store,
            subset(&members, &[0, 1])?,
            "standing retained finding",
        )?;
        old.state = ReviewState::Retained;
        if historical_verdict {
            make_stale(&mut old)?;
        }
        store.replace(Table::ReviewCases, &old)?;
        let mut rejected = verdict(&old);
        rejected.accepted = false;
        rejected.kind = "rejected".to_owned();
        if historical_verdict {
            store.replace(Table::IdentityVerdicts, &rejected)?;
        }
        resolved(&store, subset(&members, &[0, 1])?, "new resolution")?;
        let expected = if historical_verdict { 1 } else { 0 };
        if crate::index::apply::apply_decisions(&store, AT)? != expected {
            return Err("standing control failed".into());
        }
        preserved(&store, &old)?;
        if historical_verdict
            && !store
                .scan::<ReviewVerdictRecord>(Table::IdentityVerdicts)?
                .contains(&rejected)
        {
            return Err("historical verdict changed".into());
        }
    }
    Ok(())
}

#[test]
fn ambiguous_exact_successors_are_withheld_in_every_map_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    let old = old(&store, subset(&members, &[0, 1])?)?;
    resolved(
        &store,
        subset(&members, &[0, 1])?,
        "first current resolution",
    )?;
    resolved(
        &store,
        subset(&members, &[1, 0])?,
        "second current resolution",
    )?;
    if crate::index::apply::apply_decisions(&store, AT)? != 2 {
        return Err("both ambiguity controls must apply".into());
    }
    for _ in 0..8 {
        preserved(&store, &old)?;
    }
    Ok(())
}

#[test]
fn unapplied_resolved_cases_are_not_proof() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    let old = old(&store, subset(&members, &[0, 1])?)?;
    resolved(&store, subset(&members, &[0, 1])?, "unapplied resolution")?;
    preserved(&store, &old)
}

#[test]
fn late_historical_case_gets_a_distinct_receipt_then_replay_converges() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    resolved(
        &store,
        subset(&members, &[0, 1])?,
        "already applied current resolution",
    )?;
    let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    crate::index::derive(&store, "fixture", AT, year)?;
    let prior = store.derived_generation();
    let receipts = store.receipt_count()?;
    let old = old(&store, subset(&members, &[0, 1])?)?;
    let report = crate::index::derive(&store, "fixture", AT, year)?;
    let mut closed = old.clone();
    closed.state = ReviewState::Superseded;
    if report.superseded == 0
        || !store
            .scan::<ReviewCase>(Table::ReviewCases)?
            .contains(&closed)
        || store
            .journal_payload("identity_retained_supersession_v1", &old.id)?
            .is_none()
        || store.derived_generation() == prior
        || store.receipt_count()? != receipts + 1
    {
        return Err("late historical case collided with completed receipt".into());
    }
    let pointer = store.derived_generation();
    let receipts = store.receipt_count()?;
    let repeated = crate::index::derive(&store, "fixture", AT, year)?;
    if repeated.superseded != 0
        || store.derived_generation() != pointer
        || store.receipt_count()? != receipts
    {
        return Err("supersession input binding invalidated its own completed generation".into());
    }
    drop(store);
    let store = Store::open(directory.path())?;
    let repeated = crate::index::derive(&store, "fixture", AT, year)?;
    if repeated.superseded != 0
        || store.derived_generation() != pointer
        || store.receipt_count()? != receipts
    {
        return Err("reopened supersession did not replay the same receipt".into());
    }
    Ok(())
}

#[test]
fn supersession_digest_is_independent_of_hash_map_order_and_binds_the_season() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    old(&store, subset(&members, &[0, 1])?)?;
    resolved(&store, subset(&members, &[0, 1])?, "current resolution")?;
    crate::index::apply::apply_decisions(&store, AT)?;
    let snapshot = store.snapshot();
    let cases: Vec<ReviewCase> = snapshot.scan(Table::ReviewCases)?;
    let left: HashMap<_, _> = cases
        .iter()
        .cloned()
        .map(|case| (case.id.clone(), case))
        .collect();
    let right: HashMap<_, _> = cases
        .into_iter()
        .rev()
        .map(|case| (case.id.clone(), case))
        .collect();
    let (_, planned_left) = plan(&snapshot, &left, &[], AT)?;
    let (_, planned_right) = plan(&snapshot, &right, &[], AT)?;
    let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    let next = census_domain::model::SchoolYear::new(2027).ok_or("next fixture season")?;
    let digest = publication_digest(&snapshot, &left, "base", &planned_left, year)?;
    if digest != publication_digest(&snapshot, &right, "base", &planned_right, year)?
        || digest == publication_digest(&snapshot, &left, "base", &planned_left, next)?
    {
        return Err("receipt depends on map order or does not bind output season".into());
    }
    Ok(())
}

#[test]
fn completed_witnesses_require_the_exact_historical_cluster_and_schema() -> TestResult {
    for shape in 0..4 {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let members = fixture(&store)?;
        let mut old = old(&store, subset(&members, &[0, 1])?)?;
        old.state = ReviewState::Superseded;
        store.replace(Table::ReviewCases, &old)?;
        let successor_members = if shape == 3 { &[0, 2][..] } else { &[0, 1][..] };
        let successor = resolved(
            &store,
            subset(&members, successor_members)?,
            "historical successor",
        )?;
        let payload = match shape {
            0 => serde_json::json!([successor.id]),
            1 => serde_json::json!([successor.id, 7]),
            2 => serde_json::json!([successor.id, "unrecognized reason"]),
            _ => serde_json::json!([successor.id, SUPERSESSION_REASON]),
        };
        let mut batch = store.write_batch();
        batch.journal_done("identity_retained_supersession_v1", &old.id, &payload)?;
        batch.commit()?;
        let snapshot = store.snapshot();
        let stored = crate::index::stored_cases(&snapshot)?;
        let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
        if !matches!(
            publication_digest(&snapshot, &stored, "base", &[], year),
            Err(census_store::StoreError::Json { .. }
                | census_store::StoreError::PublicationRefused { .. })
        ) {
            return Err("malformed or cross-cluster completed witness was accepted".into());
        }
    }
    Ok(())
}

#[test]
fn policy_qualified_publication_retains_the_old_unversioned_receipt() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    resolved(&store, subset(&members, &[0, 1])?, "current resolution")?;
    crate::index::apply::apply_decisions(&store, AT)?;
    let base = crate::index::stage_digest(&store.snapshot())?;
    let operation = format!("index-stage:{base}");
    store.write_batch().commit_once(&operation, &base)?;
    let original_receipt = store.receipt(&operation)?.ok_or("old receipt missing")?;
    let old = old(&store, subset(&members, &[0, 1])?)?;
    let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    let report = crate::index::derive(&store, "fixture", AT, year)?;
    if report.superseded == 0
        || store
            .journal_payload("identity_retained_supersession_v1", &old.id)?
            .is_none()
        || store.receipt(&operation)? != Some(original_receipt)
    {
        return Err("new policy collided with or rewrote the historical receipt".into());
    }
    Ok(())
}

#[test]
fn refreshed_current_verdict_closes_once_without_reinterpreting_old_applications() -> TestResult {
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path())?;
    let members = fixture(&store)?;
    let cluster = subset(&members, &[0, 1])?;
    let old = old(&store, cluster.clone())?;
    let successor = resolved(&store, cluster.clone(), "current resolution")?;
    if crate::index::apply::apply_decisions(&store, AT)? != 1 {
        return Err("original accepted application was not persisted".into());
    }
    let prior = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    let prior = prior.first().ok_or("original application absent")?.clone();
    let mut refreshed = verdict(&successor);
    refreshed.rationale = "same accepted identity with refreshed verdict provenance".to_owned();
    refreshed.observed_at = "2026-10-14".to_owned();
    store.replace(Table::IdentityVerdicts, &refreshed)?;
    if crate::index::apply::apply_decisions(&store, "2026-10-14")? != 1 {
        return Err("refreshed accepted application was not persisted".into());
    }
    let expected_id = format!(
        "identity:{}:p{}:{}",
        successor.id,
        census_domain::model::ATHLETE_IDENTITY_POLICY,
        census_domain::model::identity_verdict_digest(&refreshed)?
    );
    let projection = store.athlete_identity_projection()?;
    if projection
        .rejected_applications()
        .get(&census_domain::model::IdentityDecisionIssue::MissingAcceptedVerdict)
        != Some(&1)
        || cluster
            .iter()
            .map(|id| projection.status(id.as_str()))
            .collect::<Result<Vec<_>, _>>()?
            .iter()
            .any(|status| *status != IdentityStatus::Verified)
    {
        return Err("historical rejection contaminated current verified authority".into());
    }
    let applications = store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)?;
    if applications.len() != 2
        || !applications.contains(&prior)
        || expected_id == prior.id
        || !applications.iter().any(|row| row.id == expected_id)
    {
        return Err("current authority or immutable old application was lost".into());
    }
    let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
    let first = crate::index::derive(&store, "fixture", "2026-10-14", year)?;
    let mut expected = old.clone();
    expected.state = ReviewState::Superseded;
    if first.superseded != 1
        || !store
            .scan::<ReviewCase>(Table::ReviewCases)?
            .contains(&expected)
        || store.journal_payload("identity_retained_supersession_v1", &old.id)?
            != Some(serde_json::json!([successor.id, SUPERSESSION_REASON]))
        || store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)? != applications
    {
        return Err("current successor did not close exactly once while preserving history".into());
    }
    let pointer = store.derived_generation();
    let receipts = store.receipt_count()?;
    let repeated = crate::index::derive(&store, "fixture", "2026-10-15", year)?;
    if repeated.superseded != 0
        || store.derived_generation() != pointer
        || store.receipt_count()? != receipts
        || store.scan::<AppliedAthleteIdentity>(Table::AthleteIdentityDecisions)? != applications
    {
        return Err("refreshed identity authority did not converge on repeat".into());
    }
    Ok(())
}

#[test]
fn genuine_cohort_conflict_still_blocks_supersession() -> TestResult {
    contradicted_pair("111", GradYear::new(2028).ok_or("fixture cohort")?)
}

#[test]
fn genuine_provider_conflict_still_blocks_supersession() -> TestResult {
    contradicted_pair("222", GradYear::CO2027)
}

#[test]
fn moved_evidence_between_planning_and_publication_refuses_the_supersession() -> TestResult {
    for moved in [false, true] {
        let directory = tempfile::tempdir()?;
        let store = Store::open(directory.path())?;
        let members = fixture(&store)?;
        let old = old(&store, subset(&members, &[0, 1])?)?;
        let successor = resolved(&store, subset(&members, &[0, 1])?, "current resolution")?;
        check!(eq; crate::index::apply::apply_decisions(&store, AT)?, 1);
        let mut stage = store.stage_derived()?;
        let stored = crate::index::stored_cases(stage.view())?;
        let (closed, provenance) = plan(stage.view(), &stored, &[], AT)?;
        let mut expected = old.clone();
        expected.state = ReviewState::Superseded;
        check!(eq; closed, vec![expected.clone()]);
        let record = provenance.first().ok_or("supersession was not planned")?;
        if provenance.len() != 1 || record.old_id != old.id || record.new_id != successor.id {
            return Err("the exact successor was not planned".into());
        }
        let base = crate::index::stage_digest(stage.view())?;
        let year = census_domain::model::SchoolYear::new(2026).ok_or("fixture season")?;
        let digest = publication_digest(stage.view(), &stored, &base, &provenance, year)?;
        let operation = format!("{}:{digest}", crate::index::STAGE_RECEIPT);
        crate::index::carry_outputs(&mut stage)?;
        stage_findings(&mut stage, &closed, &[], provenance)?;
        let pointer = store.derived_generation();
        if moved {
            let (school, _) =
                CanonicalSchool::new(UsJurisdiction::Wisconsin, "School D", "school d", None);
            let before = store.evidence_generation();
            store.append(Table::Schools, &school)?;
            check!(
                store.evidence_generation() != before,
                "the fixture must move the inputs"
            );
        }
        let outcome = stage.publish(&operation, &digest);
        if !moved {
            let publication = outcome?;
            check!(publication.application.written());
            check!(eq; store.derived_generation(), publication.generation);
            check!(eq; store.journal_payload("identity_retained_supersession_v1", &old.id)?,
                Some(serde_json::json!([successor.id, SUPERSESSION_REASON])));
            check!(store
                .scan::<ReviewCase>(Table::ReviewCases)?
                .contains(&expected));
            check!(store.receipt(&operation)?.is_some());
            continue;
        }
        let detail = match outcome {
            Err(census_store::StoreError::PublicationRefused { detail }) => detail,
            other => {
                return Err(format!("moved evidence must refuse: {other:?}").into());
            }
        };
        check!(
            detail.contains("inputs moved"),
            "the refusal must name the moved inputs: {detail}"
        );
        check!(eq; store.derived_generation(), pointer);
        check!(eq; store.receipt_count()?, 0);
        check!(store.receipt(&operation)?.is_none());
        check!(store
            .journal_payload("identity_retained_supersession_v1", &old.id)?
            .is_none());
        check!(
            store
                .scan::<ReviewCase>(Table::ReviewCases)?
                .iter()
                .all(|case| case.state != ReviewState::Superseded),
            "a refused publication must publish no supersession"
        );
        check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &old.id)?, Some(old.clone()));
        drop(store);
        let store = Store::open(directory.path())?;
        check!(eq; store.derived_generation(), pointer);
        check!(store
            .journal_payload("identity_retained_supersession_v1", &old.id)?
            .is_none());
        check!(eq; store.snapshot().get_merged::<ReviewCase>(Table::ReviewCases, &old.id)?, Some(old.clone()));
    }
    Ok(())
}
