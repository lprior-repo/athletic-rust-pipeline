use crate::census::source_boundary;
use census_crawl::milesplit::boundary::Hook as _;
use census_store::Store;
use sha2::{Digest, Sha256};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};

use super::{point_fixture, Fixture, OPERATION};
use serde_json::json;

type TestResult = Result<(), Box<dyn Error>>;

static ENV_LOCK: Mutex<()> = Mutex::const_new(());

async fn armed_env(fixture: &Fixture) -> MutexGuard<'static, ()> {
    let guard = ENV_LOCK.lock().await;
    std::env::set_var("CENSUS_NATIVE_SOURCE_BOUNDARY", &fixture.config);
    guard
}

fn check(condition: bool, detail: &str) -> TestResult {
    if condition {
        Ok(())
    } else {
        Err(detail.to_string().into())
    }
}

#[tokio::test]
async fn scoped_hook_publishes_schema_2_with_real_identity() -> TestResult {
    let fixture = point_fixture("before_source_request", 1)?;
    let env = armed_env(&fixture).await;
    let store = Store::open(fixture.root.path())?;
    let scope_value = source_boundary::Scope {
        store: Arc::new(store),
        operation: OPERATION.to_string(),
        request_digest: compute_digest(OPERATION, "team-123", "2026-10-11"),
        observed_on: "2026-10-11".to_string(),
    };
    let held = source_boundary::scope(scope_value, async {
        let hook = &super::super::NATIVE_BOUNDARY_HOOK;
        let point = census_crawl::milesplit::boundary::Point::BeforeSourceRequest;
        hook.reached(point).await
    })
    .await;
    check(held.is_err(), "an armed boundary point must hold")?;
    let marker_value = read_marker(&fixture)?;
    check(
        marker_value["schema"] == json!(2),
        "marker schema must be 2",
    )?;
    check(
        marker_value["phase"] == json!("before_source_request"),
        "marker phase must name the held point",
    )?;
    check(
        marker_value["operation"] == json!(OPERATION),
        "marker operation must be the armed operation",
    )?;
    check(
        marker_value["request_digest"] != json!("fault_injection"),
        "request_digest must not be the placeholder",
    )?;
    check(
        marker_value["observed_on"] != json!("fault_injection"),
        "observed_on must not be the placeholder",
    )?;
    check(
        marker_value["acknowledged_effects"].is_object(),
        "marker must carry the acknowledged effects checkpoint",
    )?;
    drop(env);
    std::env::remove_var("CENSUS_NATIVE_SOURCE_BOUNDARY");
    Ok(())
}

#[tokio::test]
async fn hook_without_scope_refuses_with_invariant() -> TestResult {
    let fixture = point_fixture("before_source_request", 1)?;
    let env = armed_env(&fixture).await;
    let hook = &super::super::NATIVE_BOUNDARY_HOOK;
    let point = census_crawl::milesplit::boundary::Point::BeforeSourceRequest;
    let error = match hook.reached(point).await {
        Ok(()) => {
            return Err("an armed point without a scope must refuse"
                .to_string()
                .into())
        }
        Err(error) => format!("{error}"),
    };
    check(
        error.contains("outside a scoped source acquisition"),
        &format!("refusal must name the missing scope: {error}"),
    )?;
    check(
        error.contains("before_source_request"),
        &format!("refusal must name the point: {error}"),
    )?;
    check(
        !fixture.marker().exists(),
        "a refused boundary must not publish a marker",
    )?;
    drop(env);
    std::env::remove_var("CENSUS_NATIVE_SOURCE_BOUNDARY");
    Ok(())
}

#[test]
fn digest_is_stable_and_distinguishes_inputs() -> TestResult {
    let op = "jurisdiction:VT:2026-27:2/teams/milesplit";
    let d1 = compute_digest(op, "team-123", "2026-10-11");
    let d2 = compute_digest(op, "team-123", "2026-10-11");
    check(d1 == d2, "digest must be stable")?;
    let d3 = compute_digest(op, "team-456", "2026-10-11");
    check(d1 != d3, "digest must bind the team")?;
    let d4 = compute_digest(op, "team-123", "2026-10-12");
    check(d1 != d4, "digest must bind the observation date")?;
    Ok(())
}

fn compute_digest(operation: &str, team: &str, observed_on: &str) -> String {
    let input = format!("{}\u{1f}{}\u{1f}{}", operation, team, observed_on);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn read_marker(fixture: &Fixture) -> Result<serde_json::Value, Box<dyn Error>> {
    Ok(serde_json::from_slice(&std::fs::read(fixture.marker())?)?)
}
