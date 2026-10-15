use census_domain::public_identity_history::{BindingResolution, PublicResolution};
use census_store::{
    AcceptedPublicIdentitySnapshot, FrozenPublicIdentityBundleV1, LocalAdjudicationCommitOutcomeV1,
    PublicIdentityError, StoreError,
};
use sha2::{Digest, Sha256};

use super::fixture::Plan;
use super::TestResult;

pub(super) fn merged(plan: &Plan, snapshot: &AcceptedPublicIdentitySnapshot) -> TestResult {
    check!(eq; snapshot.head(), &plan.head);
    check!(eq; snapshot.public_ids().count(), 3);
    let [canonical, redirected, unrelated] = &plan.public_ids;
    check!(
        matches!(snapshot.resolve_public(canonical)?, PublicResolution::Canonical(value)
        if &value.public_id == canonical)
    );
    check!(
        matches!(snapshot.resolve_public(redirected)?, PublicResolution::Redirect { successor, .. }
        if &successor.public_id == canonical)
    );
    check!(
        matches!(snapshot.resolve_public(unrelated)?, PublicResolution::Canonical(value)
        if &value.public_id == unrelated)
    );
    for (candidate, public_id) in plan.candidates.iter().zip(&plan.public_ids) {
        check!(eq; snapshot.resolve_candidate(candidate)?, snapshot.resolve_public(public_id)?);
    }
    Ok(())
}

pub(super) fn split(
    plan: &Plan,
    snapshot: &AcceptedPublicIdentitySnapshot,
    outcome: &LocalAdjudicationCommitOutcomeV1,
) -> TestResult {
    check!(eq; snapshot.head(), outcome.head());
    check!(eq; snapshot.head().sequence, 5);
    check!(eq; outcome.introduced_public_ids().len(), 2);
    let mut expected_ids = plan
        .public_ids
        .iter()
        .chain(outcome.introduced_public_ids())
        .cloned()
        .collect::<Vec<_>>();
    expected_ids.sort();
    check!(eq; snapshot.public_ids().cloned().collect::<Vec<_>>(), expected_ids);
    let [canonical, redirected, unrelated] = &plan.public_ids;
    let PublicResolution::Ambiguous {
        successors,
        unresolved_bindings,
        changed_at,
        revoked_merges,
    } = snapshot.resolve_public(canonical)?
    else {
        return Err("old canonical must retain both split successors".into());
    };
    let mut introduced = outcome.introduced_public_ids().to_vec();
    introduced.sort();
    check!(eq; successors.iter().map(|row| row.public_id.clone()).collect::<Vec<_>>(), introduced);
    check!(!unresolved_bindings);
    check!(eq; changed_at, 5);
    check!(eq; revoked_merges, vec![4]);
    let second = snapshot.resolve_candidate(plan.candidates.get(1).ok_or("second candidate")?)?;
    let PublicResolution::Canonical(second) = second else {
        return Err("split candidate unresolved".into());
    };
    check!(
        matches!(snapshot.resolve_public(redirected)?, PublicResolution::RevokedRedirect {
        successor, revoked_merges, changed_at,
    } if successor == second && revoked_merges == vec![4] && changed_at == 5)
    );
    check!(
        matches!(snapshot.resolve_public(unrelated)?, PublicResolution::Canonical(value)
        if &value.public_id == unrelated)
    );
    for id in outcome.introduced_public_ids() {
        let PublicResolution::Canonical(value) = snapshot.resolve_public(id)? else {
            return Err("new public ID must be canonical".into());
        };
        check!(eq; &value.public_id, id);
        check!(eq; snapshot.resolve_candidate(&value.representative_candidate)?, PublicResolution::Canonical(value));
    }
    for partition in &plan.request.partitions {
        let resolution = snapshot.resolve_candidate(&partition.representative)?;
        check!(matches!(&resolution, PublicResolution::Canonical(value)
            if outcome.introduced_public_ids().contains(&value.public_id)));
        for binding in &partition.binding_ids {
            check!(
                matches!(snapshot.resolve_binding(binding)?, BindingResolution::Resolved {
                identity, superseded_by: None, ..
            } if identity == resolution)
            );
        }
    }
    check!(eq; snapshot.affected_record_keys(5), Some(plan.request.affected_record_keys.as_slice()));
    Ok(())
}

pub(super) fn frozen(
    plan: &Plan,
    bundle: &FrozenPublicIdentityBundleV1,
    expected: &AcceptedPublicIdentitySnapshot,
    outcome: &LocalAdjudicationCommitOutcomeV1,
) -> TestResult {
    let replayed = bundle.replay()?;
    split(plan, &replayed, outcome)?;
    check!(eq; replayed.store_identity(), expected.store_identity());
    check!(eq; replayed.history_records(), expected.history_records());
    for id in expected.public_ids() {
        check!(eq; replayed.resolve_public(id)?, expected.resolve_public(id)?);
    }
    for candidate in &plan.candidates {
        check!(eq; replayed.resolve_candidate(candidate)?, expected.resolve_candidate(candidate)?);
    }
    reject_fake_accepted_decision(bundle)
}

fn reject_fake_accepted_decision(bundle: &FrozenPublicIdentityBundleV1) -> TestResult {
    let mut wire: serde_json::Value = serde_json::from_slice(bundle.bytes())?;
    let operations = wire
        .get_mut("operations")
        .and_then(serde_json::Value::as_array_mut)
        .ok_or("frozen operations")?;
    let retained_decision = operations
        .first()
        .and_then(|row| row.get("decision"))
        .ok_or("source accepted decision")?
        .clone();
    let manual = operations
        .last_mut()
        .and_then(serde_json::Value::as_object_mut)
        .ok_or("manual operation")?;
    check!(eq; manual.get("sequence"), Some(&serde_json::json!(5)));
    check!(!manual.contains_key("decision"));
    check!(manual
        .insert("decision".into(), retained_decision)
        .is_none());
    let changed = serde_json::to_vec(&wire)?;
    let digest = format!("{:x}", Sha256::digest(&changed));
    check!(matches!(
        census_store::replay_frozen_public_identity_v1(&changed, &digest),
        Err(StoreError::PublicIdentity(PublicIdentityError::Corrupt(
            "frozen decision material kind"
        )))
    ));
    Ok(())
}
