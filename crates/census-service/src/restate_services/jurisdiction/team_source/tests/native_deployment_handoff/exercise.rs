use super::endpoint_owner::EndpointOwner;
use super::fixture::{Effects, NewOwner, Retained};
use super::{
    admin, deployment, endpoint, journal, transport, Evidence, NativeNode, TestResult, KEY, SERVICE,
};
use restate_sdk::prelude::Endpoint;
use serde_json::{json, Value};
use std::sync::atomic::Ordering;
use std::sync::Arc;

pub(super) struct Exercise<'a> {
    pub(super) client: &'a reqwest::Client,
    pub(super) discovery: &'a reqwest::Client,
    pub(super) node: &'a mut NativeNode,
    pub(super) old: &'a mut EndpointOwner,
    pub(super) replacement: &'a mut Option<EndpointOwner>,
    pub(super) old_effects: &'a Arc<Effects>,
    pub(super) new_effects: &'a Arc<Effects>,
    pub(super) evidence: &'a Evidence,
}

pub(super) struct Invocation {
    pub(super) id: String,
    pub(super) deployment: String,
    pub(super) prefix: Vec<Value>,
    pub(super) input: Retained,
    pub(super) bytes: Vec<u8>,
}

impl Exercise<'_> {
    pub(super) async fn run(&mut self) -> TestResult {
        self.node.start()?;
        self.node.wait_ready(self.client).await?;
        let old_manifest = deployment::manifest(self.discovery, self.old.address).await?;
        self.evidence
            .write("manifest-original.json", &old_manifest)?;
        endpoint::register_named_endpoint(self.client, self.node, self.old.address, SERVICE, "run")
            .await?;
        let pinned = deployment::only_id(self.client, self.node).await?;
        let original_deployment = deployment::read(self.client, self.node, &pinned).await?;
        self.evidence
            .write("deployment-original.json", &original_deployment)?;
        let input = Retained {
            token: "original-retained-value".into(),
            sequence: 20261021,
        };
        let bytes = serde_json::to_vec(&input)?;
        self.evidence.write("request.json", &input)?;
        let id = transport::send(self.client, &self.node.ingress, KEY, &bytes).await?;
        let prefix = admin::wait_sleep(self.client, self.node, &id).await?;
        journal::require_sleep_prefix(&prefix, &bytes)?;
        self.evidence.write("journal-before-pause.json", &prefix)?;
        let invocation = Invocation {
            id,
            deployment: pinned,
            prefix,
            input,
            bytes,
        };
        self.pause(&invocation).await?;
        self.replace(&invocation, &old_manifest, &original_deployment)
            .await?;
        self.finish(&invocation).await
    }

    async fn pause(&self, invocation: &Invocation) -> TestResult {
        let running = admin::invocation(self.client, self.node, &invocation.id).await?;
        check!(eq; running["status"].as_str(), Some("running"));
        deployment::require_pin(&running, &invocation.deployment, KEY)?;
        self.evidence
            .write("invocation-before-pause.json", &running)?;
        self.effects_unchanged()?;
        admin::control(
            self.client,
            self.node,
            &invocation.id,
            "pause",
            self.evidence,
        )
        .await?;
        let paused = admin::wait_status(self.client, self.node, &invocation.id, "paused").await?;
        deployment::require_pin(&paused, &invocation.deployment, KEY)?;
        self.evidence.write("invocation-paused.json", &paused)?;
        let journal = admin::journal(self.client, self.node, &invocation.id).await?;
        admin::prefix(&journal, &invocation.prefix)?;
        self.evidence.write("journal-paused.json", &journal)?;
        self.effects_unchanged()
    }

    async fn replace(
        &mut self,
        invocation: &Invocation,
        old_manifest: &Value,
        original: &Value,
    ) -> TestResult {
        let drained = self.old.stop().await?;
        self.evidence.write(
            "old-owner-drained.json",
            &json!({"report": format!("{drained:?}")}),
        )?;
        check!(eq; self.old_effects.active.load(Ordering::SeqCst), 0);
        *self.replacement = Some(
            EndpointOwner::start(
                self.old.address,
                Endpoint::builder()
                    .bind(NewOwner {
                        effects: Arc::clone(self.new_effects),
                    })
                    .build(),
            )
            .await?,
        );
        let new_manifest = deployment::manifest(self.discovery, self.old.address).await?;
        deployment::require_additive(old_manifest, &new_manifest)?;
        self.evidence
            .write("manifest-replacement.json", &new_manifest)?;
        let dry_run = deployment::update(
            self.client,
            self.node,
            &invocation.deployment,
            self.old.address,
            true,
            self.evidence,
        )
        .await?;
        deployment::require_rediscovered(original, &dry_run)?;
        check!(eq; deployment::read(self.client, self.node, &invocation.deployment).await?, original.clone());
        let updated = deployment::update(
            self.client,
            self.node,
            &invocation.deployment,
            self.old.address,
            false,
            self.evidence,
        )
        .await?;
        deployment::require_rediscovered(original, &updated)?;
        let persisted = deployment::read(self.client, self.node, &invocation.deployment).await?;
        check!(eq; persisted, updated);
        self.evidence
            .write("deployment-replacement.json", &persisted)?;
        check!(eq; deployment::only_id(self.client, self.node).await?, invocation.deployment);
        let paused = admin::invocation(self.client, self.node, &invocation.id).await?;
        check!(eq; paused["status"].as_str(), Some("paused"));
        deployment::require_pin(&paused, &invocation.deployment, KEY)?;
        let journal = admin::journal(self.client, self.node, &invocation.id).await?;
        admin::prefix(&journal, &invocation.prefix)?;
        self.evidence
            .write("journal-after-rediscovery.json", &journal)?;
        self.effects_unchanged()
    }

    pub(super) fn effects_unchanged(&self) -> TestResult {
        check!(eq; self.old_effects.runs.load(Ordering::SeqCst), 1);
        check!(eq; self.new_effects.runs.load(Ordering::SeqCst), 0);
        Ok(())
    }
}
