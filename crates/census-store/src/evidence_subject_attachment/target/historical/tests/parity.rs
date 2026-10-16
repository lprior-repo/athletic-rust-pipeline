use super::*;
use crate::evidence_subject_attachment::target::group::{Claim, Group as IssuedGroup};
use census_domain::model::SourceNamespace;
use census_domain::public_identity_history::{EffectiveScope, SourceObjectKey};

#[test]
fn actual_group_encoder_preserves_borrowed_binding_and_membership_claims() -> TestResult {
    let binding = BindingClaim {
        id: "a-binding-\"é\\雪\"".into(),
        source: SourceObjectKey {
            namespace: SourceNamespace::MilesplitAthlete,
            identifier: "controlled-1".into(),
        },
        candidate: AthleteCandidateId::mint("ath", &["serialization first"]),
        effective: EffectiveScope::Unknown,
        supersedes: Some("prior-\"quote\"".into()),
    };
    let membership = CandidateMembershipClaimV1 {
        id: "b-membership".into(),
        candidate: AthleteCandidateId::mint("ath", &["serialization second"]),
        anchor_binding_id: binding.id.clone(),
        source: SourceObjectKey {
            namespace: SourceNamespace::MilesplitAthlete,
            identifier: "controlled-2".into(),
        },
    };
    let claims = [Claim::Binding(&binding), Claim::Membership(&membership)];
    let store = StoreIdentity::parse(&"1".repeat(64))?;
    let public = PublicAthleteId::parse(&format!("ath_public_v1_{}", "2".repeat(64)))?;
    let wire = IssuedGroup {
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
    let mut writer_grant = AttachmentReplayGrantV1::new();
    let issued = encoding::object(&wire, &mut writer_grant)?;
    let decoded: Group<'_> = serde_json::from_slice(issued.bytes())?;
    let mut read_grant = AttachmentReplayGrantV1::new();
    encoding::require_canonical(&decoded, issued.bytes(), &mut read_grant)?;
    check!(eq; serde_json::to_vec(&decoded)?, issued.bytes());
    for (actual, expected) in decoded.claims.iter().zip(&claims) {
        check!(eq; actual.get().as_bytes(), census_domain::model::serialized_bytes(expected)?);
    }
    check!(eq; decoded.claims.len(), claims.len());
    check!(eq; read_grant.usage().traversal_steps(), 1);
    check!(eq; read_grant.usage().encoded_bytes(), 0);
    Ok(())
}
