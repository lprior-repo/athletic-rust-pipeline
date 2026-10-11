use super::frame::frame_counts;
use super::latest_sample;
use super::model::{Curated, Facts, Generation};
use super::readers::{
    drain_certificate, read_binaries, read_json, read_kv, read_optional, read_optional_json,
    read_services, read_submission,
};
use super::storage::{git, relative, store_bytes};
use anyhow::{Context, Result};
use serde_json::{json, Value};
use std::fs;
use std::path::Path;

pub(super) fn read_curated(dir: &Path) -> Result<Curated> {
    Ok(Curated {
        meta: read_json(&dir.join("review_meta.json"))?,
        matrix: read_matrix(dir)?,
        gates: read_json(&dir.join("gates.json"))?,
        findings: read_json(&dir.join("findings.json"))?,
        prior_delta: read_json(&dir.join("prior_delta.json"))?,
        limits: read_json(&dir.join("limits.json"))?,
    })
}

fn read_matrix(dir: &Path) -> Result<Value> {
    let rust = dir.join("matrix/static-matrix.rust.json");
    let legacy = dir.join("matrix/static-matrix.json");
    let (path, producer) = if rust.exists() {
        (rust, "xtask review-matrix")
    } else {
        (legacy, "review script (superseded)")
    };
    let mut value = read_json(&path)?;
    if value.is_null() {
        value = json!({});
    }
    value
        .as_object_mut()
        .context("matrix input must be a JSON object or null")?
        .insert(
            "matrix_provenance".to_string(),
            json!({
                "path": relative(&path),
                "producer": producer
            }),
        );
    Ok(value)
}

pub(super) fn read_facts(dir: &Path, run_dir: &Path) -> Result<Facts> {
    let generation_path = run_dir.join("school-address/current/pipeline_report.json");
    let file = fs::File::open(&generation_path)
        .with_context(|| format!("opening {}", generation_path.display()))?;
    let generation: Generation = serde_json::from_reader(std::io::BufReader::new(file))
        .with_context(|| format!("parsing {}", generation_path.display()))?;
    let progress = read_optional(&run_dir.join("progress.log"))?;
    Ok(Facts {
        commit: git(&["rev-parse", "HEAD"])?,
        subject: git(&["log", "-1", "--format=%s"])?,
        status: git(&["status", "--short"])?
            .lines()
            .map(str::to_string)
            .collect(),
        run_start: read_kv(&run_dir.join("run-start.txt"))?,
        binaries: read_binaries(&run_dir.join("binary-sha256.txt"))?,
        submission: read_submission(&run_dir.join("national-submit.log"))?,
        deployment_services: read_services(&run_dir.join("registration.json"))?,
        generation,
        frame: frame_counts(&run_dir.join("school-address/current/school_directory.csv"))?,
        sample: progress.as_deref().and_then(latest_sample),
        summary: read_optional_json(&dir.join("open-work-summary.json"))?,
        drain: read_optional(&run_dir.join("serve.log"))?.and_then(|log| drain_certificate(&log)),
        store_bytes: store_bytes(run_dir)?,
        readback: read_optional_json(&dir.join("readback.json"))?,
    })
}
