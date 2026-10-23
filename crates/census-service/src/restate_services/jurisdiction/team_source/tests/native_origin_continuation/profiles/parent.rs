use super::{public_resources_fixture, transport, Fixture, NativeNode, TestResult};
use crate::census::StateProgress;
use crate::restate_services::jurisdiction::team_collection;
use crate::restate_services::wire::{JurisdictionState, SourcePlan};
use census_crawl::CollectionDisposition;
use census_reconcile::identity::WorkflowIdentity;
use std::time::Duration;

pub(super) async fn seed(
    fixture: &Fixture,
    client: &reqwest::Client,
    node: &NativeNode,
) -> TestResult<(String, JurisdictionState)> {
    let request = &fixture.request.jurisdiction;
    let key =
        WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision)
            .to_string();
    let mut teams = team_collection::empty_outcome(
        &fixture.request.observed_on,
        CollectionDisposition::Complete,
    );
    teams.records = 1;
    let state = JurisdictionState {
        identity: key.clone(),
        history_window: Some(request.history),
        plan: Some(SourcePlan {
            sweepable: vec!["milesplit".into()],
            fingerprint: public_resources_fixture::fingerprint(&fixture.request),
            ..SourcePlan::default()
        }),
        teams: team_collection::finish(teams, Vec::new(), &fixture.request.observed_on),
        rosters: Some(settled(fixture)),
        ..JurisdictionState::default()
    };
    replace(client, node, &key, &state, None).await?;
    Ok((key, state))
}

pub(super) async fn replace(
    client: &reqwest::Client,
    node: &NativeNode,
    key: &str,
    state: &JurisdictionState,
    progress: Option<&[u8]>,
) -> TestResult {
    let mut new_state = serde_json::json!({"state": serde_json::to_vec(state)?});
    if let Some(progress) = progress {
        new_state["public_profile_collection_v1"] = serde_json::to_value(progress)?;
    }
    let response = client
        .post(format!("{}/services/JurisdictionCensus/state", node.admin))
        .json(&serde_json::json!({
            "object_key": key, "scope": null, "version": null,
            "new_state": new_state
        }))
        .send()
        .await?;
    let status = response.status();
    let detail = response.text().await?;
    check!(eq; status, reqwest::StatusCode::ACCEPTED,
        "isolated controlled profile parent seed refused: {detail}");
    await_seed(client, node, &key, &state).await?;
    Ok(())
}

fn settled(fixture: &Fixture) -> StateProgress {
    StateProgress {
        jurisdiction: fixture.request.jurisdiction.jurisdiction,
        rosters_total: 1,
        rosters_committed: 1,
        rosters_remaining: 0,
        rosters_skipped: 0,
        athletes: 0,
        class_of_2027: 0,
        class_of_2027_boys: 0,
        class_of_2027_girls: 0,
        errors: Vec::new(),
        blocked: false,
        blocked_skipped: 0,
        teams: 1,
    }
}

async fn await_seed(
    client: &reqwest::Client,
    node: &NativeNode,
    key: &str,
    expected: &JurisdictionState,
) -> TestResult {
    let expected = serde_json::to_value(expected)?;
    let digest = census_domain::model::serialized_digest(&expected)?;
    for attempt in 0..40 {
        let reply = transport::call_empty(
            client,
            &node.ingress,
            "JurisdictionCensus",
            key,
            "state",
            &format!("profile-parent-seed-{digest}-{attempt}"),
        )
        .await?;
        let state: JurisdictionState = serde_json::from_slice(&reply.body)?;
        if serde_json::to_value(state)? == expected {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    Err("controlled profile parent did not settle within forty reads".into())
}
