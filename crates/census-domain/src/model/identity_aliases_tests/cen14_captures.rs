use crate::model::{
    AttestationQualification, IdentityAttestation, SourceIdentity, SourceNamespace, SourceRef,
};
use sha2::{Digest, Sha256};

pub(super) fn claim(publisher: &str) -> Result<IdentityAttestation, Box<dyn std::error::Error>> {
    let subject = SourceIdentity::new(SourceNamespace::TfrrsAthlete, "9");
    let bytes = serde_json::to_vec(&serde_json::json!({
        "fixture_publisher": publisher,
        "fixture_producer": publisher,
        "published_subject": subject,
    }))?;
    Ok(IdentityAttestation {
        subject,
        source: SourceRef::new(
            publisher,
            Some(format!("https://{publisher}.test/document")),
        ),
        capture_sha256: format!("{:x}", Sha256::digest(bytes)),
        acquired_at: "2026-10-07T12:00:00Z".into(),
        source_family: publisher.into(),
        upstream_producer: publisher.into(),
        subject_locator: "published_subject".into(),
        qualification: AttestationQualification::IndependentPublished,
    })
}
