use super::evidence::RowEvidence;
use super::*;
use census_domain::model::{ContactClaimEvidence, ContactProofField, RawContactRow};

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

fn evaluate(row: &RawContactRow, body: &str) -> RowEvidence {
    let mut evidence = RowEvidence::default();
    let sha256 = format!("{:x}", sha2::Digest::sha256(body.as_bytes()));
    evidence
        .absorb(
            body,
            row,
            &row.source_urls[0],
            "2026-09-26T12:00:00Z",
            &sha256,
        )
        .expect("static selectors");
    evidence
}

#[test]
fn real_name_does_not_verify_an_absent_address() {
    let evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid</td><td>Head XC Coach</td></tr>"),
    );
    assert!(!evidence.verdict().shipped());
    assert!(!evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::PublicProfessionalEmail));
}

#[test]
fn valid_dual_sport_appointment_accepted() {
    let body = staff("<tr><td>Dana Reid Head XC and Basketball Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body);
    assert_eq!(evidence.verdict(), Verdict::Ok);
    assert!(evidence
        .claims
        .iter()
        .any(|claim| claim.field == ContactProofField::PublicProfessionalEmail));
}

#[test]
fn basketball_only_does_not_establish_xc() {
    let body = staff("<tr><td>Dana Reid Head Basketball Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body);
    assert!(!evidence.verdict().shipped());
    assert!(!evidence
        .claims
        .iter()
        .any(|c| c.field == ContactProofField::PublicProfessionalEmail));
}

#[test]
fn contradiction_overrides_a_matching_role() {
    let mut evidence = evaluate(
        &fragment(),
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    );
    assert_eq!(evidence.verdict(), Verdict::Ok);
    evidence.contradicted = true;
    assert_eq!(evidence.verdict(), Verdict::RoleContradicted);
}

#[test]
fn adjacent_staff_cards_cannot_supply_someone_elses_email() {
    let body = staff("<tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Other Person Head XC Coach dana@example.org</td></tr>");
    assert!(!evaluate(&fragment(), &body).verdict().shipped());
}

#[test]
fn exact_record_preserves_field_relationship_and_source_bytes() {
    let body = staff("<tr><td>Dana Reid</td><td>Head XC Coach</td><td>dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body);
    assert_eq!(evidence.verdict(), Verdict::Ok);
    let email = evidence
        .claims
        .iter()
        .find(|claim| claim.field == ContactProofField::PublicProfessionalEmail)
        .expect("verified address");
    assert_eq!(email.value, "dana@example.org");
    assert_eq!(email.person, "Dana Reid");
    assert_eq!(email.source_url, "https://example.org/staff");
    assert_eq!(email.fetched_at, "2026-09-26T12:00:00Z");
}

#[test]
fn public_role_consumer_mailbox_is_not_discarded() {
    let mut row = fragment();
    row.public_professional_email = "schooltrack@gmail.com".to_string();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach schooltrack@gmail.com</td></tr>"),
    );
    assert_eq!(evidence.verdict(), Verdict::Ok);
}

#[test]
fn a_different_school_cannot_verify_the_claimed_institution() {
    let body = "<h1>Other High School WI</h1><table><tr><td>Dana Reid Head XC Coach dana@example.org</td></tr></table>";
    assert!(!evaluate(&fragment(), body).verdict().shipped());
}

#[test]
fn former_role_escapes_as_contradiction() {
    let body = staff("<tr><td>Dana Reid former Head XC Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body);
    assert!(evidence.contradicted);
    assert!(!evidence.verdict().shipped());
}

#[test]
fn not_current_role_triggers_contradiction() {
    let body = staff("<tr><td>Dana Reid not current Head XC Coach dana@example.org</td></tr>");
    let evidence = evaluate(&fragment(), &body);
    assert!(evidence.contradicted);
}

#[test]
fn different_school_card_does_not_verify() {
    let body = "<h1>Mosinee High School WI</h1><table><tr><td>Dana Reid Head XC Coach</td></tr><tr><td>Jane Smith Head XC Coach dana@example.org</td></tr></table>";
    let evidence = evaluate(&fragment(), &body);
    assert!(!evidence.verdict().shipped());
}

#[test]
fn exact_email_bytes_preserved_not_casefolded() {
    let body = staff("<tr><td>Dana Reid Head XC Coach Dana@Example.ORG</td></tr>");
    let mut row = fragment();
    row.public_professional_email = "Dana@Example.ORG".to_string();
    let evidence = evaluate(&row, &body);
    assert_eq!(evidence.verdict(), Verdict::Ok);
    let email = evidence
        .claims
        .iter()
        .find(|c| c.field == ContactProofField::PublicProfessionalEmail)
        .unwrap();
    assert_eq!(email.value, "Dana@Example.ORG");
}

#[test]
fn proof_digest_matches_evidence() {
    let body = staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>");
    let row = fragment();
    let mut evidence = evaluate(&row, &body);
    let digest = super::census_domain::model::compute_contact_proof(&row, &evidence.claims)
        .expect("valid proof");
    assert!(!digest.is_empty());
    let mut mutated_claims = evidence.claims.clone();
    if let Some(claim) = mutated_claims.first_mut() {
        claim.value = "mutated@example.org".to_string();
    }
    let result = super::census_domain::model::compute_contact_proof(&row, &mutated_claims);
    assert!(result.is_err());
}

#[test]
fn uncertain_or_failed_evidence_never_ships() {
    [
        Verdict::OkRoleContext,
        Verdict::RoleContradicted,
        Verdict::RobotsBlocked,
        Verdict::FetchFailed,
        Verdict::Empty,
        Verdict::Mismatch,
        Verdict::RenderRequired,
    ]
    .into_iter()
    .for_each(|verdict| assert!(!verdict.shipped()));
}

#[test]
fn future_observation_dates_cannot_be_verified() {
    let mut row = fragment();
    row.last_observed = "2027-09-21".to_string();
    assert!(!evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>")
    )
    .verdict()
    .shipped());
}

#[test]
fn final_reconciliation_detects_email_or_citation_mutation() {
    let dir = tempfile::tempdir().expect("scratch");
    let path = dir.path().join("contacts.csv");
    let row = fragment();
    let evidence = evaluate(
        &row,
        &staff("<tr><td>Dana Reid Head XC Coach dana@example.org</td></tr>"),
    );
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
    write_fragment(&path, std::slice::from_ref(&outcome)).expect("publish");
    assert_eq!(
        reconcile(&path, std::slice::from_ref(&verified))
            .expect("reconcile")
            .unmatched_total(),
        0
    );
    let mut mutated = outcome.clone();
    mutated.row.public_professional_email = "someoneelse@example.org".to_string();
    write_fragment(&path, &[mutated]).expect("publish changed row");
    assert_eq!(
        reconcile(&path, std::slice::from_ref(&verified))
            .expect("reconcile")
            .unmatched_total(),
        1
    );
    let mut mutated = outcome;
    mutated.row.source_urls = vec!["https://different.example.org/staff".to_string()];
    write_fragment(&path, &[mutated]).expect("publish changed citation");
    assert_eq!(
        reconcile(&path, &[verified])
            .expect("reconcile")
            .unmatched_total(),
        1
    );
}
