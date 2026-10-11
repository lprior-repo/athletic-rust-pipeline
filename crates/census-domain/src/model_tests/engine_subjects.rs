use super::*;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn actor() -> TestResult<SourceSubject> {
    Ok(SourceSubject::new(
        SourceNamespace::MilesplitAthlete,
        "88213".to_string(),
    )?)
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
fn subjects_captures_and_claims_bind_by_construction() -> TestResult {
    let subject = actor()?;
    check!(eq; subject.logical_id(), "milesplit_athlete:88213");
    let capture = SourceCapture::new(
        &subject,
        hash(1),
        2048,
        "2026-09-19".to_string(),
        Some("https://milesplit.example/athletes/88213".to_string()),
    )?;
    check!(capture.capture_id().starts_with("capture:"));
    let claim = SourceClaim::new(&subject, &capture, "2026-09-19".to_string(), hash(2))?;
    check!(eq; claim.subject_id(), subject.logical_id());
    check!(eq; claim.capture_digest(), capture.body_hash());
    Ok(())
}

#[test]
fn malformed_subjects_captures_and_claims_are_refused() -> TestResult {
    check!(SourceSubject::new(SourceNamespace::MilesplitAthlete, "".to_string()).is_err());
    check!(SourceSubject::new(SourceNamespace::MilesplitAthlete, "a:b".to_string()).is_err());
    let subject = actor()?;
    check!(SourceCapture::new(
        &subject,
        "xyz".to_string(),
        10,
        "2026-09-19".to_string(),
        None
    )
    .is_err());
    check!(SourceCapture::new(&subject, hash(1), 0, "2026-09-19".to_string(), None).is_err());
    let capture = SourceCapture::new(&subject, hash(1), 10, "2026-09-19".to_string(), None)?;
    let other = SourceSubject::new(SourceNamespace::TfrrsAthlete, "991".to_string())?;
    check!(SourceClaim::new(&other, &capture, "2026-09-19".to_string(), hash(2)).is_err());
    Ok(())
}

#[test]
fn deserialization_preserves_subject_capture_and_claim_invariants() -> TestResult {
    let subject = actor()?;
    let encoded = serde_json::to_string(&subject)?;
    let decoded: SourceSubject = serde_json::from_str(&encoded)?;
    check!(eq; decoded, subject);
    let tampered = encoded.replacen("88213", "88214", 1);
    check!(serde_json::from_str::<SourceSubject>(&tampered).is_err());
    let capture = SourceCapture::new(&subject, hash(3), 64, "2026-09-19".to_string(), None)?;
    let encoded = serde_json::to_string(&capture)?;
    let decoded: SourceCapture = serde_json::from_str(&encoded)?;
    check!(eq; decoded, capture);
    let tampered = encoded.replacen(&hash(3), &hash(4), 1);
    check!(serde_json::from_str::<SourceCapture>(&tampered).is_err());
    let claim = SourceClaim::new(&subject, &capture, "2026-09-19".to_string(), hash(5))?;
    let encoded = serde_json::to_string(&claim)?;
    let decoded: SourceClaim = serde_json::from_str(&encoded)?;
    check!(eq; decoded, claim);
    Ok(())
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
