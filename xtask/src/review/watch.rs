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
    };
    let reply = ingress::block_on(census.open_work(Json(request)).call())?
        .map_err(ingress::error)?
        .into_body()
        .map_err(ingress::error)?
        .0;
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
    json!({
        "sampled_at": rfc3339_now().unwrap_or_default(),
        "season": reply.season,
        "revision": reply.revision,
        "jurisdiction_sweeps": reply.jurisdiction_sweeps,
        "jurisdiction_total": reply.jurisdictions.len(),
        "source_objects_owed": reply.source_objects,
        "silent_sources": reply.silent_sources.len(),
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
    let days = (unix_secs / 86_400) as i64;
    let of_day = unix_secs % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        of_day / 3600,
        (of_day % 3600) / 60,
        of_day % 60
    )
}

fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = shifted.div_euclid(146_097);
    let day_of_era = shifted.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    };
    (
        if month <= 2 { year + 1 } else { year },
        month as u32,
        day as u32,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_formats_as_utc() {
        assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
        assert_eq!(rfc3339(1_000_000_000), "2001-09-09T01:46:40Z");
        assert_eq!(rfc3339(1_600_000_000), "2020-09-13T12:26:40Z");
    }

    #[test]
    fn leap_day_formats() {
        assert_eq!(rfc3339(1_582_934_400), "2020-02-29T00:00:00Z");
    }

    #[test]
    fn the_admin_query_endpoint_joins_once() {
        assert_eq!(
            admin_query_endpoint("http://127.0.0.1:19095/"),
            "http://127.0.0.1:19095/query"
        );
        assert_eq!(
            admin_query_endpoint("http://127.0.0.1:19095"),
            "http://127.0.0.1:19095/query"
        );
    }

    #[test]
    fn the_written_block_round_trips_through_the_review_parser() {
        let summary = json!({
            "sampled_at": "2026-10-09T16:11:17Z",
            "season": "2026-27",
            "revision": 1,
            "jurisdiction_sweeps": 49,
            "jurisdiction_total": 49,
            "source_objects_owed": 63605,
            "silent_sources": 0,
            "unreadable": 0,
            "invocations": Value::Null,
            "owed_sets": [
                {"count": 7, "stages": "rosters, meets_history, results_history"},
                {"count": 40, "stages": "teams, rosters, meets_history"}
            ]
        });
        let block = block_text(&summary);
        assert!(!block.contains("null"));
        assert!(!block.contains('"'));
        let parsed = crate::review::build::latest_sample(&block)
            .map_or(Value::Null, core::convert::identity);
        assert_eq!(
            parsed.get("sampled_at"),
            Some(&json!("2026-10-09T16:11:17Z"))
        );
        assert_eq!(parsed.get("sweeps_owed"), Some(&json!(49)));
        assert_eq!(parsed.get("sweeps_total"), Some(&json!(49)));
        assert_eq!(parsed.get("source_objects_owed"), Some(&json!(63605)));
        assert_eq!(parsed.get("invocations"), Some(&Value::Null));
        assert_eq!(parsed.pointer("/owed_sets/0/count"), Some(&json!(7)));
        assert_eq!(parsed.pointer("/owed_sets/1/count"), Some(&json!(40)));
        assert_eq!(
            parsed.pointer("/owed_sets/1/stages"),
            Some(&json!("teams, rosters, meets_history"))
        );
    }
}
