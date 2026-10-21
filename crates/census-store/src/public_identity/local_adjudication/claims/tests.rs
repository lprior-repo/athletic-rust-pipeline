use super::*;
use crate::public_identity_tests::support::{subject, TestResult};

#[test]
fn running_claim_budget_accepts_exact_size_and_stops_before_retaining_overflow() -> TestResult {
    let candidate = subject("claim-budget", "1")?.id.cast();
    let value = serde_json::json!("retained observation");
    let claim = RetainedClaimV1 {
        candidate,
        section: RetainedIdentitySectionV1::Athlete,
        ordinal: 0,
        row_digest: "row-digest".into(),
        claim_path: "/name".into(),
        claim_digest: serialized_digest(&value)?,
        retained_value: value.clone(),
    };
    let size = serde_json::to_vec(&claim)?.len();
    let mut inventory = Inventory {
        claims: Vec::new(),
        remaining: size,
    };
    inventory.push(
        &claim.candidate,
        claim.section,
        0,
        &claim.row_digest,
        claim.claim_path.clone(),
        &value,
    )?;
    check!(eq;inventory.remaining, 0);
    check!(eq;inventory.claims, vec![claim.clone()]);
    check!(inventory
        .push(
            &claim.candidate,
            claim.section,
            0,
            &claim.row_digest,
            claim.claim_path.clone(),
            &value
        )
        .is_err());
    check!(eq;inventory.claims, vec![claim.clone()]);
    let mut short = Inventory {
        claims: Vec::new(),
        remaining: size.checked_sub(1).ok_or("size")?,
    };
    check!(short
        .push(
            &claim.candidate,
            claim.section,
            0,
            &claim.row_digest,
            claim.claim_path.clone(),
            &value
        )
        .is_err());
    check!(short.claims.is_empty());
    Ok(())
}
