use super::super::super::{artifacts, GUEST};
use super::super::http::{query, request, rows, INGRESS};
use super::captures::{self, CaptureRef};
use super::injection;
use anyhow::{ensure, Context, Result};
use census_domain::{model::SchoolYear, UsJurisdiction};
use census_reconcile::identity::{Revision, WorkflowIdentity};
use census_service::restate_services::{plan, BrowserLaneState, JurisdictionRequest, SourcePlan};
use reqwest::{Client, Method, Url};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub(super) struct Original {
    pub(super) id: String,
    pub(super) key: String,
    pub(super) request: JurisdictionRequest,
    pub(super) accepted: Value,
    pub(super) clock: Value,
    pub(super) manifest: Value,
    pub(super) deployment: Value,
    pub(super) owner: Value,
    pub(super) source_plan: SourcePlan,
    pub(super) captures_before: Vec<CaptureRef>,
}

#[tracing::instrument(skip(client))]
pub(super) async fn submit(client: &Client) -> Result<Original> {
    let root = Path::new(GUEST);
    ensure!(!root.join("jurisdiction-recovery-intent.json").exists(), "original submission already attempted; refuse replacement even after uncertain acknowledgement");
    ensure!(
        !root.join("jurisdiction-recovery-original.json").exists(),
        "original must be reattached, never resubmitted"
    );
    let manifest = artifacts::json(&root.join("manifest.json"))?;
    let request_body = qualification_request(&manifest)?;
    let key = identity(&request_body);
    let existing = query(client, &format!("SELECT id FROM sys_invocation WHERE target_service_name = 'JurisdictionCensus' AND target_service_key = '{key}' LIMIT 2")).await?;
    ensure!(
        rows(&existing)?.is_empty(),
        "jurisdiction identity already used; fresh run required"
    );
    let clock = super::super::clock::clock()?;
    let source_plan = SourcePlan::of(
        &plan(request_body.jurisdiction, BrowserLaneState::Absent),
        String::new(),
    );
    ensure!(
        source_plan.sweepable.iter().any(|slug| slug == "milesplit")
            && source_plan.sweepable.iter().any(|slug| slug == "riil"),
        "qualification source arms unavailable"
    );
    let captures_before = captures::snapshot(key.clone()).await?;
    let deployment = artifacts::json(&root.join("deployment.json"))?;
    let owner = artifacts::json(&root.join("current-owner.json"))?;
    let native_source_boundary = injection::prepare(&request_body, &source_plan)?;
    let intent = json!({"key":key,"request":request_body,"clock":clock,"manifest":manifest,"source_plan":source_plan,"captures_before":captures_before,"deployment":deployment,"owner":owner,"native_source_boundary":native_source_boundary});
    artifacts::write(
        &root.join("jurisdiction-recovery-intent.json"),
        &serde_json::to_vec_pretty(&intent)?,
    )?;
    let accepted = request(
        client,
        Method::POST,
        &target("send", "JurisdictionCensus", &key, "run")?,
        Some(&serde_json::to_value(&request_body)?),
    )
    .await?;
    let id = text(&accepted, "invocationId")?.to_owned();
    safe_id(&id)?;
    let original = Original {
        id,
        key,
        request: request_body,
        accepted,
        clock,
        manifest,
        source_plan,
        captures_before,
        deployment,
        owner,
    };
    artifacts::publish(&root.join("jurisdiction-recovery-original.json"), &original)?;
    Ok(original)
}

fn qualification_request(manifest: &Value) -> Result<JurisdictionRequest> {
    let season = i16::try_from(
        manifest
            .get("season")
            .and_then(Value::as_i64)
            .context("manifest season absent")?,
    )?;
    let revision = u32::try_from(
        manifest
            .get("revision")
            .and_then(Value::as_u64)
            .context("manifest revision absent")?,
    )?;
    ensure!(revision > 0, "run revision must be nonzero");
    ensure!(season == 2026, "qualification is the 2026-2027 season");
    Ok(JurisdictionRequest {
        jurisdiction: UsJurisdiction::RhodeIsland,
        season: SchoolYear::new(season).context("invalid season")?,
        revision: Revision(revision),
        refresh: false,
        limit_per_state: Some(1),
        concurrency: 1,
        observed_on: None,
        authorized_hosts: Vec::new(),
        source_parallelism: census_crawl::net::DEFAULT_FAMILY_PARALLELISM,
    })
}

pub(super) fn load() -> Result<Original> {
    let value = artifacts::json(&Path::new(GUEST).join("jurisdiction-recovery-original.json"))?;
    let original: Original = serde_json::from_value(value)?;
    safe_id(&original.id)?;
    ensure!(
        original.key == identity(&original.request),
        "stored request identity mismatch"
    );
    ensure!(
        text(&original.accepted, "invocationId")? == original.id,
        "stored accepted original differs"
    );
    Ok(original)
}

pub(super) fn identity(request: &JurisdictionRequest) -> String {
    WorkflowIdentity::jurisdiction(request.jurisdiction, request.season, request.revision)
        .to_string()
}

pub(super) fn target(verb: &str, service: &str, key: &str, handler: &str) -> Result<String> {
    let mut url = Url::parse(INGRESS)?;
    url.path_segments_mut()
        .map_err(|()| anyhow::anyhow!("ingress cannot carry object key"))?
        .extend(["restate", verb, service, key, handler]);
    Ok(url.into())
}

pub(super) fn safe_id(id: &str) -> Result<()> {
    ensure!(
        id.starts_with("inv_")
            && id.len() <= 256
            && id
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        "invalid invocation ID"
    );
    Ok(())
}

pub(super) fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .with_context(|| format!("required {field} absent"))
}
