use super::{
    fixture::Fixture,
    transport::{call, call_empty, call_status, Reply},
    NativeNode, TestResult,
};
use crate::restate_services::jurisdiction::team_collection;
use crate::restate_services::wire::{
    JurisdictionState, SourcePlan, TeamsReconciliationRequest, TeamsSourceOutcome, TeamsStage,
};
use census_crawl::CollectionDisposition;
use census_domain::model::serialized_digest;
use census_reconcile::identity::WorkflowIdentity;
use std::time::Duration;

pub(super) struct Parent {
    key: String,
    state: JurisdictionState,
    request: TeamsReconciliationRequest,
}

impl Parent {
    pub(super) async fn seed(
        fixture: &Fixture,
        client: &reqwest::Client,
        node: &NativeNode,
        original: &TeamsSourceOutcome,
    ) -> TestResult<Self> {
        let request = &fixture.request;
        let key = WorkflowIdentity::jurisdiction(
            request.jurisdiction.jurisdiction,
            request.jurisdiction.season,
            request.jurisdiction.revision,
        )
        .to_string();
        let mut outcome =
            team_collection::empty_outcome(&request.observed_on, CollectionDisposition::Complete);
        let mut failures = Vec::new();
        team_collection::retain("milesplit", original.clone(), &mut outcome, &mut failures)?;
        let state = JurisdictionState {
            identity: key.clone(),
            history_window: Some(request.jurisdiction.history),
            plan: Some(SourcePlan {
                sweepable: vec!["milesplit".into()],
                fingerprint: super::super::public_resources_fixture::fingerprint(request),
                ..SourcePlan::default()
            }),
            teams: team_collection::finish(outcome, failures, &request.observed_on),
            ..JurisdictionState::default()
        };
        let reconciliation = TeamsReconciliationRequest {
            jurisdiction: request.jurisdiction.clone(),
            expected_teams_digest: serialized_digest(&state.teams)?,
            expected_plan_digest: serialized_digest(&state.plan)?,
        };
        let response = client
            .post(format!("{}/services/JurisdictionCensus/state", node.admin))
            .json(&serde_json::json!({
                "object_key": key, "scope": null, "version": null,
                "new_state": { "state": serde_json::to_vec(&state)? }
            }))
            .send()
            .await?;
        let status = response.status();
        let detail = response.text().await?;
        check!(eq; status, reqwest::StatusCode::ACCEPTED,
            "isolated native parent state seed refused: {detail}");
        let parent = Self {
            key,
            state,
            request: reconciliation,
        };
        parent.await_seed(client, node).await?;
        Ok(parent)
    }

    async fn await_seed(&self, client: &reqwest::Client, node: &NativeNode) -> TestResult {
        let expected = serde_json::to_value(&self.state)?;
        for attempt in 0..40 {
            let state = self
                .state(client, node, &format!("seed-state-{attempt}"))
                .await?;
            if serde_json::to_value(state)? == expected {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Err("isolated native parent state was not applied within forty reads".into())
    }

    pub(super) async fn reconcile(
        &self,
        client: &reqwest::Client,
        node: &NativeNode,
    ) -> TestResult<Reply> {
        let reply = self.call(client, node).await?;
        let teams: TeamsStage = serde_json::from_slice(&reply.body)?;
        check!(matches!(&teams, TeamsStage::Completed(completed)
            if completed.outcome().records == 1 && completed.outcome().disposition.is_complete()
                && completed.outcome().errors.is_empty() && completed.outcome().unfinished.is_empty()));
        self.assert_parent(client, node, &reply, "parent-after-reconciliation")
            .await?;
        Ok(reply)
    }

    pub(super) async fn assert_unsettled_refused(
        &self,
        client: &reqwest::Client,
        node: &NativeNode,
    ) -> TestResult {
        let (status, reply) = call_status(
            client,
            &node.ingress,
            "JurisdictionCensus",
            &self.key,
            "reconcile_teams",
            "refuse-before-continuation",
            &self.request,
        )
        .await?;
        check!(status.is_client_error() || status.is_server_error());
        check!(String::from_utf8_lossy(&reply.body)
            .contains("teams reconciliation requires a settled authorized continuation"));
        let state = self
            .state(client, node, "parent-after-unsettled-refusal")
            .await?;
        check!(eq; serde_json::to_value(state)?, serde_json::to_value(&self.state)?);
        Ok(())
    }

    pub(super) async fn assert_replayed(
        &self,
        client: &reqwest::Client,
        node: &NativeNode,
        expected: &Reply,
    ) -> TestResult {
        let reply = self.call(client, node).await?;
        check!(eq; reply.invocation, expected.invocation);
        check!(eq; reply.body, expected.body);
        self.assert_parent(client, node, expected, "parent-after-restart")
            .await
    }

    async fn call(&self, client: &reqwest::Client, node: &NativeNode) -> TestResult<Reply> {
        call(
            client,
            &node.ingress,
            "JurisdictionCensus",
            &self.key,
            "reconcile_teams",
            "reconcile-original-failure",
            &self.request,
        )
        .await
    }

    async fn state(
        &self,
        client: &reqwest::Client,
        node: &NativeNode,
        idempotency: &str,
    ) -> TestResult<JurisdictionState> {
        let reply = call_empty(
            client,
            &node.ingress,
            "JurisdictionCensus",
            &self.key,
            "state",
            idempotency,
        )
        .await?;
        Ok(serde_json::from_slice(&reply.body)?)
    }

    async fn assert_parent(
        &self,
        client: &reqwest::Client,
        node: &NativeNode,
        expected: &Reply,
        idempotency: &str,
    ) -> TestResult {
        let state = self.state(client, node, idempotency).await?;
        check!(eq; state.identity, self.state.identity);
        check!(eq; state.history_window, self.state.history_window);
        check!(eq; state.plan, self.state.plan);
        check!(state.rosters.is_none() && state.consolidated.is_none());
        check!(eq; serde_json::to_value(&state.teams)?, serde_json::from_slice::<serde_json::Value>(&expected.body)?);
        Ok(())
    }
}
