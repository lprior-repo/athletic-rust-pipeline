use super::{compute_contact_proof, verify_contact_proof, ContactProofError};
use super::make_valid_coach_row;
use super::make_valid_claims;


#[test]
fn wrong_person_for_email_claim() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[1].person = "Wrong Person".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}


#[test]
fn wrong_role_for_coach_claim() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].role = "Wrong Role".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}

#[test]
fn wrong_role_for_ad_claim() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[2].role = "Wrong Role".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}


#[test]
fn wrong_school_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].school = "OtherU".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}

#[test]
fn wrong_state_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].state = "NY".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}

#[test]
fn wrong_sport_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].sport = "Basketball".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}


#[test]
fn source_url_not_in_row() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].source_url = "https://other.com/page".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::SourceUrlNotFound => {}
        other => panic!("expected SourceUrlNotFound, got {:?}", other),
    }
}

#[test]
fn empty_source_url_in_claim() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].source_url = "".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::EmptySourceUrl => {}
        other => panic!("expected EmptySourceUrl, got {:?}", other),
    }
}


#[test]
fn mismatched_date_rejected() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    claims[0].claimed_observed_on = "2024-01-01".to_string();
    let result = compute_contact_proof(&row, &claims);
    assert!(result.is_err(), "claimed_observed_on must match last_observed");
    match result.unwrap_err() {
        ContactProofError::ValueMismatch(_) => {}
        other => panic!("expected ValueMismatch, got {:?}", other),
    }
}


#[test]
fn empty_evidence_span_cannot_verify_a_contact() -> Result<(), ContactProofError> {
    let row = make_valid_coach_row();
    let mut claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&row, &claims)?;
    for span in ["", " \t\n"] {
        claims.first_mut().ok_or(ContactProofError::UnverifiedClaim)?.span = span.to_owned();
        assert!(matches!(
            verify_contact_proof(&row, &claims, &proof),
            Err(ContactProofError::UnverifiedClaim)
        ));
    }
    Ok(())
}


#[test]
fn valid_coach_and_ad_claims_pass() {
    let row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    let proof = compute_contact_proof(&row, &claims).expect("should compute proof");
    assert_eq!(proof.len(), 64);

    let validated =
        verify_contact_proof(&row, &claims, &proof).expect("should verify");
    assert_eq!(validated.as_str(), proof);
}


#[test]
fn changed_source_hash_rejects_old_proof() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    let proof = compute_contact_proof(&row, &claims).unwrap();
    claims[0].source_sha256 = "e".repeat(64);
    let result = verify_contact_proof(&row, &claims, &proof);
    assert!(result.is_err());
    match result.unwrap_err() {
        ContactProofError::DigestMismatch => {}
        other => panic!("expected DigestMismatch, got {:?}", other),
    }
}


#[test]
fn changed_source_url_rejects_old_proof() {
    let row = make_valid_coach_row();
    let fetched = "2025-06-01T12:00:00+00:00";
    let mut claims = make_valid_claims(&row, fetched);
    let proof = compute_contact_proof(&row, &claims).unwrap();
    claims[0].source_url = "https://changed.com/page".to_string();
    let result = verify_contact_proof(&row, &claims, &proof);
    assert!(result.is_err());
}

#[test]
fn unused_empty_source_url_rejects_the_row() {
    let mut row = make_valid_coach_row();
    let claims = make_valid_claims(&row, "2025-06-01T12:00:00+00:00");
    row.source_urls.push(String::new());
    assert!(matches!(
        compute_contact_proof(&row, &claims),
        Err(ContactProofError::EmptySourceUrl)
    ));
}
