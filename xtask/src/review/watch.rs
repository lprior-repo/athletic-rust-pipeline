#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use census_service::restate_services::{CensusIngressClient, OpenWorkReply, OpenWorkRequest};
use restate_sdk::prelude::Json;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::PathBuf;

use crate::ingress;

const INVOCATION_SQL: &str = "SELECT status, COUNT(*) as n FROM sys_invocation GROUP BY status";

pub(super) struct WatchConfig {
    pub run_dir: PathBuf,
    pub review_dir: PathBuf,
    pub origin: String,
    pub admin_origin: String,
    pub season: i16,
    pub revision: u32,
    pub interval_secs: u64,
    pub once: bool,
}

pub(super) fn run(config: &WatchConfig) -> Result<()> {
    loop {
        let reply = sample(config)?;
        let summary = summarize(&reply, invocation_counts(&config.admin_origin).ok());
        write_json(
            &config.review_dir.join("open-work-latest.json"),
            &serde_json::to_value(&reply)?,
        )?;
        write_json(&config.review_dir.join("open-work-summary.json"), &summary)?;
        append_log(config, &summary)?;
        println!(
            "{}: sweeps owed {} of {}, source objects owed {}",
            summary
                .get("sampled_at")
                .map_or_else(|| "?".to_string(), Value::to_string),
            summary
                .get("jurisdiction_sweeps")
                .map_or_else(|| "?".to_string(), Value::to_string),
            summary
                .get("jurisdiction_total")
                .map_or_else(|| "?".to_string(), Value::to_string),
            summary
                .get("source_objects_owed")
                .map_or_else(|| "?".to_string(), Value::to_string)
        );
        if config.once {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_secs(config.interval_secs));
    }
}

fn sample(config: &WatchConfig) -> Result<OpenWorkReply> {
    let (endpoint, client) = ingress::client(&config.origin)?;
    ingress::announce(&endpoint, "Census", "open_work");
    let census = CensusIngressClient::from_client(client);
    let request = OpenWorkRequest {
        season: config.season,
        revision: config.revision,
        source_objects: Vec::new(),
        endpoint_page: None,
    };
    let reply: OpenWorkReply = ingress::block_on(census.open_work(Json(request)).call())?
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
    if reply.endpoint_page.is_none() {
        anyhow::bail!("serving endpoint does not provide bounded diagnostic pages");
    }
    Ok(reply)
}

fn summarize(reply: &OpenWorkReply, invocations: Option<Value>) -> Value {
    let mut owed: BTreeMap<String, u64> = BTreeMap::new();
    let mut unreadable = 0u64;
    for row in &reply.jurisdictions {
        if row.unreadable {
            unreadable = unreadable.saturating_add(1);
            continue;
        }
        let owing = row.stages.owing();
        if owing.is_empty() {
            continue;
        }
        let entry = owed.entry(owing.join(", ")).or_insert(0);
        *entry = entry.saturating_add(1);
    }
    let owed_sets: Vec<Value> = owed
        .iter()
        .map(|(stages, count)| json!({"stages": stages, "count": count}))
        .collect();
    let sampled_at = rfc3339_now().map_or(String::new(), String::from);
    json!({
        "sampled_at": sampled_at,
        "season": reply.season,
        "revision": reply.revision,
        "jurisdiction_sweeps": reply.jurisdiction_sweeps,
        "jurisdiction_total": reply.jurisdictions.len(),
        "source_objects_owed": reply.source_objects,
        "silent_sources": reply.silent_sources.len(),
        "endpoint_page": reply.endpoint_page,
        "unreadable": unreadable,
        "invocations": invocations.map_or(Value::Null, core::convert::identity),
        "owed_sets": owed_sets
    })
}

fn append_log(config: &WatchConfig, summary: &Value) -> Result<()> {
    let text = block_text(summary);
    let path = config.run_dir.join("progress.log");
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("opening {}", path.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("appending {}", path.display()))
}

fn block_text(summary: &Value) -> String {
    let mut text = String::new();
    text.push_str(&format!("=== {} ===\n", plain(summary.get("sampled_at"))));
    text.push_str(&format!(
        "season {} revision {}\n",
        plain(summary.get("season")),
        plain(summary.get("revision"))
    ));
    text.push_str(&format!(
        "jurisdiction sweeps owed: {} of {}\n",
        numeric(summary.get("jurisdiction_sweeps")),
        numeric(summary.get("jurisdiction_total"))
    ));
    text.push_str(&format!(
        "source objects owed: {}\n",
        numeric(summary.get("source_objects_owed"))
    ));
    match summary.get("invocations") {
        Some(Value::Null) | None => {}
        Some(invocations) => text.push_str(&format!("{invocations}\n")),
    }
    if let Some(sets) = summary.get("owed_sets").and_then(Value::as_array) {
        for set in sets {
            text.push_str(&format!(
                "  {}x owes: {}\n",
                numeric(set.get("count")),
                set.get("stages")
                    .and_then(Value::as_str)
                    .map_or("", core::convert::identity)
            ));
        }
    }
    text
}

fn plain(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(value) => value.to_string(),
        None => "?".to_string(),
    }
}

fn numeric(value: Option<&Value>) -> String {
    value.map_or_else(|| "unmeasured".to_string(), Value::to_string)
}

fn invocation_counts(admin_origin: &str) -> Result<Value> {
    let query = json!({"query": INVOCATION_SQL});
    let endpoint = admin_query_endpoint(admin_origin);
    let body = ingress::block_on(async {
        let client = reqwest::Client::new();
        let response = client
            .post(endpoint)
            .header("accept", "application/json")
            .json(&query)
            .send()
            .await?
            .text()
            .await?;
        Ok::<String, reqwest::Error>(response)
    })??;
    serde_json::from_str(&body).context("parsing the admin query reply")
}

fn admin_query_endpoint(admin_origin: &str) -> String {
    format!("{}/query", admin_origin.trim_end_matches('/'))
}

fn write_json(path: &PathBuf, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{text}\n")).with_context(|| format!("writing {}", path.display()))
}

fn rfc3339_now() -> Result<String> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .context("system clock before the unix epoch")?
        .as_secs();
    Ok(rfc3339(secs))
}

fn rfc3339(unix_secs: u64) -> String {
    let days = unix_secs / 86_400;
    let of_day = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        (of_day % 3600) / 60,
        of_day % 60
    )
}

fn civil_from_days(days: u64) -> (u64, u64, u64) {
    let shifted = days.saturating_add(719_468);
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era = day_of_era
        .saturating_sub(day_of_era / 1460)
        .saturating_add(day_of_era / 36_524)
        .saturating_sub(day_of_era / 146_096)
        / 365;
    let year = year_of_era.saturating_add(era.saturating_mul(400));
    let day_of_year = day_of_era.saturating_sub(
        year_of_era
            .saturating_mul(365)
            .saturating_add(year_of_era / 4)
            .saturating_sub(year_of_era / 100),
    );
    let month_prime = day_of_year.saturating_mul(5).saturating_add(2) / 153;
    let day = day_of_year
        .saturating_sub(month_prime.saturating_mul(153).saturating_add(2) / 5)
        .saturating_add(1);
    let month = if month_prime < 10 {
        month_prime.saturating_add(3)
    } else {
        month_prime.saturating_sub(9)
    };
    (
        if month <= 2 {
            year.saturating_add(1)
        } else {
            year
        },
        month,
        day,
    )
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
