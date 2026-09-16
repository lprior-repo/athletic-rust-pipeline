use crate::model::Prospect;
use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

pub struct Prepared {
    pub fingerprint: String,
    pub config_digest: String,
    pub prospects: Vec<Prospect>,
    _lock: File,
}

#[derive(Serialize, Deserialize, PartialEq)]
struct Binding {
    schema_version: u32,
    fingerprint: String,
    config_digest: String,
    ingress: String,
    population: usize,
    no_ai: bool,
    authorization_ack: bool,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Intent {
    pub row_key: String,
    pub attempt: u32,
    pub pending: bool,
}

pub fn atomic_json(path: &Path, value: &impl Serialize) -> Result<()> {
    let temporary = path.with_extension("json.tmp");
    let mut file = File::create(&temporary)?;
    serde_json::to_writer(&mut file, value)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    fs::rename(&temporary, path)?;
    File::open(path.parent().context("artifact has no parent")?)?.sync_all()?;
    Ok(())
}

pub fn prepare(
    input: &Path,
    config: &Path,
    directory: &Path,
    ingress: &str,
    first: bool,
    no_ai: bool,
    authorization_ack: bool,
) -> Result<Prepared> {
    fs::create_dir_all(directory)?;
    // File locks are released by the OS after interruption, unlike sentinel locks.
    let lock = OpenOptions::new()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(directory.join("run.lock"))?;
    lock.try_lock()
        .context("output directory is locked by another run")?;
    let digest = crate::restate_types::config_digest(config)?;
    let configuration = crate::config::Config::load(config)?;
    if configuration.retrieval.authorized_direct_fetch && !authorization_ack {
        bail!("direct retrieval also requires --i-have-written-authorization");
    }
    if !no_ai
        && (!configuration.ollama.enabled
            || !configuration
                .identity_review
                .as_ref()
                .is_some_and(|model| model.enabled))
    {
        bail!("AI mode requires enabled extraction and identity review models");
    }
    let mode = if first {
        crate::xlsx::ScanMode::FirstWorksheet
    } else {
        crate::xlsx::ScanMode::Exhaustive
    };
    let review = if no_ai { "no-ai" } else { "ai-review" };
    let selection = if first {
        "first-worksheet"
    } else {
        "exhaustive"
    };
    let scope = format!(
        "restate-v{}:{selection}:{review}:authorization={authorization_ack}",
        crate::restate_types::RESTATE_SCHEMA_VERSION
    );
    let expected_fingerprint = crate::coverage::run_fingerprint(input, config, &scope)?;
    let prospects =
        crate::xlsx::scan(input, mode, configuration.workbook.expected_graduation_year)?.prospects;
    if prospects.is_empty() {
        bail!("workbook selection contains no prospects");
    }
    if !directory.join("run-manifest.json").exists() {
        for name in [
            "restate-binding.json",
            "restate-results.jsonl",
            "issues.jsonl",
            "restate-intents",
        ] {
            if directory.join(name).exists() {
                bail!("unbound Restate artifact: {name}");
            }
        }
    }
    let fingerprint = crate::coverage::bind_run(input, config, directory, &scope)?;
    if fingerprint != expected_fingerprint {
        bail!("workbook or configuration changed while scanning source rows");
    }
    if crate::restate_types::config_digest(config)? != digest {
        bail!("configuration changed while preparing run");
    }
    // Also validates duplicate and empty source keys before any HTTP request.
    let report = crate::coverage::build_report(&fingerprint, &prospects, &Default::default())?;
    let binding = Binding {
        schema_version: crate::restate_types::RESTATE_SCHEMA_VERSION,
        fingerprint: fingerprint.clone(),
        config_digest: digest.clone(),
        ingress: ingress.to_owned(),
        population: prospects.len(),
        no_ai,
        authorization_ack,
    };
    let path = directory.join("restate-binding.json");
    if path.exists() {
        let previous: Binding = serde_json::from_reader(File::open(&path)?)?;
        if previous != binding {
            bail!("immutable Restate run binding mismatch");
        }
    } else {
        atomic_json(&path, &binding)?;
    }
    fs::create_dir_all(directory.join("restate-intents"))?;
    crate::coverage::write_report(directory, &report)?;
    Ok(Prepared {
        fingerprint,
        config_digest: digest,
        prospects,
        _lock: lock,
    })
}

pub async fn load_intent(directory: std::path::PathBuf, key: String) -> Result<Option<Intent>> {
    tokio::task::spawn_blocking(move || {
        let path = directory
            .join("restate-intents")
            .join(format!("{key}.json"));
        match File::open(path) {
            Ok(file) => {
                let intent: Intent = serde_json::from_reader(file)?;
                if intent.row_key != key || intent.attempt == 0 {
                    bail!("invalid submission intent");
                }
                Ok(Some(intent))
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    })
    .await
    .context("joining intent read")?
}

pub async fn save_intent(directory: std::path::PathBuf, intent: Intent) -> Result<()> {
    tokio::task::spawn_blocking(move || {
        let path = directory
            .join("restate-intents")
            .join(format!("{}.json", intent.row_key));
        atomic_json(&path, &intent)
    })
    .await
    .context("joining intent persistence")?
}
