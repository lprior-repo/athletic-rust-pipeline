use super::input::{self, Original};
use super::observe::Observation;
use anyhow::{ensure, Result};
use census_service::restate_services::{JurisdictionRequest, SourcePlan};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::Path;

mod files;
mod journal;
mod proof;
#[cfg(test)]
mod tests;

const CONFIG: &str = "/srv/qualification/native-source-boundary-config.json";
const MARKER: &str = "teams-source-reservation-reached.json";
const LIMIT: u64 = 4096;

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Configuration {
    schema: u8,
    operation: String,
    attempt: u8,
    timeout_seconds: u8,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Marker {
    schema: u8,
    phase: String,
    operation: String,
    attempt: u8,
    request_digest: String,
    observed_on: String,
}

fn configuration(request: &JurisdictionRequest, plan: &SourcePlan) -> Result<Configuration> {
    ensure!(
        plan.sweepable.iter().any(|source| source == "milesplit"),
        "native boundary source is absent from the original plan"
    );
    Ok(Configuration {
        schema: 1,
        operation: format!("{}/teams/milesplit", input::identity(request)),
        attempt: 1,
        timeout_seconds: 60,
    })
}

pub(super) fn operation(original: &Original) -> Result<String> {
    ensure!(
        original.key == input::identity(&original.request),
        "native boundary original request identity differs"
    );
    Ok(configuration(&original.request, &original.source_plan)?.operation)
}

pub(super) fn prepare(request: &JurisdictionRequest, plan: &SourcePlan) -> Result<Value> {
    prepare_at(Path::new(CONFIG), &configuration(request, plan)?)
}

fn prepare_at(path: &Path, configuration: &Configuration) -> Result<Value> {
    files::publish_config(path, configuration)?;
    files::evidence(path, &files::read_required(path)?)
}

pub(super) fn witness(
    observation: &Observation,
    original: &Original,
    witness: Value,
) -> Result<Option<Value>> {
    proof::witness_at(Path::new(CONFIG), observation, original, witness)
}

pub(super) fn retained(original: &Original, before: &Value) -> Result<Value> {
    proof::retained_at(Path::new(CONFIG), original, before)
}
