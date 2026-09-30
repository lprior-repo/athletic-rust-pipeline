use super::validation::validate_claimed_digest;
use super::{compute_contact_proof, verify_contact_proof, ContactProofError};
use census_domain::model::{ContactClaimEvidence, ContactProofField, RawContactRow};

pub(crate) fn make_valid_coach_row() -> RawContactRow {
    RawContactRow {
        school: "TestU".to_string(),
        city: "Cityville".to_string(),
        state: "CA".to_string(),
        sport: "Cross Country".to_string(),
        role: "Head Coach".to_string(),
        coach_name: "Coach Smith".to_string(),
        public_professional_email: "coach@testu.edu".to_string(),
        ad_name: "AD Jones".to_string(),
        ad_email: "ad@testu.edu".to_string(),
        source_urls: vec!["https://example.com/roster".to_string()],
        last_observed: "2025-01-15".to_string(),
    }
}

pub(crate) fn make_valid_claims(row: &RawContactRow, fetched: &str) -> Vec<ContactClaimEvidence> {
    vec![
        ContactClaimEvidence {
            field: ContactProofField::CoachName,
            value: row.coach_name.clone(),
            person: row.coach_name.clone(),
            role: row.role.clone(),
            sport: row.sport.clone(),
            school: row.school.clone(),
            state: row.state.clone(),
            source_url: row.source_urls[0].clone(),
            claimed_observed_on: row.last_observed.clone(),
            source_sha256: "a".repeat(64),
            fetched_at: fetched.to_string(),
            span: "some text".to_string(),
        },
        ContactClaimEvidence {
            field: ContactProofField::PublicProfessionalEmail,
            value: row.public_professional_email.clone(),
            person: row.coach_name.clone(),
            role: row.role.clone(),
            sport: row.sport.clone(),
            school: row.school.clone(),
            state: row.state.clone(),
            source_url: row.source_urls[0].clone(),
            claimed_observed_on: row.last_observed.clone(),
            source_sha256: "b".repeat(64),
            fetched_at: fetched.to_string(),
            span: "email snippet".to_string(),
        },
        ContactClaimEvidence {
            field: ContactProofField::AdName,
            value: row.ad_name.clone(),
            person: row.ad_name.clone(),
            role: "Athletic Director".to_string(),
            sport: row.sport.clone(),
            school: row.school.clone(),
            state: row.state.clone(),
            source_url: row.source_urls[0].clone(),
            claimed_observed_on: row.last_observed.clone(),
            source_sha256: "c".repeat(64),
            fetched_at: fetched.to_string(),
            span: "AD snippet".to_string(),
        },
        ContactClaimEvidence {
            field: ContactProofField::AdEmail,
            value: row.ad_email.clone(),
            person: row.ad_name.clone(),
            role: "Athletic Director".to_string(),
            sport: row.sport.clone(),
            school: row.school.clone(),
            state: row.state.clone(),
            source_url: row.source_urls[0].clone(),
            claimed_observed_on: row.last_observed.clone(),
            source_sha256: "d".repeat(64),
            fetched_at: fetched.to_string(),
            span: "AD email snippet".to_string(),
        },
    ]
}

mod claims;
mod director;

#[test]
fn valid_coach_ad_row_computes_proof() {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&row, &claims).expect("should compute proof");
    assert_eq!(proof.len(), 64, "proof must be 64 hex chars");

    let validated = verify_contact_proof(&row, &claims, &proof).expect("should verify");
    assert_eq!(validated.as_str(), proof);
}

#[test]
fn missing_email_claim_for_populated_email_field() {
    let row = make_valid_coach_row();
    let mut claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    claims.retain(|c| c.field != ContactProofField::PublicProfessionalEmail);
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ClaimFieldNotCovered(f) => {
            assert_eq!(f, "public_professional_email");
        }
        other => panic!("expected ClaimFieldNotCovered, got {:?}", other),
    }
}

#[test]
fn empty_claims_rejected() {
    let row = make_valid_coach_row();
    let claims: Vec<ContactClaimEvidence> = vec![];
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::UnverifiedClaim => {}
        other => panic!("expected UnverifiedClaim, got {:?}", other),
    }
}

#[test]
fn wrong_digest_rejected() {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let wrong_digest = "0".repeat(64);
    let result = verify_contact_proof(&row, &claims, &wrong_digest);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::DigestMismatch => {}
        other => panic!("expected DigestMismatch, got {:?}", other),
    }
}

#[test]
fn future_claimed_date_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].claimed_observed_on = "2026-01-01".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ClaimedAfterFetched => {}
        other => panic!("expected ClaimedAfterFetched, got {:?}", other),
    }
}

#[test]
fn invalid_date_format_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].claimed_observed_on = "Jan 10, 2025".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ClaimedObservedOnInvalid => {}
        other => panic!("expected ClaimedObservedOnInvalid, got {:?}", other),
    }
}

#[test]
fn claimed_digest_rejects_unicode_fake_hex() {
    let fake = "aaaa\u{03B1}".to_string() + &"a".repeat(60);
    let result = validate_claimed_digest(&fake);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::MalformedClaimedDigest => {}
        other => panic!("expected MalformedClaimedDigest, got {:?}", other),
    }
}

#[test]
fn source_hash_rejects_short_hex() {
    let result = validate_claimed_digest(&"a".repeat(63));
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::MalformedClaimedDigest => {}
        other => panic!("expected MalformedClaimedDigest, got {:?}", other),
    }
}

#[test]
fn source_hash_in_claim_must_be_valid_hex() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].source_sha256 = "g".repeat(64);
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::MalformedSourceHash => {}
        other => panic!("expected MalformedSourceHash, got {:?}", other),
    }
}

#[test]
fn proof_from_another_school_cannot_verify_valid_claims() -> Result<(), ContactProofError> {
    let first = make_valid_coach_row();
    let claims = make_valid_claims(&first, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&first, &claims)?;
    let mut second = first;
    second.school = "BetaU".to_string();
    let claims = make_valid_claims(&second, "2025-06-01T12:00:00+00:00");
    assert!(matches!(
        verify_contact_proof(&second, &claims, &proof),
        Err(ContactProofError::DigestMismatch)
    ));
    Ok(())
}

#[test]
fn email_case_mismatch_causes_failure() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[1].value = "coach@TESTU.EDU".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
}

#[test]
fn span_exceeds_limit() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].span = "x".repeat(4097);
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::SpanTooLong(_) => {}
        other => panic!("expected SpanTooLong, got {:?}", other),
    }
}

#[test]
fn fetched_at_invalid_format() {
    let row = make_valid_coach_row();
    let mut claims = make_valid_claims(&row, "not-rfc3339");
    claims[0].fetched_at = "not-rfc3339".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::FetchedAtInvalid => {}
        other => panic!("expected FetchedAtInvalid, got {:?}", other),
    }
}

#[test]
fn last_observed_invalid_format() {
    let mut row = make_valid_coach_row();
    row.last_observed = "not-a-date".to_string();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::LastObservedInvalid => {}
        other => panic!("expected LastObservedInvalid, got {:?}", other),
    }
}

#[test]
fn too_many_claims_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let base_claims = make_valid_claims(&row, fetched);
    let mut claims = base_claims.clone();
    for _ in 0..253 {
        claims.push(base_claims[0].clone());
    }
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::TooManyClaims(count, _) => {
            assert!(count > 256);
        }
        other => panic!("expected TooManyClaims, got {:?}", other),
    }
}
