use super::fixture::{candidate, FIRST};
use super::*;
use crate::public_identity_history::local_adjudication::*;
use crate::public_identity_history::local_membership_withholding::*;
use serde_json::{json, Value};

mod manual;
mod withholding;

fn claim() -> RetainedClaimV1 {
    RetainedClaimV1 {
        candidate: candidate("a"),
        section: RetainedIdentitySectionV1::CandidateMembership,
        ordinal: 1,
        row_digest: "a".repeat(64),
        claim_path: "membership:one".into(),
        claim_digest: "b".repeat(64),
        retained_value: json!({"source":"retained"}),
    }
}

fn wire() -> Value {
    let mut members = vec![candidate("a"), candidate("b")];
    members.sort();
    json!({
        "version":3,"policy":LOCAL_ADJUDICATION_GRAPH_POLICY,
        "store":"1".repeat(64),"operation":"manual:one",
        "expected_head":{"sequence":1,"digest":"a".repeat(64)},
        "evidence_generation":1,"derived_generation":1,"snapshot_sequence":1,
        "graph":{"version":1,"capture_digest":"c".repeat(64),"seal_root":"d".repeat(64)},
        "members":members,"observed_at":"2026-10-15T00:00:00Z",
        "evidence_basis":"RetainedObservationOnly",
        "request":{"case_id":"case:one","canonical":FIRST,
            "supersedes":[{"sequence":1,"digest":"a".repeat(64)}],
            "partitions":[],"unresolved_bindings":[],"affected_record_keys":["performance:one"],
            "dispositions":[{"claim":claim(),"treatment":"RejectIdentityCorroboration","rationale":"reviewed retained evidence"}],
            "rationale":"reviewed retained evidence"}
    })
}

fn replace(value: &mut Value, key: &str, replacement: Value) -> TestResult {
    value
        .as_object_mut()
        .ok_or("object")?
        .insert(key.into(), replacement);
    Ok(())
}
