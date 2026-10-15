use census_domain::model::{
    AthleteIdentityIndex, AttestationQualification, CanonicalAthlete, Evidence, Gender, GradYear,
    IdentityAttestation, ReviewCase, ReviewState, ReviewVerdictRecord, SchoolId, SourceIdentity,
    SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::public_identity_history::local_adjudication::{
    ClaimTreatmentV1, LocalIdentitySplitV1, RetainedClaimDispositionV1, SupersededMergeV1,
};
use census_domain::public_identity_history::{BindingPartition, HistoryRequest};
use census_store::{PublicIdentityWriterV1, ReviewedIdentityRequestV1, Store, Table};
use sha2::{Digest, Sha256};

use super::fixture::Plan;
use super::{results, TestResult};

pub(super) fn install(store: &Store, writer: &PublicIdentityWriterV1) -> TestResult<Plan> {
    let mut athletes = [
        subject("first", SourceNamespace::MilesplitAthlete, "9401")?,
        subject("second", SourceNamespace::TfrrsAthlete, "9402")?,
        subject("unrelated", SourceNamespace::MilesplitAthlete, "9403")?,
    ];
    store.append_many(Table::Athletes, &athletes)?;
    let candidates = athletes.each_ref().map(|athlete| athlete.id.cast());
    let births = candidates
        .iter()
        .enumerate()
        .map(|(ordinal, candidate)| {
            Ok(writer.establish_source_bound(
                store,
                &format!("manual-crash-birth-{ordinal}"),
                candidate,
                store.snapshot().revision(),
                &writer.head(store)?,
            )?)
        })
        .collect::<TestResult<Vec<_>>>()?;
    let public_ids = births
        .iter()
        .map(|birth| birth.public_id().clone())
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "three synthetic births required")?;
    corroborate(&mut athletes)?;
    store.append_many(Table::Athletes, &athletes)?;
    let (case, verdict) = review(&athletes)?;
    store.replace_many(Table::ReviewCases, std::slice::from_ref(&case))?;
    store.replace_many(Table::IdentityVerdicts, &[verdict])?;
    let [canonical, second, _] = &public_ids;
    let mut members = vec![canonical.clone(), second.clone()];
    members.sort();
    writer.apply_reviewed_identity(
        store,
        "manual-crash-genuine-prior-merge",
        &ReviewedIdentityRequestV1::Merge {
            case_id: case.id.clone(),
            canonical: canonical.clone(),
            members,
        },
        store.snapshot().revision(),
        &writer.head(store)?,
    )?;
    results::install(store, &candidates)?;
    let request = split_request(store, writer, &case, canonical)?;
    check!(eq; request.affected_record_keys, results::expected_keys(&candidates)?);
    Ok(Plan {
        snapshot_sequence: store.snapshot().sequence(),
        identity: store.store_identity()?,
        head: writer.head(store)?,
        candidates,
        public_ids,
        request,
    })
}

fn subject(label: &str, namespace: SourceNamespace, id: &str) -> TestResult<CanonicalAthlete> {
    let school = SchoolId::mint("sch", &["manual-crash-synthetic", label]);
    let mut athlete = CanonicalAthlete::new(
        &school,
        format!("Synthetic Manual Crash Runner {label}"),
        GradYear::CO2027,
        Gender::Girls,
        SourceIdentity::new(namespace, id),
    )?;
    athlete.evidence.push(Evidence::parsed(
        SourceRef::new(
            "manual-crash-fixture",
            Some(format!("https://synthetic.test/{id}")),
        ),
        "2026-10-15",
    ));
    Ok(athlete)
}

fn corroborate(athletes: &mut [CanonicalAthlete; 3]) -> TestResult {
    let [first, second, _] = athletes;
    let source = second.source.as_ref().ok_or("second source")?.clone();
    first.add_identity(source.clone());
    first
        .identity_attestations
        .push(attestation(&source, "school", "school-producer")?);
    second
        .identity_attestations
        .push(attestation(&source, "timer", "timer-producer")?);
    Ok(())
}

fn attestation(
    subject: &SourceIdentity,
    publisher: &str,
    producer: &str,
) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "synthetic_fixture": true, "publisher": publisher,
        "producer": producer, "published_subject": subject,
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(bytes)),
        acquired_at: "2026-10-15T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: producer.into(),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}

fn review(athletes: &[CanonicalAthlete; 3]) -> TestResult<(ReviewCase, ReviewVerdictRecord)> {
    let mut index = AthleteIdentityIndex::default();
    for athlete in athletes {
        index.observe(athlete)?;
    }
    let members = athletes
        .iter()
        .take(2)
        .map(|row| row.id.cast())
        .collect::<Vec<_>>();
    let subject = "Synthetic manually corrected merge";
    let detail = "Independent synthetic captures previously supported same_person";
    let evidence = index.case_evidence(subject, detail, &members)?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        members.first().ok_or("first review member")?.as_str(),
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

fn split_request(
    store: &Store,
    writer: &PublicIdentityWriterV1,
    case: &ReviewCase,
    canonical: &census_domain::public_identity_history::PublicAthleteId,
) -> TestResult<LocalIdentitySplitV1> {
    let inventory = writer.inspect_local_adjudication(store, canonical)?;
    let accepted = store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("accepted snapshot")?;
    let partitions = accepted
        .history_records()
        .iter()
        .take(2)
        .map(|record| {
            let HistoryRequest::Establish { binding, .. } = &record.request else {
                return Err("expected synthetic source birth".into());
            };
            Ok(BindingPartition {
                representative: binding.candidate.clone(),
                binding_ids: vec![binding.id.clone()],
            })
        })
        .collect::<TestResult<Vec<_>>>()?;
    let dispositions = inventory
        .claims()
        .iter()
        .map(|claim| RetainedClaimDispositionV1 {
            claim: claim.clone(),
            treatment: if claim.claim_path.starts_with("/source_links/")
                || claim.claim_path.starts_with("/identity_attestations/")
            {
                ClaimTreatmentV1::RejectIdentityCorroboration
            } else {
                ClaimTreatmentV1::RetainAsContext
            },
            rationale:
                "Synthetic local review separates the retained athletes without deleting facts"
                    .into(),
        })
        .collect();
    Ok(LocalIdentitySplitV1 {
        case_id: case.id.clone(),
        canonical: canonical.clone(),
        supersedes: vec![SupersededMergeV1 {
            sequence: accepted.head().sequence,
            digest: accepted.head().digest.clone(),
        }],
        partitions,
        unresolved_bindings: Vec::new(),
        affected_record_keys: inventory.affected_record_keys().to_vec(),
        dispositions,
        rationale: "Synthetic operator approval reverses the retained identity interpretation"
            .into(),
    })
}
