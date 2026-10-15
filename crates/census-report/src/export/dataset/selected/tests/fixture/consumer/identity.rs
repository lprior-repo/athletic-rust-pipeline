use super::{CanonicalAthlete, Fixture, TestResult};
use census_domain::model::{ReviewCase, ReviewState, ReviewVerdictRecord, ATHLETE_IDENTITY_FAMILY};
use census_domain::public_identity_history::{HistoryRequest, PublicResolution};
use census_store::{SourceCandidateMembershipRequestV1, Table};

pub(super) fn attach(
    fixture: &Fixture,
    original: &CanonicalAthlete,
    roster: &CanonicalAthlete,
) -> TestResult {
    let retained = fixture
        .store
        .snapshot()
        .freeze_public_identity_v2()?
        .replay()?;
    let PublicResolution::Canonical(owner) = retained.resolve_candidate(&original.id.cast())?
    else {
        return Err("source membership requires original canonical identity".into());
    };
    let anchor = retained
        .history_records()
        .iter()
        .find_map(|record| match &record.request {
            HistoryRequest::Establish { binding, .. }
                if binding.candidate == original.id.cast() =>
            {
                Some(binding.id.clone())
            }
            _ => None,
        })
        .ok_or("source membership anchor")?;
    let case = review(fixture, original, roster)?;
    let request = SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: owner.public_id.clone(),
        anchor_binding_id: anchor,
        candidate: roster.id.cast(),
    };
    let head = fixture.writer.head(&fixture.store)?;
    let outcome = fixture.writer.extend_source_candidate(
        &fixture.store,
        &format!("synthetic-consumer-roster-membership-{}", roster.id),
        &request,
        fixture.store.snapshot().revision(),
        &head,
    )?;
    check!(eq; outcome.public_id(), &owner.public_id);
    check!(outcome.introduced_public_ids().is_empty());
    check!(eq; outcome.head().sequence, head.sequence.checked_add(1).ok_or("membership sequence")?);
    let replay = fixture
        .store
        .snapshot()
        .freeze_public_identity_v2()?
        .replay()?;
    check!(eq; replay.public_ids().count(), retained.public_ids().count());
    let historical = replay
        .history_records()
        .get(..retained.history_records().len())
        .ok_or("prior history")?;
    check!(eq; serde_json::to_vec(historical)?, serde_json::to_vec(retained.history_records())?);
    check!(eq; replay.resolve_candidate(&original.id.cast())?, replay.resolve_candidate(&roster.id.cast())?);
    Ok(())
}

fn review(
    fixture: &Fixture,
    first: &CanonicalAthlete,
    second: &CanonicalAthlete,
) -> TestResult<ReviewCase> {
    let members = vec![first.id.cast(), second.id.cast()];
    let subject = format!("Synthetic captured consumer roster identity {}", second.id);
    let detail =
        "Retained synthetic provider documents identify the same person and current roster";
    let index = fixture.store.snapshot().athlete_identity_index()?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        &subject,
        detail,
        index.case_evidence(&subject, detail, &members)?,
    );
    case.member_ids = members;
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
        reviewer: "synthetic consumer fixture".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    };
    let mut cases: Vec<ReviewCase> = fixture.store.snapshot().scan(Table::ReviewCases)?;
    let mut verdicts: Vec<ReviewVerdictRecord> =
        fixture.store.snapshot().scan(Table::IdentityVerdicts)?;
    cases.push(case.clone());
    verdicts.push(verdict);
    fixture.store.replace_many(Table::ReviewCases, &cases)?;
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &verdicts)?;
    Ok(case)
}
