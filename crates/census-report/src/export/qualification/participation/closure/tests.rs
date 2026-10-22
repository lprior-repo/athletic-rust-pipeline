use super::*;
use census_domain::model::SourceRef;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn exact_distinct_proof_boundary_accepts_duplicates_and_refuses_next_without_mutation() -> TestResult
{
    let mut references = BTreeSet::new();
    for ordinal in 0..MAX_EVIDENCE_REFERENCES {
        retain_reference(&mut references, &evidence(ordinal))?;
    }
    check!(eq; u64::try_from(references.len())?, MAX_EVIDENCE_REFERENCES);
    let before = references.clone();
    retain_reference(&mut references, &evidence(0))?;
    check!(eq; references, before);
    check!(matches!(
        retain_reference(&mut references, &evidence(MAX_EVIDENCE_REFERENCES)),
        Err(StoreError::BudgetExceeded {
            what: "participation evidence closure distinct proofs",
            needed: 65_537,
            available: 65_536,
        })
    ));
    check!(eq; references, before);
    Ok(())
}

#[test]
fn proofs_above_previous_limit_keep_exact_canonical_evidence_references() -> TestResult {
    let mut references = BTreeSet::new();
    for ordinal in 0..257 {
        retain_reference(&mut references, &evidence(ordinal))?;
    }
    check!(eq; references.len(), 257);
    for ordinal in 0..257 {
        check!(references.contains(&format!(
            "canonical_evidence:{}",
            serialized_digest(&evidence(ordinal))?
        )));
    }
    Ok(())
}

fn evidence(ordinal: u64) -> Evidence {
    Evidence::derived(
        SourceRef::id(format!("proof-{ordinal}")),
        "2026-10-22",
        "retained boundary fixture",
    )
}
