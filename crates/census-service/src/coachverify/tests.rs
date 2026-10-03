use super::evidence::RowEvidence;
use super::*;
use census_domain::model::{ContactProofField, RawContactRow};
use sha2::{Digest, Sha256};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn fragment() -> RawContactRow {
    RawContactRow {
        school: "Mosinee High School".to_string(),
        city: "Mosinee".to_string(),
        state: "WI".to_string(),
        sport: "Cross Country".to_string(),
        role: "Head XC Coach".to_string(),
        coach_name: "Dana Reid".to_string(),
        public_professional_email: "dana@example.org".to_string(),
        ad_name: String::new(),
        ad_email: String::new(),
        source_urls: vec!["https://example.org/staff".to_string()],
        last_observed: "2026-09-21".to_string(),
    }
}

fn staff(body: &str) -> String {
    format!("<h1>Mosinee High School WI</h1><table>{body}</table>")
}

fn evaluate(row: &RawContactRow, body: &str) -> TestResult<RowEvidence> {
    let mut evidence = RowEvidence::default();
    let sha256 = format!("{:x}", Sha256::digest(body.as_bytes()));
    evidence.absorb(
        body,
        row,
        &row.source_urls[0],
        "2026-09-26T12:00:00Z",
        &sha256,
    )?;
    Ok(evidence)
}

#[test]
fn real_name_does_not_verify_an_absent_address() -> TestResult {
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid</td><td>Head XC Coach</td></tr>"),
    )?;
    check!(!evidence.verdict().shipped());
    check!(!evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::PublicProfessionalEmail));
    Ok(())
}

#[test]
fn valid_dual_sport_appointment_accepted() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC and Basketball Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    check!(evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::PublicProfessionalEmail));
    Ok(())
}

#[test]
fn basketball_only_does_not_establish_xc() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head Basketball Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(!evidence.verdict().shipped());
    check!(!evidence
        .claims
        .iter()
        .any(|c| c.field == ContactProofField::PublicProfessionalEmail));
    Ok(())
}

#[test]
fn contradiction_overrides_a_matching_role() -> TestResult {
    let mut evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    evidence.contradicted = true;
    check!(eq; evidence.verdict(), Verdict::RoleContradicted);
    Ok(())
}

#[test]
fn adjacent_staff_cards_cannot_supply_someone_elses_email() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Other Person Head XC Coach dana@example.org</td></tr>");
    check!(!evaluate(&fragment(), &body)?.verdict().shipped());
    Ok(())
}

#[test]
fn exact_record_preserves_field_relationship_and_source_bytes() -> TestResult {
    let body = staff("<tr><td>Dana Reid</td><td>Head XC Coach</td><td>dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    let email = evidence
        .claims
        .iter()
        .find(|claim| claim.field == ContactProofField::PublicProfessionalEmail)
        .ok_or("missing verified address")?;
    check!(eq; email.value, "dana@example.org");
    check!(eq; email.person, "Dana Reid");
    check!(eq; email.source_url, "https://example.org/staff");
    check!(eq; email.fetched_at, "2026-09-26T12:00:00Z");
    Ok(())
}

#[test]
fn public_role_consumer_mailbox_is_not_discarded() -> TestResult {
    let mut row = fragment();
    row.public_professional_email = "schooltrack@gmail.com".to_string();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach schooltrack@gmail.com</td></tr>"),
    )?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    Ok(())
}

#[test]
fn a_different_school_cannot_verify_the_claimed_institution() -> TestResult {
    let body = "<h1>Other High School WI</h1><table><tr><td>Dana Reid Head XC Coach dana@example.org</td></tr></table>";
    check!(!evaluate(&fragment(), body)?.verdict().shipped());
    Ok(())
}

#[test]
fn former_role_escapes_as_contradiction() -> TestResult {
    let body = staff("<tr><td>Dana Reid former Head XC Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(evidence.contradicted);
    check!(!evidence.verdict().shipped());
    Ok(())
}

#[test]
fn not_current_role_triggers_contradiction() -> TestResult {
    let body = staff("<tr><td>Dana Reid not current Head XC Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body)?;
    check!(evidence.contradicted);
    Ok(())
}

#[test]
fn different_school_card_does_not_verify() -> TestResult {
    let body = "<h1>Mosinee High School WI</h1><table><tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Jane Smith Head XC Coach dana@example.org</td></tr></table>";
    let evidence = evaluate(&fragment(), body)?;
    check!(!evidence.verdict().shipped());
    Ok(())
}

#[test]
fn exact_email_bytes_preserved_not_casefolded() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach Dana@Example.ORG</td></tr>");
    let mut row = fragment();
    row.public_professional_email = "Dana@Example.ORG".to_string();
    let evidence = evaluate(&row, &body)?;
    check!(eq; evidence.verdict(), Verdict::Ok);
    let email = evidence
        .claims
        .iter()
        .find(|c| c.field == ContactProofField::PublicProfessionalEmail)
        .ok_or("missing verified address")?;
    check!(eq; email.value, "Dana@Example.ORG");
    Ok(())
}

#[test]
fn proof_digest_matches_evidence() -> TestResult {
    let body = staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>");
    let row = fragment();
    let evidence = evaluate(&row, &body)?;
    let digest = census_domain::model::compute_contact_proof(&row, &evidence.claims)?;
    check!(!digest.is_empty());
    let mut mutated_claims = evidence.claims.clone();
    if let Some(claim) = mutated_claims.first_mut() {
        claim.value = "mutated@example.org".to_string();
    }
    let result = census_domain::model::compute_contact_proof(&row, &mutated_claims);
    check!(result.is_err());
    Ok(())
}

#[test]
fn uncertain_or_failed_evidence_never_ships() {
    [
        Verdict::OkRoleContext,
        Verdict::RoleContradicted,
        Verdict::FetchFailed,
        Verdict::Empty,
        Verdict::Mismatch,
        Verdict::RenderRequired,
    ]
    .into_iter()
    .for_each(|verdict| assert!(!verdict.shipped()));
}

#[test]
fn future_observation_dates_cannot_be_verified() -> TestResult {
    let mut row = fragment();
    row.last_observed = "2027-09-21".to_string();
    check!(!evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>")
    )?
    .verdict()
    .shipped());
    Ok(())
}

#[test]
fn final_reconciliation_detects_email_or_citation_mutation() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("contacts.csv");
    let row = fragment();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    )?;
    let outcome = RowOutcome {
        row,
        verdict: evidence.verdict(),
        evidence: evidence.claims,
    };
    let verified = FragmentOutcome {
        file: "WI.csv".to_string(),
        rows: vec![outcome.clone()],
        counts: Default::default(),
    };
    write_fragment(&path, std::slice::from_ref(&outcome))?;
    check!(eq;
        reconcile(&path, std::slice::from_ref(&verified))?
            .unmatched_total(),
        0
    );
    let mut mutated = outcome.clone();
    mutated.row.public_professional_email = "someoneelse@example.org".to_string();
    write_fragment(&path, &[mutated])?;
    check!(eq;
        reconcile(&path, std::slice::from_ref(&verified))?
            .unmatched_total(),
        1
    );
    let mut mutated = outcome;
    mutated.row.source_urls = vec!["https://different.example.org/staff".to_string()];
    write_fragment(&path, &[mutated])?;
    check!(eq;
        reconcile(&path, &[verified])?
            .unmatched_total(),
        1
    );
    Ok(())
}
