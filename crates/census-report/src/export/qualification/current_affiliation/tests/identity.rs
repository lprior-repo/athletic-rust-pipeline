use super::*;
use census_domain::public_identity_history::{HistoryRequest, PublicResolution};
use census_store::SourceCandidateMembershipRequestV1;

pub(in super::super) fn accept_pair(
    store: &Store,
    writer: &census_store::PublicIdentityWriterV1,
    left: &AthleteId,
    right: &AthleteId,
) -> TestResult<AthleteId> {
    let before = store.snapshot().freeze_public_identity_v1()?;
    let accepted = before.replay()?;
    let PublicResolution::Canonical(owner) = accepted.resolve_candidate(&left.cast())? else {
        return Err("first source must be established before the second roster arrives".into());
    };
    check!(eq; accepted.resolve_candidate(&right.cast())?, PublicResolution::Unknown);
    let binding = accepted
        .history_records()
        .iter()
        .find_map(|record| match &record.request {
            HistoryRequest::Establish { binding, .. } if binding.candidate == left.cast() => {
                Some(binding.id.clone())
            }
            _ => None,
        })
        .ok_or("original source binding")?;
    let (case, verdict) = review(store, &[left.cast(), right.cast()])?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: owner.public_id.clone(),
        anchor_binding_id: binding,
        candidate: right.cast(),
    };
    let snapshot = store.snapshot();
    let head = writer.head(store)?;
    let attached = writer.extend_source_candidate(
        store,
        "attach-current-school",
        &request,
        snapshot.revision(),
        &head,
    )?;
    check!(eq; attached.public_id(), &owner.public_id);
    check!(attached.introduced_public_ids().is_empty());
    let after = store.snapshot().freeze_public_identity_v1()?;
    let replayed = after.replay()?;
    check!(eq; replayed.public_ids().count(), 1);
    check!(eq; replayed.resolve_candidate(&left.cast())?, replayed.resolve_candidate(&right.cast())?);
    let repeated = writer.extend_source_candidate(
        store,
        "attach-current-school",
        &request,
        snapshot.revision(),
        &head,
    )?;
    check!(repeated.replayed());
    check!(eq; repeated.head(), attached.head());
    check!(eq; before.replay()?.resolve_candidate(&right.cast())?, PublicResolution::Unknown);
    Ok(left.clone())
}

fn review(
    store: &Store,
    members: &[AthleteCandidateId],
) -> TestResult<(ReviewCase, ReviewVerdictRecord)> {
    let mut index = AthleteIdentityIndex::default();
    for athlete in store.scan::<CanonicalAthlete>(Table::Athletes)? {
        index.observe(&athlete)?;
    }
    let subject = "Synthetic retained transfer roster identity";
    let detail = "Both captured school rosters publish the same provider person, cohort and gender";
    let evidence = index.case_evidence(subject, detail, members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members.first().ok_or("review member")?.as_str(),
        subject,
        detail,
        evidence,
    );
    case.member_ids = members.to_vec();
    case.state = ReviewState::Resolved;
    let verdict = ReviewVerdictRecord {
        id: case.id.clone(),
        case_id: case.id.clone(),
        subject_id: case.subject_id.clone(),
        family: case.family.clone(),
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: detail.into(),
        reviewer: "synthetic deterministic fixture".into(),
        observed_at: "2026-10-07".into(),
        member_ids: case.member_ids.clone(),
    };
    Ok((case, verdict))
}

pub(in super::super) fn assert_unselected(
    dataset: &ExportDataset,
    affiliations: &VerifiedCurrentAffiliations,
    canonical: &AthleteId,
) -> TestResult {
    let selected = crate::report::Derivation::selected(
        dataset,
        crate::report::Scope::AllSources,
        None,
        &[census_domain::UsJurisdiction::Wisconsin],
        affiliations,
    )?;
    check!(selected.athletes().is_empty());
    check!(eq; selected.affiliation_exclusions().get(canonical), Some(&crate::report::CurrentAffiliationExclusion::Unresolved));
    Ok(())
}
