use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(serde::Deserialize)]
pub(super) struct Generation {
    pub(super) manifest_digest: String,
    pub(super) now: String,
    pub(super) corpus: Corpus,
}

#[derive(serde::Deserialize)]
pub(super) struct Corpus {
    pub(super) rows: u64,
    pub(super) entries: u64,
    pub(super) skipped: u64,
    pub(super) notes: u64,
}

pub(super) struct Curated {
    pub(super) meta: Value,
    pub(super) matrix: Value,
    pub(super) gates: Value,
    pub(super) findings: Value,
    pub(super) prior_delta: Value,
    pub(super) limits: Value,
}

pub(super) struct Facts {
    pub(super) commit: String,
    pub(super) subject: String,
    pub(super) status: Vec<String>,
    pub(super) run_start: BTreeMap<String, String>,
    pub(super) binaries: Map<String, Value>,
    pub(super) submission: Option<Value>,
    pub(super) deployment_services: Value,
    pub(super) generation: Generation,
    pub(super) frame: Value,
    pub(super) sample: Option<Value>,
    pub(super) summary: Option<Value>,
    pub(super) drain: Option<String>,
    pub(super) store_bytes: Map<String, Value>,
    pub(super) readback: Option<Value>,
}
