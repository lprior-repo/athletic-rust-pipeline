use super::validation::validate_claimed_digest;
use super::{
    compute_contact_proof, verify_contact_proof, ContactClaimEvidence, ContactProofError,
    ContactProofField, RawContactRow,
};

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
fn valid_coach_ad_row_computes_proof() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&row, &claims)?;
    check!(eq; proof.len(), 64, "proof must be 64 hex chars");

    let validated = verify_contact_proof(&row, &claims, &proof)?;
    check!(eq; validated.as_str(), proof);
    Ok(())
}

#[test]
fn missing_email_claim_for_populated_email_field() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let mut claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    claims.retain(|c| c.field != ContactProofField::PublicProfessionalEmail);
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ClaimFieldNotCovered(f) => {
            check!(eq; f, "public_professional_email");
        }
        other => return Err(format!("expected ClaimFieldNotCovered, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn empty_claims_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let claims: Vec<ContactClaimEvidence> = vec![];
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::UnverifiedClaim => {}
        other => return Err(format!("expected UnverifiedClaim, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn wrong_digest_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let wrong_digest = "0".repeat(64);
    let result = verify_contact_proof(&row, &claims, &wrong_digest);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::DigestMismatch => {}
        other => return Err(format!("expected DigestMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn future_claimed_date_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].claimed_observed_on = "2026-01-01".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ClaimedAfterFetched => {}
        other => return Err(format!("expected ClaimedAfterFetched, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn invalid_date_format_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].claimed_observed_on = "Jan 10, 2025".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ClaimedObservedOnInvalid => {}
        other => return Err(format!("expected ClaimedObservedOnInvalid, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn claimed_digest_rejects_unicode_fake_hex() -> Result<(), Box<dyn std::error::Error>> {
    let fake = "aaaa\u{03B1}".to_string() + &"a".repeat(60);
    let result = validate_claimed_digest(&fake);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::MalformedClaimedDigest => {}
        other => return Err(format!("expected MalformedClaimedDigest, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn source_hash_rejects_short_hex() -> Result<(), Box<dyn std::error::Error>> {
    let result = validate_claimed_digest(&"a".repeat(63));
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::MalformedClaimedDigest => {}
        other => return Err(format!("expected MalformedClaimedDigest, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn source_hash_in_claim_must_be_valid_hex() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].source_sha256 = "g".repeat(64);
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::MalformedSourceHash => {}
        other => return Err(format!("expected MalformedSourceHash, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn proof_from_another_school_cannot_verify_valid_claims() -> Result<(), Box<dyn std::error::Error>>
{
    let first = make_valid_coach_row();
    let claims = make_valid_claims(&first, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&first, &claims)?;
    let mut second = first;
    second.school = "BetaU".to_string();
    let claims = make_valid_claims(&second, "2025-06-01T12:00:00+00:00");
    check!(matches!(
        verify_contact_proof(&second, &claims, &proof),
        Err(ContactProofError::DigestMismatch)
    ));
    Ok(())
}

#[test]
fn email_case_mismatch_causes_failure() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[1].value = "coach@TESTU.EDU".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    Ok(())
}

#[test]
fn span_exceeds_limit() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].span = "x".repeat(4097);
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::SpanTooLong(_) => {}
        other => return Err(format!("expected SpanTooLong, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn fetched_at_invalid_format() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let mut claims = make_valid_claims(&row, "not-rfc3339");
    claims[0].fetched_at = "not-rfc3339".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::FetchedAtInvalid => {}
        other => return Err(format!("expected FetchedAtInvalid, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn last_observed_invalid_format() -> Result<(), Box<dyn std::error::Error>> {
    let mut row = make_valid_coach_row();
    row.last_observed = "not-a-date".to_string();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::LastObservedInvalid => {}
        other => return Err(format!("expected LastObservedInvalid, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn too_many_claims_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let base_claims = make_valid_claims(&row, fetched);
    let mut claims = base_claims.clone();
    for _ in 0..253 {
        claims.push(base_claims[0].clone());
    }
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::TooManyClaims(count, _) => {
            check!(count > 256);
        }
        other => return Err(format!("expected TooManyClaims, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn a_claim_that_disagrees_with_the_row_does_not_bind() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-02-01T00:00:00Z");
    let email_claim = claims
        .iter()
        .find(|claim| claim.field == ContactProofField::PublicProfessionalEmail)
        .ok_or("fixture carries an email claim")?
        .clone();
    check!(super::claim_binds_to_row(&row, &email_claim));
    let mut emailless = row.clone();
    emailless.public_professional_email = String::new();
    check!(!super::claim_binds_to_row(&emailless, &email_claim));
    let mut stale = email_claim.clone();
    stale.claimed_observed_on = "2025-01-14".to_string();
    check!(!super::claim_binds_to_row(&row, &stale));
    let mut foreign = email_claim;
    foreign.source_url = "https://example.com/other".to_string();
    check!(!super::claim_binds_to_row(&row, &foreign));
    Ok(())
}
