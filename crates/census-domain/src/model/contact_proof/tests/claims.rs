use super::make_valid_claims;
use super::make_valid_coach_row;
use super::{compute_contact_proof, verify_contact_proof, ContactProofError};

#[test]
fn wrong_person_for_email_claim() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[1].person = "Wrong Person".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn wrong_role_for_coach_claim() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].role = "Wrong Role".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn wrong_role_for_ad_claim() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[2].role = "Wrong Role".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn wrong_school_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].school = "OtherU".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn wrong_state_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].state = "NY".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn wrong_sport_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].sport = "Basketball".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn source_url_not_in_row() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].source_url = "https://other.com/page".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::SourceUrlNotFound => {}
        other => return Err(format!("expected SourceUrlNotFound, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn empty_source_url_in_claim() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].source_url = "".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::EmptySourceUrl => {}
        other => return Err(format!("expected EmptySourceUrl, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn mismatched_date_rejected() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].claimed_observed_on = "2024-01-01".to_string();
    let result = compute_contact_proof(&row, &claims);
    check!(
        result.is_err(),
        "claimed_observed_on must match last_observed"
    );
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::ValueMismatch(_) => {}
        other => return Err(format!("expected ValueMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn empty_evidence_span_cannot_verify_a_contact() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let mut claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&row, &claims)?;
    for span in ["", " \t\n"] {
        claims
            .first_mut()
            .ok_or(ContactProofError::UnverifiedClaim)?
            .span = span.to_owned();
        check!(matches!(
            verify_contact_proof(&row, &claims, &proof),
            Err(ContactProofError::UnverifiedClaim)
        ));
    }
    Ok(())
}

#[test]
fn valid_coach_and_ad_claims_pass() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&row, &claims)?;
    check!(eq; proof.len(), 64);

    let validated = verify_contact_proof(&row, &claims, &proof)?;
    check!(eq; validated.as_str(), proof);
    Ok(())
}

#[test]
fn changed_source_hash_rejects_old_proof() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    let proof = compute_contact_proof(&row, &claims)?;
    claims[0].source_sha256 = "e".repeat(64);
    let result = verify_contact_proof(&row, &claims, &proof);
    check!(result.is_err());
    match result.err().ok_or("expected contact-proof rejection")? {
        ContactProofError::DigestMismatch => {}
        other => return Err(format!("expected DigestMismatch, got {:?}", other).into()),
    }
    Ok(())
}

#[test]
fn changed_source_url_rejects_old_proof() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    let proof = compute_contact_proof(&row, &claims)?;
    claims[0].source_url = "https://changed.com/page".to_string();
    let result = verify_contact_proof(&row, &claims, &proof);
    check!(result.is_err());
    Ok(())
}

#[test]
fn unused_empty_source_url_rejects_the_row() -> Result<(), Box<dyn std::error::Error>> {
    let mut row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    row.source_urls.push(String::new());
    check!(matches!(
        compute_contact_proof(&row, &claims),
        Err(ContactProofError::EmptySourceUrl)
    ));
    Ok(())
}
