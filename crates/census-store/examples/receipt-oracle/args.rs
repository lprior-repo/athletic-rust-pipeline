use census_store::NativeEffectCheckpoint;
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

const KINDS: [&str; 7] = [
    "before_source_request",
    "response_received",
    "capture_committed",
    "page_chunk",
    "before_apply",
    "after_commit_before_ack",
    "teams_reserved_before_acquisition",
];
const MAX_MARKER_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Marker {
    pub schema: u8,
    pub phase: String,
    pub operation: String,
    pub attempt: u8,
    pub request_digest: String,
    pub observed_on: String,
    pub acknowledged_effects: Option<NativeEffectCheckpoint>,
}

pub struct Args {
    pub store: PathBuf,
    pub marker: PathBuf,
    pub invocation: String,
    pub operation: String,
    pub kind: String,
    pub evidence: PathBuf,
}

pub struct Fault {
    pub code: i32,
    pub message: String,
}

impl Fault {
    pub fn usage(message: impl Into<String>) -> Self {
        Self {
            code: 2,
            message: message.into(),
        }
    }

    pub fn verify(message: impl Into<String>) -> Self {
        Self {
            code: 1,
            message: message.into(),
        }
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self {
            code: 2,
            message: message.into(),
        }
    }
}

pub fn parse_args(argv: Vec<String>) -> Result<Args, Fault> {
    let mut values: BTreeMap<String, String> = BTreeMap::new();
    let mut index = 0;
    while let Some(flag) = argv.get(index) {
        let name = flag
            .strip_prefix("--")
            .ok_or_else(|| Fault::usage(format!("unexpected argument {flag}")))?;
        if !matches!(
            name,
            "store" | "marker" | "invocation" | "operation" | "kind" | "evidence"
        ) {
            return Err(Fault::usage(format!("unknown flag --{name}")));
        }
        let value = argv
            .get(index.saturating_add(1))
            .ok_or_else(|| Fault::usage(format!("--{name} requires a value")))?;
        if values.insert(name.to_string(), value.clone()).is_some() {
            return Err(Fault::usage(format!("--{name} was given more than once")));
        }
        index = index.saturating_add(2);
    }
    let store = PathBuf::from(
        values
            .remove("store")
            .ok_or_else(|| Fault::usage("--store is required"))?,
    );
    let marker = PathBuf::from(
        values
            .remove("marker")
            .ok_or_else(|| Fault::usage("--marker is required"))?,
    );
    let invocation = values
        .remove("invocation")
        .ok_or_else(|| Fault::usage("--invocation is required"))?;
    let operation = values
        .remove("operation")
        .ok_or_else(|| Fault::usage("--operation is required"))?;
    let kind = values
        .remove("kind")
        .ok_or_else(|| Fault::usage("--kind is required"))?;
    let evidence = PathBuf::from(
        values
            .remove("evidence")
            .ok_or_else(|| Fault::usage("--evidence is required"))?,
    );
    if !store.is_dir() || !store.join("fjall").is_dir() {
        return Err(Fault::usage(
            "--store must be an existing census store directory containing fjall/",
        ));
    }
    if !marker.is_file() {
        return Err(Fault::usage("--marker must be an existing regular file"));
    }
    let marker_bytes = fs::metadata(&marker)
        .map_err(|error| Fault::usage(format!("stat --marker: {error}")))?
        .len();
    if marker_bytes > MAX_MARKER_BYTES {
        return Err(Fault::usage(format!(
            "--marker is {marker_bytes} bytes, above the {MAX_MARKER_BYTES} byte bound"
        )));
    }
    if !evidence.is_dir() {
        return Err(Fault::usage("--evidence must be an existing directory"));
    }
    if evidence.starts_with(&store) {
        return Err(Fault::usage("--evidence must not be inside --store"));
    }
    if !KINDS.contains(&kind.as_str()) {
        return Err(Fault::usage(format!(
            "--kind {kind} is not a census source boundary kind"
        )));
    }
    if operation.is_empty() || operation.chars().count() > 1024 {
        return Err(Fault::usage("--operation must be 1..=1024 characters"));
    }
    let invocation_valid = invocation.strip_prefix("inv_").is_some_and(|digits| {
        !digits.is_empty() && digits.chars().all(|c| c.is_ascii_alphanumeric())
    });
    if !invocation_valid {
        return Err(Fault::usage("--invocation must match inv_[A-Za-z0-9]+"));
    }
    Ok(Args {
        store,
        marker,
        invocation,
        operation,
        kind,
        evidence,
    })
}
