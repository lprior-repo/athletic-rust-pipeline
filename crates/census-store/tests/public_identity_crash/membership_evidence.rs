use super::TestResult;
use census_domain::model::{
    AthleteIdentityIndex, CanonicalAthlete, Evidence, Gender, GradYear, ReviewCase, ReviewState,
    ReviewVerdictRecord, SchoolId, SourceIdentity, SourceNamespace, SourceRef,
    ATHLETE_IDENTITY_FAMILY,
};
use census_domain::public_identity_history::HistoryRequest;
use census_store::{PublicIdentityWriterV1, SourceCandidateMembershipRequestV1, Store, Table};

pub(crate) fn install(
    store: &Store,
    writer: &PublicIdentityWriterV1,
) -> TestResult<SourceCandidateMembershipRequestV1> {
    let prior = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("prior snapshot")?;
    let first = subject("first")?;
    store.append(Table::Athletes, &first)?;
    let birth = writer.establish_source_bound(
        store,
        "native-membership-birth",
        &first.id.cast(),
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    let second = subject("second")?;
    store.append(Table::Athletes, &second)?;
    let (case, verdict) = review(&[first, second.clone()])?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let snapshot = store.snapshot();
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("birth snapshot")?;
    check!(eq; accepted.history_records().get(..prior.history_records().len()), Some(prior.history_records()));
    let record = accepted
        .history_records()
        .iter()
        .find(|record| {
            record.sequence == birth.head().sequence && record.digest == birth.head().digest
        })
        .ok_or("returned birth record")?;
    let HistoryRequest::Establish { binding, .. } = &record.request else {
        return Err("source birth kind".into());
    };
    Ok(SourceCandidateMembershipRequestV1 {
        case_id: case.id,
        owner: birth.public_id().clone(),
        anchor_binding_id: binding.id.clone(),
        candidate: second.id.cast(),
    })
}

fn subject(school: &str) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &SchoolId::mint("sch", &["native-membership", school]),
        "Synthetic School Transfer Runner",
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "8401"),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "native-membership-fixture",
            Some("https://synthetic.test/8401".into()),
        ),
        "2026-10-15",
    ));
    Ok(athlete)
}

fn review(athletes: &[CanonicalAthlete; 2]) -> TestResult<(ReviewCase, ReviewVerdictRecord)> {
    let mut index = AthleteIdentityIndex::default();
    athletes.iter().try_for_each(|row| index.observe(row))?;
    let members = athletes.iter().map(|row| row.id.cast()).collect::<Vec<_>>();
    let subject = "Synthetic same-provider school transfer";
    let detail = "Retained exact primary identity and compatible physical facts";
    let evidence = index.case_evidence(subject, detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members.first().ok_or("first member")?.as_str(),
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
        kind: "value_proposed".into(),
        field: "identity".into(),
        value: "same_person".into(),
        accepted: true,
        confidence: 100,
        rationale: detail.into(),
        reviewer: "synthetic deterministic fixture".into(),
        observed_at: "2026-10-15".into(),
        member_ids: case.member_ids.clone(),
    };
    Ok((case, verdict))
}
