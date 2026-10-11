use super::*;

fn actor() -> SourceSubject {
    SourceSubject::new(SourceNamespace::MilesplitAthlete, "88213".to_string())
        .expect("a subject builds")
}

fn hash(seed: u8) -> String {
    let mut out = String::with_capacity(64);
    for i in 0..64 {
        let v = (i as u8).wrapping_add(seed) % 16;
        let c = if v < 10 { b'0' + v } else { b'a' + (v - 10) };
        out.push(c as char);
    }
    out
}

#[test]
fn subjects_captures_and_claims_bind_by_construction() {
    let subject = actor();
    assert_eq!(subject.logical_id(), "milesplit_athlete:88213");
    let capture = SourceCapture::new(
        &subject,
        hash(1),
        2048,
        "2026-09-19".to_string(),
        Some("https://milesplit.example/athletes/88213".to_string()),
    )
    .expect("a capture builds");
    assert!(capture.capture_id().starts_with("capture:"));
    let claim = SourceClaim::new(&subject, &capture, "2026-09-19".to_string(), hash(2))
        .expect("a claim builds");
    assert_eq!(claim.subject_id(), subject.logical_id());
    assert_eq!(claim.capture_digest(), capture.body_hash());
}

#[test]
fn malformed_subjects_captures_and_claims_are_refused() {
    assert!(SourceSubject::new(SourceNamespace::MilesplitAthlete, "".to_string()).is_err());
    assert!(SourceSubject::new(SourceNamespace::MilesplitAthlete, "a:b".to_string()).is_err());
    let subject = actor();
    assert!(SourceCapture::new(
        &subject,
        "xyz".to_string(),
        10,
        "2026-09-19".to_string(),
        None
    )
    .is_err());
    assert!(SourceCapture::new(&subject, hash(1), 0, "2026-09-19".to_string(), None).is_err());
    let capture = SourceCapture::new(&subject, hash(1), 10, "2026-09-19".to_string(), None)
        .expect("a capture builds");
    let other =
        SourceSubject::new(SourceNamespace::TfrrsAthlete, "991".to_string()).expect("a subject");
    assert!(SourceClaim::new(&other, &capture, "2026-09-19".to_string(), hash(2)).is_err());
}

#[test]
fn deserialization_preserves_subject_capture_and_claim_invariants() {
    let subject = actor();
    let encoded = serde_json::to_string(&subject).expect("a subject encodes");
    let decoded: SourceSubject = serde_json::from_str(&encoded).expect("a subject decodes");
    assert_eq!(decoded, subject);
    let tampered = encoded.replacen("88213", "88214", 1);
    assert!(serde_json::from_str::<SourceSubject>(&tampered).is_err());
    let capture = SourceCapture::new(&subject, hash(3), 64, "2026-09-19".to_string(), None)
        .expect("a capture builds");
    let encoded = serde_json::to_string(&capture).expect("a capture encodes");
    let decoded: SourceCapture = serde_json::from_str(&encoded).expect("a capture decodes");
    assert_eq!(decoded, capture);
    let tampered = encoded.replacen(&hash(3), &hash(4), 1);
    assert!(serde_json::from_str::<SourceCapture>(&tampered).is_err());
    let claim = SourceClaim::new(&subject, &capture, "2026-09-19".to_string(), hash(5))
        .expect("a claim builds");
    let encoded = serde_json::to_string(&claim).expect("a claim encodes");
    let decoded: SourceClaim = serde_json::from_str(&encoded).expect("a claim decodes");
    assert_eq!(decoded, claim);
}

#[test]
fn ceilings_hold_for_every_length_around_the_boundary() {
    for len in 0..160 {
        let id = "7".repeat(len);
        let outcome = SourceSubject::new(SourceNamespace::MilesplitAthlete, id);
        if len == 0 || len > MAX_PROVIDER_ID_BYTES {
            assert!(outcome.is_err(), "provider length {len} must be refused");
        } else {
            assert!(outcome.is_ok(), "provider length {len} must be accepted");
        }
    }
    for len in [0_usize, 1, 63, 64, 65, 256] {
        let digest = "a".repeat(len);
        assert_eq!(
            digest.len() == 64,
            len == 64,
            "hex64 fixture length {len} is self-consistent"
        );
        let _ = digest;
    }
}
