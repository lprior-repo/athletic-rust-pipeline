use super::*;
use census_domain::model::SourceNamespace;
use census_domain::public_identity_history::{EffectiveScope, SourceObjectKey};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn tagged_group_wire_preserves_every_binding_and_membership_field_in_order() -> TestResult {
    let binding = BindingClaim {
        id: "a-binding".into(),
        source: SourceObjectKey {
            namespace: SourceNamespace::MilesplitAthlete,
            identifier: "controlled-source-1".into(),
        },
        candidate: AthleteCandidateId::mint("ath", &["controlled first"]),
        effective: EffectiveScope::Unknown,
        supersedes: Some("earlier-binding".into()),
    };
    let membership = CandidateMembershipClaimV1 {
        id: "b-membership".into(),
        candidate: AthleteCandidateId::mint("ath", &["controlled second"]),
        anchor_binding_id: binding.id.clone(),
        source: SourceObjectKey {
            namespace: SourceNamespace::MilesplitAthlete,
            identifier: "controlled-source-2".into(),
        },
    };
    let claims = [Claim::Binding(&binding), Claim::Membership(&membership)];
    let store = StoreIdentity::parse(&"1".repeat(64))?;
    let public = PublicAthleteId::parse(&format!("ath_public_v1_{}", "2".repeat(64)))?;
    let group = Group {
        schema: "retained-evidence-subject-attachment-group/v1",
        store: &store,
        qualification: "retained_authority_graph_v3",
        public_id: &public,
        representative: &binding.candidate,
        established: EstablishmentOrdinal {
            sequence: 1,
            position: 0,
        },
        claims: &claims,
    };
    let mut grant = AttachmentReplayGrantV1::new();
    let object = encoding::object(&group, &mut grant)?;
    let wire: serde_json::Value = serde_json::from_slice(object.bytes())?;
    check!(eq; wire.get("claims"), Some(&serde_json::json!([
        {"kind":"binding","claim":binding},
        {"kind":"membership","claim":membership}
    ])));
    check!(eq; grant.usage().traversal_steps(), 3);
    Ok(())
}
