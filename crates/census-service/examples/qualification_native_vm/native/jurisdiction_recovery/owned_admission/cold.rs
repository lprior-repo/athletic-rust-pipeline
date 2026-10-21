use super::super::super::super::{artifacts, GUEST};
use super::Original;
use anyhow::{ensure, Context, Result};
use census_crawl::net::{FetchOptions, Fetcher};
use census_domain::model::{serialized_digest, AcquisitionDigest, CaptureCacheIntent, CensusRun};
use census_store::{Application, CaptureWriterV1, PublicResourceRootKindV1, Receipt, Store};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

pub(in super::super) fn read(
    store: &Store,
    writer: &CaptureWriterV1,
    original: &Original,
    finished: &Value,
) -> Result<Value> {
    retained_call(original, finished)?;
    let owner = writer.public_resource_owner(store, original.request.jurisdiction)?;
    let scope = owner.scope();
    let manifest = store
        .run_manifest()?
        .context("cold admitted run manifest absent")?;
    let run = CensusRun::new(original.request.season, original.request.revision.get())
        .context("cold original run invalid")?;
    let intent = if original.request.refresh {
        CaptureCacheIntent::AcquireNew
    } else {
        CaptureCacheIntent::ReuseEligible
    };
    ensure!(
        scope.owner_key() == original.key
            && scope.jurisdiction() == original.request.jurisdiction
            && scope.run() == run
            && manifest.run == run
            && manifest.store_identity == writer.store_identity().as_str()
            && manifest
                .jurisdictions
                .contains(&original.request.jurisdiction)
            && scope.history() == original.request.history
            && scope.cache_intent() == intent
            && scope.run_manifest_digest()
                == &AcquisitionDigest::parse(&serialized_digest(&manifest)?)?,
        "cold protected owner differs from original run scope"
    );
    let expected = format!(
        "public_resource_capture_v1:declare:{}",
        scope.source_admission().plan_fingerprint()
    );
    let acknowledged: Application = serde_json::from_value(
        finished
            .pointer("/owned_resource_admission/application/application")
            .context("owned admission acknowledgement absent")?
            .clone(),
    )?;
    let actual = store
        .receipt(&expected)?
        .context("cold original declaration receipt absent")?;
    receipt(&acknowledged, &actual, &expected)?;
    let roots = roots(store, &owner, original)?;
    Ok(
        json!({"original_invocation_id":original.id,"actual_declaration_receipt":actual,
        "owner_key":scope.owner_key(),"site":scope.jurisdiction(),"run":scope.run(),
        "history":scope.history(),"manifest_digest":scope.run_manifest_digest(),
        "roots":roots,"protected_binding_qualified":true,"single_owner_offline":true,
        "scope":"existing checked protected declaration/owner/root reads and exact acknowledged receipt; production admission retains original National/request/plan validation"}),
    )
}

fn receipt(acknowledged: &Application, actual: &Receipt, expected: &str) -> Result<()> {
    ensure!(
        actual.operation == expected
            && acknowledged.receipt() == actual
            && actual.appended == 0
            && !actual.credited,
        "acknowledgement does not match the cold original protected declaration receipt"
    );
    Ok(())
}

fn roots(
    store: &Store,
    owner: &census_store::PublicResourceCaptureOwnerV1,
    original: &Original,
) -> Result<Value> {
    ensure!(
        store.http_cache_dir().is_dir(),
        "original HTTP cache root absent"
    );
    let fetcher = Fetcher::new(
        store.http_cache_dir(),
        None,
        Duration::ZERO,
        HashMap::new(),
        original.request.authorized_hosts.clone(),
    )?;
    let site = census_crawl::milesplit::Site::for_jurisdiction(original.request.jurisdiction);
    let options = FetchOptions {
        refresh: original.request.refresh,
        allow_not_found: true,
        ..FetchOptions::default()
    };
    let directory = owner.for_root(store, PublicResourceRootKindV1::MileSplitTeamDirectory)?;
    let robots = owner.for_root(store, PublicResourceRootKindV1::Robots)?;
    ensure!(
        directory.request() == &fetcher.public_resource_request(&site.teams_url(), &options)?
            && robots.request()
                == &fetcher.public_resource_request(
                    &format!("https://{}/robots.txt", site.host()),
                    &options
                )?,
        "cold protected roots differ from the original normalized requests"
    );
    Ok(json!({"directory":directory,"robots":robots,"source_dispatches":0}))
}

fn retained_call(original: &Original, finished: &Value) -> Result<()> {
    let before = artifacts::json(&Path::new(GUEST).join("jurisdiction-recovery-boundary.json"))?;
    let call: super::super::journal::SourceCall = serde_json::from_value(
        before
            .pointer("/boundary/witness/source_call")
            .context("original source call absent")?
            .clone(),
    )?;
    let intent = finished
        .pointer("/owned_resource_admission/application/intent")
        .context("retained admission intent absent")?;
    ensure!(
        intent.get("parent_invocation_id").and_then(Value::as_str) == Some(original.id.as_str())
            && intent.get("original_source_call") == Some(&serde_json::to_value(&call)?)
            && intent.get("request") == Some(&serde_json::to_value(&call.request)?),
        "retained admission intent differs from the original source call"
    );
    let bytes = super::super::journal::bytes(
        intent
            .get("admission_request_bytes")
            .context("admission request bytes absent")?,
    )?;
    ensure!(
        bytes == serde_json::to_vec(&serde_json::to_value(&call.request)?)?,
        "retained admission request bytes changed"
    );
    let source = super::super::journal::bytes(
        intent
            .get("original_source_request_bytes")
            .context("original source bytes absent")?,
    )?;
    let settled = serde_json::from_value(
        finished
            .pointer("/owned_resource_admission/source_before_admission")
            .context("source observation before admission absent")?
            .clone(),
    )?;
    ensure!(
        source == super::source::input_bytes(&call, &settled)?,
        "retained original source bytes differ from its actual journal input"
    );
    let request: census_service::restate_services::TeamsSourceRequest =
        serde_json::from_slice(&source)?;
    ensure!(
        serde_json::to_value(request)? == serde_json::to_value(&call.request)?,
        "retained original source input differs from admission"
    );
    Ok(())
}

#[cfg(test)]
mod tests;
