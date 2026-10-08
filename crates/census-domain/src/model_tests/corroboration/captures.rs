use super::*;
use sha2::{Digest, Sha256};

pub(super) fn captured_claim(
    subject: &SourceIdentity,
    publisher: &str,
    producer: &str,
) -> Result<IdentityAttestation, Box<dyn std::error::Error>> {
    let body = serde_json::to_vec(&serde_json::json!({
        "publisher": publisher, "producer": producer,
        "published_subject": subject, "locator": "published_subject",
    }))?;
    let claim = IdentityAttestation {
        subject: subject.clone(),
        source: SourceRef::new(publisher, Some(format!("https://{publisher}.test/capture"))),
        capture_sha256: format!("{:x}", Sha256::digest(&body)),
        acquired_at: "2026-10-07T12:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: producer.into(),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    };
    claim.validate()?;
    Ok(claim)
}

pub(super) fn independent_pair() -> Result<[CanonicalAthlete; 2], Box<dyn std::error::Error>> {
    let key = tfrrs("999");
    let mut linked = observed("School A", milesplit("111"), std::slice::from_ref(&key));
    let mut owner = observed("School B", key.clone(), &[]);
    linked.identity_attestations.push(captured_claim(
        &key,
        "official-roster-fixture",
        "school-fixture",
    )?);
    owner
        .identity_attestations
        .push(captured_claim(&key, "timer-fixture", "timer-fixture")?);
    Ok([linked, owner])
}

pub(super) fn admits(rows: &[CanonicalAthlete]) -> Result<bool, Box<dyn std::error::Error>> {
    let mut index = AthleteIdentityIndex::default();
    let mut members = Vec::new();
    for row in rows {
        index.observe(row)?;
        members.push(row.id.cast());
    }
    Ok(index.supports_identity(AppliedIdentityKind::SamePerson, &members))
}
