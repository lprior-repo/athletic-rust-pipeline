use anyhow::{bail, Context, Result};
use futures::TryStreamExt;
use reqwest::StatusCode;
use serde::de::DeserializeOwned;
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;
use url::Url;

use athletic_rust_pipeline::runtime::source::{SOURCE_CONCURRENCY, SOURCE_SCOPE};

const MAX_ADMIN_RESPONSE_BYTES: usize = 65_536;
const MIN_RESTATE_FLOW_CONTROL_VERSION: [u32; 3] = [1, 7, 3];
const SOURCE_RULE_QUERY: &str = "SELECT pattern, concurrency, disabled, version FROM sys_rules WHERE pattern = 'athletic-source' OR pattern = '*'";

#[derive(Debug, Deserialize)]
struct VersionResponse {
    version: String,
    features: BTreeMap<String, bool>,
}

#[derive(Debug, Deserialize)]
struct QueryResponse {
    rows: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct RuleRow {
    pattern: String,
    concurrency: Option<u32>,
    disabled: bool,
    version: u32,
}

pub(crate) async fn ensure_source_scope(client: &reqwest::Client, admin: &Url) -> Result<()> {
    let version = get_version(client, admin).await?;
    verify_server_capabilities(&version)?;
    let blocks = query(
        client,
        admin,
        "SELECT key FROM state WHERE service_name = 'SourceGateway' AND service_key = 'global' AND key = 'blocked'",
    )
    .await?;
    if !blocks.rows.is_empty() {
        bail!("source admission is blocked; scope migration or deployment cannot bypass retained source policy");
    }

    let rules = source_rules(client, admin).await?;
    validate_or_provision_source_rule(client, admin, rules).await
}

async fn get_version(client: &reqwest::Client, admin: &Url) -> Result<VersionResponse> {
    let response = client.get(admin.join("version")?).send().await?;
    read_json_response(response, "reading Restate version information").await
}

fn verify_server_capabilities(version: &VersionResponse) -> Result<()> {
    if parse_version(&version.version)? < MIN_RESTATE_FLOW_CONTROL_VERSION {
        bail!(
            "native flow control requires Restate >= 1.7.3; server reports {}",
            version.version
        );
    }

    ["vqueues", "protocol_v7", "scoped_virtual_objects"]
        .into_iter()
        .try_for_each(|required| match version.features.get(required) {
            Some(true) => Ok(()),
            Some(false) => bail!("Restate flow-control feature {required} is disabled"),
            None => bail!("Restate feature {required} is not advertised"),
        })
}

fn parse_version(value: &str) -> Result<[u32; 3]> {
    let base = value
        .split('+')
        .next()
        .ok_or_else(|| anyhow::anyhow!("server version is empty"))?;
    if base.contains('-') {
        bail!("prerelease Restate versions are not supported for native flow control");
    }
    let mut components = base.split('.');
    let major = parse_version_component(components.next(), value)?;
    let minor = parse_version_component(components.next(), value)?;
    let patch = parse_version_component(components.next(), value)?;
    if components.next().is_some() {
        bail!("server version {value:?} has more than three numeric components");
    }
    Ok([major, minor, patch])
}

fn parse_version_component(component: Option<&str>, value: &str) -> Result<u32> {
    component
        .ok_or_else(|| anyhow::anyhow!("server version {value:?} is not semantic"))?
        .parse::<u32>()
        .with_context(|| format!("server version {value:?} is not numeric"))
}

async fn source_rules(client: &reqwest::Client, admin: &Url) -> Result<Vec<RuleRow>> {
    let response = query(client, admin, SOURCE_RULE_QUERY).await?;
    response
        .rows
        .into_iter()
        .map(|row| serde_json::from_value(row).context("decoding sys_rules row"))
        .collect()
}

async fn validate_or_provision_source_rule(
    client: &reqwest::Client,
    admin: &Url,
    rules: Vec<RuleRow>,
) -> Result<()> {
    let exact = rules.iter().find(|rule| rule.pattern == SOURCE_SCOPE);
    if let Some(rule) = exact {
        validate_existing_rule(rule)?;
        return reject_stricter_wildcard(&rules, rule.concurrency.map_or(0, |limit| limit));
    }

    let source_limit = rules
        .iter()
        .find(|rule| rule.pattern == "*" && !rule.disabled)
        .and_then(|rule| rule.concurrency)
        .map_or(SOURCE_CONCURRENCY, |limit| limit.min(SOURCE_CONCURRENCY));
    if source_limit == 0 {
        bail!("Restate wildcard source concurrency must be positive");
    }
    let response = client
        .put(admin.join("limits/rules")?)
        .json(&serde_json::json!([{
            "pattern": SOURCE_SCOPE,
            "limits": {"concurrency": source_limit},
            "precondition": {"type": "does_not_exist"}
        }]))
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::OK {
        let body = bounded_bytes(response, "reading rule provisioning error").await?;
        bail!(
            "provisioning Restate source rule failed with {status}: {}",
            String::from_utf8_lossy(&body)
        );
    }
    let _: Value = read_json_response_body(response, "decoding rule provisioning response").await?;
    let confirmed = source_rules(client, admin).await?;
    let rule = confirmed
        .iter()
        .find(|rule| rule.pattern == SOURCE_SCOPE)
        .ok_or_else(|| anyhow::anyhow!("Restate source rule disappeared after provisioning"))?;
    validate_existing_rule(rule)?;
    reject_stricter_wildcard(&confirmed, rule.concurrency.map_or(0, |limit| limit))
}

fn validate_existing_rule(rule: &RuleRow) -> Result<()> {
    if rule.disabled {
        bail!(
            "Restate source rule {} is disabled (version {})",
            rule.pattern,
            rule.version
        );
    }
    match rule.concurrency {
        Some(limit) if (1..=SOURCE_CONCURRENCY).contains(&limit) => Ok(()),
        Some(limit) => bail!(
            "Restate source rule {} has disallowed concurrency {}; expected 1..={SOURCE_CONCURRENCY}",
            rule.pattern,
            limit
        ),
        None => bail!("Restate source rule {} is unlimited", rule.pattern),
    }
}

fn reject_stricter_wildcard(rules: &[RuleRow], source_limit: u32) -> Result<()> {
    if let Some(rule) = rules.iter().find(|rule| rule.pattern == "*") {
        match (rule.disabled, rule.concurrency) {
            (false, Some(limit)) if limit < source_limit => bail!(
                "cannot deploy {}: exact source limit {source_limit} overrides stricter wildcard limit {}",
                SOURCE_SCOPE,
                limit
            ),
            _ => {}
        }
    }
    Ok(())
}

async fn query(client: &reqwest::Client, admin: &Url, sql: &str) -> Result<QueryResponse> {
    let response = client
        .post(admin.join("query")?)
        .header(reqwest::header::ACCEPT, "application/json")
        .json(&serde_json::json!({"query": sql}))
        .send()
        .await?;
    read_json_response(response, "reading Restate SQL query response").await
}

async fn read_json_response<T: DeserializeOwned>(
    response: reqwest::Response,
    context: &str,
) -> Result<T> {
    let status = response.status();
    let body = bounded_bytes(response, context).await?;
    if !status.is_success() {
        bail!(
            "{context} failed with {status}: {}",
            String::from_utf8_lossy(&body)
        );
    }
    serde_json::from_slice(&body).with_context(|| format!("{context}: invalid JSON"))
}

async fn read_json_response_body<T: DeserializeOwned>(
    response: reqwest::Response,
    context: &str,
) -> Result<T> {
    let body = bounded_bytes(response, context).await?;
    serde_json::from_slice(&body).with_context(|| format!("{context}: invalid JSON"))
}

async fn bounded_bytes(response: reqwest::Response, context: &str) -> Result<Vec<u8>> {
    response
        .bytes_stream()
        .map_err(anyhow::Error::from)
        .try_fold(Vec::new(), |mut bytes, chunk| async move {
            if bytes
                .len()
                .checked_add(chunk.len())
                .is_none_or(|size| size > MAX_ADMIN_RESPONSE_BYTES)
            {
                return Err(anyhow::anyhow!("{context} exceeds 64 KiB"));
            }
            bytes.extend_from_slice(&chunk);
            Ok(bytes)
        })
        .await
}
