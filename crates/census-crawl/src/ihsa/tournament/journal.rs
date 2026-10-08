use super::wire::{MeetRow, QualifiersEnvelope};
use crate::{AdapterContext, CrawlResult};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

pub(super) const PHASE: &str = "ihsa_tournament";
const PARSER: u64 = 2;
const MEET_KEY: &str = "meet";
const QUALIFIERS_KEY: &str = "qualifiers";

#[derive(Debug, Default)]
pub(super) struct Journal {
    pub(super) meets: HashMap<String, Option<String>>,
    pub(super) qualifiers: HashSet<String>,
    pending: Vec<(String, Value)>,
}

impl Journal {
    pub(super) fn load(ctx: &AdapterContext<'_>) -> CrawlResult<Self> {
        let mut journal = Self::default();
        let performance_as_of = ctx.performance_as_of.to_string();
        for payload in ctx.store.journal_payloads(PHASE)? {
            if payload.get("parser").and_then(Value::as_u64) != Some(PARSER)
                || payload.get("parsed").and_then(Value::as_bool) != Some(true)
            {
                continue;
            }
            let Some(key) = payload.get("key").and_then(Value::as_str) else {
                continue;
            };
            if key.starts_with("meet:")
                && payload.get("performance_as_of").and_then(Value::as_str)
                    != Some(performance_as_of.as_str())
            {
                continue;
            }
            if let Some(meet) = key.strip_prefix("meet:") {
                let refreshed = payload
                    .get("refreshed_at")
                    .and_then(Value::as_str)
                    .map(str::to_string);
                journal.meets.insert(meet.to_string(), refreshed);
            } else if let Some(list) = key.strip_prefix("qualifiers:") {
                journal.qualifiers.insert(list.to_string());
            }
        }
        Ok(journal)
    }

    pub(super) fn meet(&mut self, row: &MeetRow, url: &str, admission: (usize, chrono::NaiveDate)) {
        let key = format!("{MEET_KEY}:{}", row.meet_id);
        let payload = json!({
            "key": key.clone(),
            "url": url,
            "parser": PARSER,
            "parsed": true,
            "refreshed_at": row.last_refreshed_at,
            "events": admission.0,
            "performance_as_of": admission.1,
        });
        self.pending.push((key, payload));
        self.meets
            .insert(row.meet_id.to_string(), row.last_refreshed_at.clone());
    }

    pub(super) fn list(&mut self, key: &str, url: &str, envelope: &QualifiersEnvelope) {
        let entry = format!("{QUALIFIERS_KEY}:{key}");
        let athletes: usize = envelope
            .team_qualifiers
            .iter()
            .chain(envelope.individual_qualifiers.iter())
            .map(|qualifier| qualifier.athletes.len())
            .sum();
        let payload = json!({
            "key": entry.clone(),
            "url": url,
            "parser": PARSER,
            "parsed": true,
            "athletes": athletes,
        });
        self.pending.push((entry, payload));
        self.qualifiers.insert(key.to_string());
    }

    pub(super) fn take_pending(&mut self) -> Vec<(String, Value)> {
        std::mem::take(&mut self.pending)
    }
}
