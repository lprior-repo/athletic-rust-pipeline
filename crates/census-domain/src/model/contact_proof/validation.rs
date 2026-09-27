use super::{ContactClaimEvidence, ContactProofError, ContactProofField, RawContactRow};
use chrono::{DateTime, NaiveDate, Utc};

pub(super) fn validate_claimed_digest(digest: &str) -> Result<(), ContactProofError> {
    is_hex64(digest)
        .then_some(())
        .ok_or(ContactProofError::MalformedClaimedDigest)
}

pub(super) fn validate_structure(
    row: &RawContactRow,
    claims: &[ContactClaimEvidence],
) -> Result<(), ContactProofError> {
    if claims.is_empty() {
        return Err(ContactProofError::UnverifiedClaim);
    }
    if claims.len() > super::MAX_CLAIMS {
        return Err(ContactProofError::TooManyClaims(claims.len(), super::MAX_CLAIMS));
    }
    parse_date(&row.last_observed).ok_or(ContactProofError::LastObservedInvalid)?;
    if row.source_urls.is_empty() {
        return Err(ContactProofError::EmptySourceUrl);
    }
    for url in &row.source_urls {
        validate_url(url)?;
    }
    let mut covered = [false; 4];
    for claim in claims {
        validate_claim(claim)?;
        validate_claim_against_row(row, claim, &mut covered)?;
    }
    let fields = [
        ("coach_name", row.coach_name.as_str()),
        ("public_professional_email", row.public_professional_email.as_str()),
        ("ad_name", row.ad_name.as_str()),
        ("ad_email", row.ad_email.as_str()),
    ];
    for ((name, value), present) in fields.into_iter().zip(covered) {
        if !value.is_empty() && !present {
            return Err(ContactProofError::ClaimFieldNotCovered(name.to_owned()));
        }
    }
    Ok(())
}

fn validate_claim(claim: &ContactClaimEvidence) -> Result<(), ContactProofError> {
    if [
        &claim.value, &claim.person, &claim.role, &claim.school,
        &claim.state, &claim.span,
    ].into_iter().any(|value| value.trim().is_empty())
        || (claim.sport.trim().is_empty() && !is_director(claim.field))
    {
        return Err(ContactProofError::UnverifiedClaim);
    }
    if !is_hex64(&claim.source_sha256) {
        return Err(ContactProofError::MalformedSourceHash);
    }
    validate_url(&claim.source_url)?;
    if claim.span.len() > super::MAX_SPAN {
        return Err(ContactProofError::SpanTooLong(claim.span.len()));
    }
    let claimed_date = parse_date(&claim.claimed_observed_on)
        .ok_or(ContactProofError::ClaimedObservedOnInvalid)?;
    let fetched = DateTime::parse_from_rfc3339(&claim.fetched_at)
        .map_err(|_| ContactProofError::FetchedAtInvalid)?;
    if claimed_date > fetched.with_timezone(&Utc).date_naive() {
        return Err(ContactProofError::ClaimedAfterFetched);
    }
    Ok(())
}

fn validate_claim_against_row(
    row: &RawContactRow,
    claim: &ContactClaimEvidence,
    covered: &mut [bool; 4],
) -> Result<(), ContactProofError> {
    let (value, person, role, present) = match claim.field {
        ContactProofField::CoachName =>
            (&row.coach_name, &row.coach_name, row.role.as_str(), &mut covered[0]),
        ContactProofField::PublicProfessionalEmail =>
            (&row.public_professional_email, &row.coach_name, row.role.as_str(), &mut covered[1]),
        ContactProofField::AdName =>
            (&row.ad_name, &row.ad_name, "Athletic Director", &mut covered[2]),
        ContactProofField::AdEmail =>
            (&row.ad_email, &row.ad_name, "Athletic Director", &mut covered[3]),
    };
    let sport_matches = claim.sport == row.sport
        || (is_director(claim.field) && claim.sport.is_empty());
    if &claim.value != value || &claim.person != person || claim.role != role
        || claim.school != row.school || claim.state != row.state || !sport_matches
    {
        return Err(ContactProofError::ValueMismatch(format!("{:?}", claim.field)));
    }
    if claim.claimed_observed_on != row.last_observed {
        return Err(ContactProofError::ValueMismatch("claimed_observed_on".to_owned()));
    }
    if !row.source_urls.contains(&claim.source_url) {
        return Err(ContactProofError::SourceUrlNotFound);
    }
    *present = true;
    Ok(())
}

fn is_director(field: ContactProofField) -> bool {
    matches!(field, ContactProofField::AdName | ContactProofField::AdEmail)
}




fn parse_date(value: &str) -> Option<NaiveDate> {
    let canonical = value.len() == 10
        && value.bytes().enumerate().all(|(index, byte)| match index {
            4 | 7 => byte == b'-',
            _ => byte.is_ascii_digit(),
        });
    canonical.then(|| NaiveDate::parse_from_str(value, "%Y-%m-%d").ok()).flatten()
}

fn validate_url(value: &str) -> Result<(), ContactProofError> {
    if value.trim().is_empty() {
        return Err(ContactProofError::EmptySourceUrl);
    }
    if value.len() > super::MAX_URL {
        return Err(ContactProofError::SourceUrlTooLong(value.len()));
    }
    Ok(())
}

fn is_hex64(s: &str) -> bool {
    if s.len() != 64 {
        return false;
    }
    s.as_bytes()
        .iter()
        .all(|&b| (b >= b'0' && b <= b'9') || (b >= b'a' && b <= b'f'))
}
