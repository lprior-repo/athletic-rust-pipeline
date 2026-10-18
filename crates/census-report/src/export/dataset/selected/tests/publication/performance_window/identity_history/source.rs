use super::{Fixture, IdentityMap, TestResult};
use census_domain::model::{
    AttestationQualification, CanonicalAthlete, IdentityAttestation, ReviewCase, ReviewState,
    ReviewVerdictRecord, SourceIdentity, SourceNamespace, SourceRef, ATHLETE_IDENTITY_FAMILY,
};
use census_domain::public_identity_history::PublicAthleteId;
use census_store::{ReviewedIdentityRequestV1, Table};
use sha2::{Digest, Sha256};

mod roster;

pub(super) fn merge(
    fixture: &Fixture,
) -> TestResult<(String, PublicAthleteId, IdentityMap, String)> {
    let mut original = fixture
        .store
        .snapshot()
        .athletes()?
        .into_iter()
        .find(|row| row.id == fixture.eligible.athlete)
        .ok_or("eligible source athlete")?;
    let source = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "990000117");
    let mut alias = CanonicalAthlete::new(
        &original.school,
        "Controlled Synthetic PR Alias",
        original.grad_year,
        original.gender,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "990000118"),
    )?;
    fixture.store.append(Table::Athletes, &alias)?;
    alias = roster::append(&fixture.store, &alias.id)?;
    let proof = format!(
        "canonical_evidence:{}",
        census_domain::model::serialized_digest(
            alias
                .evidence
                .first()
                .ok_or("retained alias roster evidence")?
        )?
    );
    let birth = fixture.writer.establish_source_bound(
        &fixture.store,
        "controlled-pr-alias-birth",
        &alias.id.cast(),
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    original.add_identity(source.clone());
    alias.add_identity(source.clone());
    original
        .identity_attestations
        .push(claim(&source, "pr-school")?);
    alias
        .identity_attestations
        .push(claim(&source, "pr-timer")?);
    fixture
        .store
        .append_many(Table::Athletes, &[original.clone(), alias.clone()])?;
    let case_id = review(fixture, &original, &alias)?;
    let accepted = fixture
        .store
        .snapshot()
        .accepted_public_identity_snapshot()?
        .ok_or("births")?;
    let canonical = match accepted.resolve_candidate(&original.id.cast())? {
        census_domain::public_identity_history::PublicResolution::Canonical(value) => {
            value.public_id
        }
        _ => return Err("original canonical identity".into()),
    };
    let mut members = vec![canonical.clone(), birth.public_id().clone()];
    members.sort();
    let request = ReviewedIdentityRequestV1::Merge {
        case_id: case_id.clone(),
        canonical: canonical.clone(),
        members,
    };
    let outcome = fixture.writer.apply_reviewed_identity(
        &fixture.store,
        "controlled-pr-merge",
        &request,
        fixture.store.snapshot().revision(),
        &fixture.writer.head(&fixture.store)?,
    )?;
    check!(!outcome.replayed());
    let expected = [original.id, alias.id]
        .into_iter()
        .map(|candidate| (candidate.to_string(), canonical.clone()))
        .collect();
    Ok((case_id, canonical, expected, proof))
}

fn review(
    fixture: &Fixture,
    first: &CanonicalAthlete,
    second: &CanonicalAthlete,
) -> TestResult<String> {
    let members = vec![first.id.cast(), second.id.cast()];
    let subject = "Controlled synthetic eligible PR identity merge";
    let detail = "Synthetic independently published identity control; no production human approval";
    let index = fixture.store.snapshot().athlete_identity_index()?;
    let mut case = ReviewCase::pending_with_evidence(
        ATHLETE_IDENTITY_FAMILY,
        first.id.as_str(),
        subject,
        detail,
        index.case_evidence(subject, detail, &members)?,
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
        reviewer: "controlled synthetic PR regression".into(),
        observed_at: "2026-10-14".into(),
        member_ids: case.member_ids.clone(),
    };
    let mut cases: Vec<ReviewCase> = fixture.store.snapshot().scan(Table::ReviewCases)?;
    let mut verdicts: Vec<ReviewVerdictRecord> =
        fixture.store.snapshot().scan(Table::IdentityVerdicts)?;
    for old in &mut cases {
        old.state = ReviewState::Superseded;
    }
    cases.push(case.clone());
    verdicts.push(verdict);
    fixture.store.replace_many(Table::ReviewCases, &cases)?;
    fixture
        .store
        .replace_many(Table::IdentityVerdicts, &verdicts)?;
    Ok(case.id)
}

fn claim(subject: &SourceIdentity, publisher: &str) -> TestResult<IdentityAttestation> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "controlled_synthetic_fixture": true, "subject": subject, "publisher": publisher,
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(bytes)),
        acquired_at: "2026-10-14T00:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: publisher.into(),
        subject_locator: "subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}
