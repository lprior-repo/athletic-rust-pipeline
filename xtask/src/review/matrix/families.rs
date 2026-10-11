use super::arms::Arms;
use super::STRATEGIES_FILE;

use anyhow::{Context, Result};
use indexmap::IndexMap;
use regex::Regex;
use serde_json::{json, Value};
use std::path::Path;

pub(super) fn parse_strategies(root: &Path) -> Result<IndexMap<String, String>> {
    let path = root.join(STRATEGIES_FILE);
    let content =
        std::fs::read_to_string(&path).with_context(|| format!("reading {}", STRATEGIES_FILE))?;

    let re =
        Regex::new(r#"([a-z_]+)\s*=>\s*"([^"]+)"#).with_context(|| "compiling strategy regex")?;

    let mut strategies = IndexMap::new();
    for cap in re.captures_iter(&content) {
        let Some(slug) = cap.get(1) else {
            anyhow::bail!("strategy entry carries no name");
        };
        let Some(reason) = cap.get(2) else {
            anyhow::bail!("strategy entry carries no reason");
        };
        strategies.insert(slug.as_str().to_string(), reason.as_str().to_string());
    }

    let wildcard =
        Regex::new(r#"_ =>\s*"([^"]+)"#).with_context(|| "compiling wildcard strategy regex")?;
    if let Some(cap) = wildcard.captures(&content) {
        let Some(reason) = cap.get(1) else {
            anyhow::bail!("wildcard strategy carries no reason");
        };
        strategies.insert("wildcard".to_string(), reason.as_str().to_string());
    }

    Ok(strategies)
}

pub(super) fn build_families(
    applicability: &[Value],
    arbiter: &Value,
    arms: &Arms,
    strategies: &IndexMap<String, String>,
) -> Vec<Value> {
    let all_applicability: Vec<&Value> = applicability.iter().chain([arbiter]).collect();
    let mut families = Vec::new();

    for entry in all_applicability {
        families.push(family_entry(entry, arms, strategies));
    }

    let mut seen = IndexMap::new();
    for f in families {
        let slug = f
            .get("slug")
            .and_then(Value::as_str)
            .map_or_else(|| "unknown".to_string(), |s| s.to_string());
        seen.insert(slug, f);
    }

    let mut result: Vec<Value> = seen.into_values().collect();
    result.sort_by(|a, b| {
        let sa = a
            .get("slug")
            .and_then(Value::as_str)
            .map_or("", core::convert::identity);
        let sb = b
            .get("slug")
            .and_then(Value::as_str)
            .map_or("", core::convert::identity);
        sa.cmp(sb)
    });

    result
}

fn family_entry(entry: &Value, arms: &Arms, strategies: &IndexMap<String, String>) -> Value {
    let slug = entry
        .get("slug")
        .and_then(Value::as_str)
        .map_or_else(|| "unknown".to_string(), |s| s.to_string());
    let strategy = determine_strategy(&slug, arms);
    let mut family = json!({
        "slug": slug,
        "applicable": applicable_names(entry),
        "strategy": strategy,
        "arms": determine_arms(&slug, arms),
    });
    if strategy == "engineering_gap" {
        if let Value::Object(fields) = &mut family {
            fields.insert(
                "gap_reason".to_string(),
                json!(gap_reason(&slug, strategies)),
            );
        }
    }
    family
}

fn applicable_names(entry: &Value) -> Vec<String> {
    let applicable = entry
        .get("applicable")
        .and_then(Value::as_array)
        .map_or_else(Vec::new, Clone::clone);
    let mut names: Vec<String> = applicable
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    names.sort();
    names.dedup();
    names
}

fn gap_reason(slug: &str, strategies: &IndexMap<String, String>) -> String {
    strategies
        .get(slug)
        .map_or_else(
            || {
                strategies.get("wildcard").map_or(
                    "registry family has no implemented durable execution strategy",
                    String::as_str,
                )
            },
            String::as_str,
        )
        .to_string()
}

pub(super) fn determine_strategy(slug: &str, arms: &Arms) -> String {
    if arms.teams.iter().any(|(s, _)| s == slug) {
        return "teams".to_string();
    }
    if arms.meets.iter().any(|(s, _)| s == slug) {
        return "meets".to_string();
    }
    if arms.results.iter().any(|(s, _)| s == slug) {
        return "results".to_string();
    }
    "engineering_gap".to_string()
}

fn determine_arms(slug: &str, arms: &Arms) -> Vec<String> {
    let tables = [
        ("teams", &arms.teams),
        ("meets", &arms.meets),
        ("results", &arms.results),
    ];
    tables
        .iter()
        .filter(|(_, table)| table.iter().any(|(s, _)| s == slug))
        .map(|(name, _)| (*name).to_string())
        .collect()
}
