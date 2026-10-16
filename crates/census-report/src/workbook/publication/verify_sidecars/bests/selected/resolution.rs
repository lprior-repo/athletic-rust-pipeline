use census_domain::public_identity_history::{PublicResolution, ResolvedSuccessor};
use serde_json::{json, Value};

pub(super) fn expected(resolution: &PublicResolution) -> Value {
    match resolution {
        PublicResolution::Unknown => json!({ "state": "unknown" }),
        PublicResolution::Unresolved => json!({ "state": "unresolved" }),
        PublicResolution::Canonical(value) => {
            json!({ "state": "canonical", "successor": successor(value) })
        }
        PublicResolution::Redirect {
            successor: value,
            changed_at,
        } => json!({
            "state": "redirect", "successor": successor(value), "changed_at": changed_at,
        }),
        PublicResolution::RevokedRedirect {
            successor: value,
            changed_at,
            revoked_merges,
        } => json!({
            "state": "revoked_redirect", "successor": successor(value),
            "changed_at": changed_at, "revoked_merges": revoked_merges,
        }),
        PublicResolution::Ambiguous {
            successors,
            unresolved_bindings,
            changed_at,
            revoked_merges,
        } => json!({
            "state": "ambiguous", "successors": successors.iter().map(successor).collect::<Vec<_>>(),
            "unresolved_bindings": unresolved_bindings, "changed_at": changed_at,
            "revoked_merges": revoked_merges,
        }),
    }
}

fn successor(value: &ResolvedSuccessor) -> Value {
    json!({
        "public_id": value.public_id.as_str(),
        "representative_candidate": value.representative_candidate.as_str(),
        "established": { "sequence": value.established.sequence, "position": value.established.position },
    })
}
