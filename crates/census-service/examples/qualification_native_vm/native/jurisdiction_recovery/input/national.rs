use super::super::super::super::{artifacts, GUEST};
use super::super::super::http::{query, request, rows};
use super::{identity, safe_id, target, text};
use anyhow::{ensure, Context, Result};
use census_domain::model::{ProfileSelectionV1, QueryProfileV1};
use census_reconcile::identity::WorkflowIdentity;
use census_service::restate_services::{
    ExportGeneration, JurisdictionRequest, NationalRequest, NationalRequestBinding,
    PublicationContract,
};
use futures::{stream, StreamExt, TryStreamExt};
use reqwest::{Client, Method};
use serde_json::{json, Value};
use std::path::Path;
use std::time::Duration;

mod child;
pub(in super::super) mod hold;

pub(super) fn as_of() -> Result<String> {
    Ok(profile()?.input().as_of.as_str().to_owned())
}

fn profile() -> Result<QueryProfileV1> {
    let retained = artifacts::json(&Path::new(GUEST).join("selected-profile.json"))?;
    Ok(serde_json::from_value(
        retained
            .get("profile")
            .context("qualification profile absent")?
            .clone(),
    )?)
}

pub(super) fn body(child: &JurisdictionRequest) -> Result<NationalRequest> {
    let retained = artifacts::json(&Path::new(GUEST).join("selected-profile.json"))?;
    let selection: ProfileSelectionV1 = serde_json::from_value(
        retained
            .get("selection")
            .context("qualification selection absent")?
            .clone(),
    )?;
    let profile = profile()?;
    ensure!(
        profile.id()? == *selection.profile_id(),
        "qualification profile binding differs"
    );
    ensure!(
        profile.input().jurisdictions.as_slice() == [child.jurisdiction]
            && profile.input().as_of.as_str() == child.history.as_of().to_string()
            && profile.input().contact_school_year == child.season,
        "qualification selected scope differs"
    );
    Ok(NationalRequest {
        season: child.season,
        revision: child.revision,
        history: child.history,
        publication_contract: PublicationContract::SelectedPublicationV2,
        export_job: census_report::export::ExportJobRequest::SelectedPublicationV1 { selection },
        jurisdictions: vec![child.jurisdiction],
        refresh: child.refresh,
        limit_per_state: child.limit_per_state,
        concurrency: child.concurrency,
        observed_on: child.observed_on.clone(),
        authorized_hosts: child.authorized_hosts.clone(),
        source_parallelism: child.source_parallelism,
        school_address: None,
        pass_budget: census_service::restate_services::default_pass_budget(),
        pass_delay_seconds: census_service::restate_services::default_pass_delay_seconds(),
        generation: Some(ExportGeneration::parse("native-vm-qualification")?),
    })
}

pub(super) async fn ensure_unused(client: &Client, body: &NationalRequest) -> Result<()> {
    let key = WorkflowIdentity::national(body.season, body.revision, &body.jurisdictions);
    let existing = query(client, &format!(
        "SELECT id FROM sys_invocation WHERE target_service_name = 'NationalCensus' AND target_service_key = '{key}' LIMIT 2"
    )).await?;
    ensure!(
        rows(&existing)?.is_empty(),
        "qualification national identity already used"
    );
    Ok(())
}

pub(super) async fn submit(
    client: &Client,
    body: &NationalRequest,
    expected: &JurisdictionRequest,
) -> Result<Value> {
    let key = WorkflowIdentity::national(body.season, body.revision, &body.jurisdictions);
    let accepted = request(
        client,
        Method::POST,
        &target("send", "NationalCensus", key.as_str(), "run")?,
        Some(&serde_json::to_value(body)?),
    )
    .await?;
    let id = text(&accepted, "invocationId")?;
    safe_id(id)?;
    artifacts::publish(
        &Path::new(GUEST).join("national-recovery-original.json"),
        &json!({"id":id,"key":key.as_str(),"request":body,"accepted":accepted}),
    )?;
    let child = wait_child(client, id, expected).await?;
    let hold = hold::national(client, id, key.as_str()).await?;
    let bound: NationalRequestBinding = serde_json::from_value(
        request(
            client,
            Method::POST,
            &target("call", "NationalCensus", key.as_str(), "request")?,
            None,
        )
        .await?,
    )?;
    let NationalRequestBinding::Bound(bound) = bound else {
        anyhow::bail!("normal national request was not bound before child admission");
    };
    ensure!(
        serde_json::to_value(bound)? == serde_json::to_value(body)?,
        "actual bound national qualification request differs"
    );
    artifacts::publish(
        &Path::new(GUEST).join("national-recovery-paused.json"),
        &hold,
    )?;
    Ok(json!({"invocationId":child,"national_invocation_id":id,
        "national_key":key.as_str(),"national_accepted":accepted,
        "national_pause":hold,
        "child_admission":"exact original NationalCensus run call and invoked_by_id"}))
}

async fn wait_child(
    client: &Client,
    parent: &str,
    expected: &JurisdictionRequest,
) -> Result<String> {
    let key = identity(expected);
    let probes = stream::iter(0..120u32).then(|attempt| {
        let key = &key;
        async move {
            let status = query(client, &format!(
                "SELECT id, invoked_by_id, target_service_name, target_service_key, target_handler_name FROM sys_invocation WHERE invoked_by_id = '{parent}' AND target_service_name = 'JurisdictionCensus' AND target_service_key = '{key}' AND target_handler_name = 'run' LIMIT 3"
            )).await?;
            ensure!(rows(&status)?.len() <= 1, "multiple original jurisdiction calls");
            let child = child::verify(client, parent, expected, &status).await?;
            artifacts::publish(&Path::new(GUEST).join("national-child-latest.json"),
                &json!({"attempt":attempt,"parent":parent,"status":status,"child":child}))?;
            if child.is_none() { tokio::time::sleep(Duration::from_millis(100)).await; }
            Ok::<_, anyhow::Error>(child)
        }
    }).try_filter_map(|child| futures::future::ready(Ok(child)));
    futures::pin_mut!(probes);
    probes
        .try_next()
        .await?
        .context("actual national-created jurisdiction child absent")
}
