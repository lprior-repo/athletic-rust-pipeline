use super::*;

fn unscoped_director_claims(row: &RawContactRow) -> Vec<ContactClaimEvidence> {
    let mut claims = make_valid_claims(row, "2025-06-01T12:00:00Z");
    for claim in &mut claims {
        if matches!(
            claim.field,
            ContactProofField::AdName | ContactProofField::AdEmail
        ) {
            claim.sport.clear();
        }
    }
    claims
}

#[test]
fn school_wide_director_claims_do_not_need_a_coach_sport() -> Result<(), Box<dyn std::error::Error>>
{
    let row = make_valid_coach_row();
    let claims = unscoped_director_claims(&row);
    let digest = compute_contact_proof(&row, &claims)?;
    check!(eq; verify_contact_proof(&row, &claims, &digest)?.as_str(),
    digest);
    let mut changed = row;
    changed.ad_name = "Another Director".to_owned();
    check!(matches!(
        verify_contact_proof(&changed, &claims, &digest),
        Err(ContactProofError::ValueMismatch(_))
    ));
    Ok(())
}

#[test]
fn coach_claims_still_require_the_published_sport() -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    for field in [
        ContactProofField::CoachName,
        ContactProofField::PublicProfessionalEmail,
    ] {
        let mut claims = unscoped_director_claims(&row);
        claims
            .iter_mut()
            .find(|claim| claim.field == field)
            .ok_or("missing coach claim fixture")?
            .sport
            .clear();
        check!(eq; compute_contact_proof(&row, &claims),
        Err(ContactProofError::UnverifiedClaim));
    }
    Ok(())
}

#[test]
fn a_sport_specific_director_claim_cannot_cover_a_different_program(
) -> Result<(), Box<dyn std::error::Error>> {
    let row = make_valid_coach_row();
    for field in [ContactProofField::AdName, ContactProofField::AdEmail] {
        let mut claims = unscoped_director_claims(&row);
        claims
            .iter_mut()
            .find(|claim| claim.field == field)
            .ok_or("missing director claim fixture")?
            .sport = "Basketball".to_owned();
        check!(matches!(
            compute_contact_proof(&row, &claims),
            Err(ContactProofError::ValueMismatch(_))
        ));
    }
    Ok(())
}
