use super::*;
use crate::export::public_identity::output::AcceptedIdentityFields;
use census_store::{StoreError, StoreResult};

#[test]
fn selected_fields_publish_genuine_birth_and_audit_every_unknown_without_fallback() -> TestResult {
    let fixture = source_births()?;
    let (view, _) = capture(&fixture.store)?;
    let [first, second, unknown] = &fixture.athletes;
    let [birth, _] = &fixture.births;
    let fields = AcceptedIdentityFields::for_candidate(&view, first.id.as_str())?;
    check!(eq; fields.athlete_id(), birth.public_id());
    check!(eq; fields.canonical_candidate_id().as_str(), first.id.as_str());
    check!(eq; Some(fields.resolution()), view.candidate_resolution(first.id.as_str()));
    refusal(
        AcceptedIdentityFields::for_candidate(&view, unknown.id.as_str()),
        "selected identity has no unambiguous retained public athlete ID",
    )?;
    refusal(
        AcceptedIdentityFields::for_candidate(&view, "absent"),
        "selected identity candidate is absent from the retained public identity inventory",
    )?;
    let audit = view
        .identity_audit()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    check!(eq; audit.len(), 3);
    for athlete in [first, second, unknown] {
        let found = audit
            .iter()
            .find(|entry| {
                entry.get("original_candidate_id")
                    == Some(&serde_json::Value::String(athlete.id.to_string()))
            })
            .ok_or("candidate audit")?;
        check!(eq; found.get("canonical_candidate_id"), Some(&serde_json::json!(athlete.id)));
        let expected_state = if athlete.id == unknown.id {
            "unknown"
        } else {
            "canonical"
        };
        check!(eq; found.get("resolution").and_then(|resolution| resolution.get("state")), Some(&serde_json::json!(expected_state)));
    }
    let unresolved = audit
        .iter()
        .find(|entry| {
            entry.get("original_candidate_id")
                == Some(&serde_json::Value::String(unknown.id.to_string()))
        })
        .ok_or("unknown audit")?;
    check!(eq; unresolved.get("public_athlete_id"), Some(&serde_json::Value::Null));
    check!(eq; unresolved.get("publication_identity_status"), Some(&serde_json::json!("unknown")));
    Ok(())
}

#[test]
fn distinct_committed_public_identities_ignore_an_uncommitted_automatic_alias() -> TestResult {
    let fixture = source_births()?;
    prepare_merge(&fixture)?;
    let snapshot = fixture.store.snapshot();
    let lineage = store_lineage(&fixture.store)?;
    let accepted = snapshot
        .accepted_public_identity_snapshot()?
        .ok_or("accepted ledger")?;
    let head = accepted.head().clone();
    let view = VerifiedPublicIdentityView::capture(
        accepted,
        &lineage,
        &head,
        &snapshot.athletes()?,
        &accepted_review_projection(&snapshot)?,
    )?;
    let [first, second, _] = &fixture.athletes;
    for (candidate, birth) in [first, second].into_iter().zip(&fixture.births) {
        let fields = AcceptedIdentityFields::for_candidate(&view, candidate.id.as_str())?;
        check!(eq; fields.athlete_id(), birth.public_id());
        check!(eq; fields.canonical_candidate_id().as_str(), candidate.id.as_str());
    }
    let audit = view
        .identity_audit()
        .map(serde_json::to_value)
        .collect::<Result<Vec<_>, _>>()?;
    let withheld = audit
        .iter()
        .filter(|entry| {
            entry.get("publication_identity_status")
                == Some(&serde_json::json!("candidate_group_unresolved"))
        })
        .collect::<Vec<_>>();
    check!(withheld.is_empty());
    Ok(())
}

fn refusal<T>(result: StoreResult<T>, detail: &str) -> TestResult {
    match result {
        Err(StoreError::Invariant { detail: actual }) => check!(eq; actual, detail),
        Err(error) => return Err(format!("unexpected selected identity failure: {error}").into()),
        Ok(_) => return Err("withheld selected identity was accepted".into()),
    }
    Ok(())
}
