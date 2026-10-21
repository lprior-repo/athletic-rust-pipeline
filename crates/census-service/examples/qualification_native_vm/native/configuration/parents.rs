use super::super::http::{query, rows};
use super::{request, verify_owner, ADMIN};
use anyhow::{ensure, Context, Result};
use reqwest::{Client, Method};
use serde_json::{json, Value};

pub(super) async fn configure(client: &Client, deployment: &str) -> Result<Value> {
    let existing = query(client, "SELECT id FROM sys_invocation WHERE target_service_name IN ('NationalCensus', 'JurisdictionCensus') LIMIT 1").await?;
    ensure!(
        rows(&existing)?.is_empty(),
        "parent inactivity override requires no accepted National/Jurisdiction invocation"
    );
    let mut evidence = serde_json::Map::new();
    evidence.insert("pre_acceptance_inventory".to_owned(), existing);
    for name in ["NationalCensus", "JurisdictionCensus"] {
        let route = format!("{ADMIN}services/{name}");
        let before = request(client, Method::GET, &route, None).await?;
        verify_owner(&before, deployment, name)?;
        let changed = request(
            client,
            Method::PATCH,
            &route,
            Some(&json!({"inactivity_timeout":"1s"})),
        )
        .await?;
        let after = request(client, Method::GET, &route, None).await?;
        verify_owner(&after, deployment, name)?;
        unchanged(&before, &changed)?;
        unchanged(&before, &after)?;
        evidence.insert(
            name.to_owned(),
            json!({"before":before,"patch":changed,"after":after}),
        );
    }
    Ok(Value::Object(evidence))
}

fn unchanged(before: &Value, after: &Value) -> Result<()> {
    let mut expected = ordered_handlers(before)?;
    ensure!(
        expected.contains_key("inactivity_timeout"),
        "original inactivity timeout absent"
    );
    for field in [
        "abort_timeout",
        "retry_policy",
        "journal_retention",
        "idempotency_retention",
        "deployment_id",
        "handlers",
    ] {
        ensure!(
            expected.get(field).is_some_and(|value| !value.is_null()),
            "original service policy {field} absent"
        );
    }
    expected.insert("inactivity_timeout".to_owned(), json!("1s"));
    ensure!(
        ordered_handlers(after)? == expected,
        "isolated parent inactivity override changed another service field"
    );
    Ok(())
}

fn ordered_handlers(value: &Value) -> Result<serde_json::Map<String, Value>> {
    let mut object = value
        .as_object()
        .context("service configuration is not an object")?
        .clone();
    let handlers = object
        .get("handlers")
        .and_then(Value::as_array)
        .context("service handlers are not an array")?;
    let mut named = std::collections::BTreeMap::new();
    for handler in handlers {
        let name = handler
            .get("name")
            .and_then(Value::as_str)
            .filter(|name| !name.is_empty())
            .context("service handler name absent")?;
        ensure!(
            named.insert(name, handler).is_none(),
            "duplicate service handler name"
        );
    }
    let ordered = named.into_values().cloned().collect();
    object.insert("handlers".to_owned(), Value::Array(ordered));
    Ok(object)
}

#[cfg(test)]
mod tests;
