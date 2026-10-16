use super::*;
use crate::export::public_identity::tests::fixture::{capture, source_births, TestResult};
use serde_json::json;

#[test]
fn resolution_wire_preserves_each_domain_state_with_real_birth_successors() -> TestResult {
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [first_athlete, second_athlete, _] = &fixture.athletes;
    let PublicResolution::Canonical(first) = view
        .candidate_resolution(first_athlete.id.as_str())
        .ok_or("first birth")?
        .clone()
    else {
        return Err("first source birth is not canonical".into());
    };
    let PublicResolution::Canonical(second) = view
        .candidate_resolution(second_athlete.id.as_str())
        .ok_or("second birth")?
        .clone()
    else {
        return Err("second source birth is not canonical".into());
    };
    let first_wire = json!({
        "public_id": first.public_id, "representative_candidate": first_athlete.id,
        "established": { "sequence": 1, "position": 0 },
    });
    let second_wire = json!({
        "public_id": second.public_id, "representative_candidate": second_athlete.id,
        "established": { "sequence": 2, "position": 0 },
    });
    let cases = [
        (PublicResolution::Unknown, json!({"state":"unknown"})),
        (PublicResolution::Unresolved, json!({"state":"unresolved"})),
        (
            PublicResolution::Canonical(first.clone()),
            json!({"state":"canonical","successor":first_wire}),
        ),
        (
            PublicResolution::Redirect {
                successor: first.clone(),
                changed_at: 3,
            },
            json!({"state":"redirect","successor":first_wire,"changed_at":3}),
        ),
        (
            PublicResolution::RevokedRedirect {
                successor: first.clone(),
                changed_at: 4,
                revoked_merges: vec![3],
            },
            json!({"state":"revoked_redirect","successor":first_wire,"changed_at":4,"revoked_merges":[3]}),
        ),
        (
            PublicResolution::Ambiguous {
                successors: vec![first, second],
                unresolved_bindings: true,
                changed_at: 5,
                revoked_merges: vec![3, 4],
            },
            json!({"state":"ambiguous","successors":[first_wire,second_wire],"unresolved_bindings":true,"changed_at":5,"revoked_merges":[3,4]}),
        ),
    ];
    for (resolution, expected) in cases {
        check!(eq; serde_json::to_value(ResolutionAudit(&resolution))?, expected);
    }
    Ok(())
}
