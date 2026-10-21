use crate::restate_services::national::publication_tests::native_restate::{
    endpoint, node::NativeNode,
};
use evidence::Evidence;
use fixture::{Effects, OldOwner};
use restate_sdk::prelude::Endpoint;
use std::sync::Arc;
use std::time::Duration;

mod admin;
mod deployment;
mod endpoint_owner;
mod evidence;
mod exercise;
mod fixture;
mod journal;
mod settlement;
mod transport;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const SERVICE: &str = "NativeDeploymentHandoff";
const KEY: &str = "same-retained-key-20261021";

#[ignore = "explicit isolated native Restate 1.7.10 deployment handoff qualification"]
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn native_pause_same_deployment_rediscovery_resumes_original_journal() -> TestResult {
    let evidence = Evidence::new()?;
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(10))
        .build()?;
    let discovery = reqwest::Client::builder()
        .no_proxy()
        .http2_prior_knowledge()
        .timeout(Duration::from_secs(5))
        .build()?;
    let old_effects = Arc::new(Effects::default());
    let new_effects = Arc::new(Effects::default());
    let mut node = NativeNode::new()?;
    let mut old = endpoint_owner::EndpointOwner::start(
        "127.0.0.1:0".parse()?,
        Endpoint::builder()
            .bind(OldOwner {
                effects: Arc::clone(&old_effects),
            })
            .build(),
    )
    .await?;
    let mut replacement = None;
    let assertions = tokio::time::timeout(
        Duration::from_secs(180),
        exercise::Exercise {
            client: &client,
            discovery: &discovery,
            node: &mut node,
            old: &mut old,
            replacement: &mut replacement,
            old_effects: &old_effects,
            new_effects: &new_effects,
            evidence: &evidence,
        }
        .run(),
    )
    .await;
    let old_stopped = old.stop().await;
    let new_stopped = match replacement.as_mut() {
        Some(owner) => owner.stop().await.map(Some),
        None => Ok(None),
    };
    let node_stopped = node.stop().await;
    evidence.write(
        "cleanup.json",
        &serde_json::json!({
            "old_endpoint": format!("{old_stopped:?}"), "new_endpoint": format!("{new_stopped:?}"),
            "native_node": format!("{node_stopped:?}")
        }),
    )?;
    evidence.write(
        "assertions.json",
        &serde_json::json!({"outcome": format!("{assertions:?}")}),
    )?;
    assertions??;
    old_stopped?;
    new_stopped?;
    node_stopped?;
    evidence.write(
        "qualified.json",
        &serde_json::json!({
            "same_invocation": true, "same_deployment": true, "same_key": KEY,
            "service_protocol": 7, "old_run_effects": 1, "replacement_run_effects": 0,
            "native_restart_output_exact": true
        }),
    )?;
    Ok(())
}
