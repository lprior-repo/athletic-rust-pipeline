use super::test_fixture::{full_inputs, install_inputs};
use super::{Inputs, BUDGET};
use crate::public_identity_tests::support::{fixture, subject, TestResult};
use crate::{
    IdentityCommitOutcomeV1, PublicIdentityWriterV1, ReviewedIdentityRequestV1, Store, Table,
};
use census_domain::model::{
    AthleteCandidateId, AthleteIdentityIndex, AttestationQualification, CanonicalAthlete, Evidence,
    IdentityAttestation, ReviewCase, ReviewState, ReviewVerdictRecord, SourceIdentity,
    SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use sha2::{Digest, Sha256};

pub(in crate::public_identity) struct ReviewedFixture {
    pub directory: tempfile::TempDir,
    pub store: Store,
    pub writer: PublicIdentityWriterV1,
    pub candidates: [AthleteCandidateId; 3],
    pub births: [IdentityCommitOutcomeV1; 3],
    pub request: ReviewedIdentityRequestV1,
    pub capture_bodies: [Vec<u8>; 2],
    pub pending_case: ReviewCase,
}

pub(in crate::public_identity) fn ready_for_merge() -> TestResult<ReviewedFixture> {
    build_fixture(|_, _| Ok(()))
}

pub(in crate::public_identity) fn ready_for_manual() -> TestResult<ReviewedFixture> {
    build_fixture(|store, writer| {
        writer
            .enroll_manual_context_admission_v1(store)
            .map_err(Into::into)
    })
}

fn build_fixture(
    enroll: impl FnOnce(&Store, &PublicIdentityWriterV1) -> TestResult,
) -> TestResult<ReviewedFixture> {
    let (directory, store, writer) = fixture()?;
    enroll(&store, &writer)?;
    let mut inputs = full_inputs()?;
    let original = inputs.athletes.get(1).ok_or("second fixture athlete")?;
    let second = tfrrs_subject(original)?;
    *inputs.athletes.get_mut(1).ok_or("second fixture athlete")? = second;
    let unrelated = subject("reviewed-unrelated", "9204")?;
    inputs.athletes.push(unrelated.clone());
    let first = inputs
        .athletes
        .first()
        .ok_or("first fixture athlete")?
        .clone();
    let second = inputs
        .athletes
        .get(1)
        .ok_or("second fixture athlete")?
        .clone();
    let pending_case = inputs.cases.first().ok_or("pending fixture case")?.clone();
    let candidates = [first.id.cast(), second.id.cast(), unrelated.id.cast()];
    let [first_candidate, second_candidate, unrelated_candidate] = &candidates;
    install_inputs(&store, &inputs)?;
    let first_birth = writer.establish_source_bound(
        &store,
        "reviewed-first-birth",
        first_candidate,
        store.snapshot().revision(),
        &writer.head(&store)?,
    )?;
    let second_birth = writer.establish_source_bound(
        &store,
        "reviewed-second-birth",
        second_candidate,
        store.snapshot().revision(),
        first_birth.head(),
    )?;
    let unrelated_birth = writer.establish_source_bound(
        &store,
        "reviewed-unrelated-birth",
        unrelated_candidate,
        store.snapshot().revision(),
        second_birth.head(),
    )?;
    let capture_bodies = append_independent_claims(&store, first, second)?;
    let members = candidates.iter().take(2).cloned().collect::<Vec<_>>();
    let (case, verdict) = review(&store, &members, "same_person")?;
    inputs.cases.push(case.clone());
    inputs.verdicts.push(verdict);
    store.replace_many(Table::ReviewCases, &inputs.cases)?;
    store.replace_many(Table::IdentityVerdicts, &inputs.verdicts)?;
    let mut public_ids = vec![
        first_birth.public_id().clone(),
        second_birth.public_id().clone(),
    ];
    public_ids.sort();
    let request = ReviewedIdentityRequestV1::Merge {
        case_id: case.id,
        canonical: first_birth.public_id().clone(),
        members: public_ids,
    };
    Ok(ReviewedFixture {
        directory,
        store,
        writer,
        candidates,
        births: [first_birth, second_birth, unrelated_birth],
        request,
        capture_bodies,
        pending_case,
    })
}

fn tfrrs_subject(original: &CanonicalAthlete) -> TestResult<CanonicalAthlete> {
    let mut athlete = CanonicalAthlete::new(
        &original.school,
        &original.canonical_name,
        original.grad_year,
        original.gender,
        SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9202"),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "synthetic timer fixture",
            Some("https://timer-fixture.test/athletes/9202".into()),
        ),
        "2026-10-14",
    ));
    Ok(athlete)
}

fn append_independent_claims(
    store: &Store,
    mut first: CanonicalAthlete,
    mut second: CanonicalAthlete,
) -> TestResult<[Vec<u8>; 2]> {
    let key = second
        .source
        .as_ref()
        .ok_or("second primary source")?
        .clone();
    first.add_identity(key.clone());
    let (first_claim, first_body) =
        synthetic_capture(&key, "synthetic-school-roster", "synthetic-school")?;
    let (second_claim, second_body) =
        synthetic_capture(&key, "synthetic-timing-result", "synthetic-timer")?;
    first.identity_attestations.push(first_claim);
    second.identity_attestations.push(second_claim);
    store.append_many(Table::Athletes, &[first, second])?;
    Ok([first_body, second_body])
}

fn synthetic_capture(
    subject: &SourceIdentity,
    publisher: &str,
    producer: &str,
) -> TestResult<(IdentityAttestation, Vec<u8>)> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "synthetic_fixture": true,
        "publisher": publisher,
        "producer": producer,
        "published_subject": subject,
        "locator": "published_subject",
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(&bytes)),
        acquired_at: "2026-10-14T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: producer.into(),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok((claim, bytes))
}

pub(in crate::public_identity) fn review(
    store: &Store,
    members: &[AthleteCandidateId],
    value: &str,
) -> TestResult<(ReviewCase, ReviewVerdictRecord)> {
    review_inputs(&Inputs::capture(&store.snapshot())?, members, value)
}

pub(in crate::public_identity) fn review_inputs(
    inputs: &Inputs,
    members: &[AthleteCandidateId],
    value: &str,
) -> TestResult<(ReviewCase, ReviewVerdictRecord)> {
    let mut athletes = inputs.athletes.clone();
    crate::read::enrich_retained_observations(&mut athletes, &inputs.observations, BUDGET)?;
    let mut index = AthleteIdentityIndex::default();
    for athlete in &athletes {
        index.observe(athlete)?;
    }
    let subject_id = members.first().ok_or("review member")?;
    let subject = "Synthetic independently published identity review";
    let detail = format!("Retained synthetic documents support {value}");
    let evidence = index.case_evidence(subject, &detail, members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        subject_id.as_str(),
        subject,
        &detail,
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
        value: value.into(),
        accepted: true,
        confidence: 100,
        rationale: detail,
        reviewer: "synthetic deterministic fixture".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    };
    Ok((case, verdict))
}

pub(super) fn binding_id(store: &Store, sequence: u64) -> TestResult<String> {
    let key = crate::public_identity::witness::key(sequence);
    let raw = store.journal.get(key)?.ok_or("source witness")?;
    let held: crate::public_identity::witness::Witness = serde_json::from_slice(&raw)?;
    Ok(format!("source_binding_v1_{}", held.request_digest))
}
