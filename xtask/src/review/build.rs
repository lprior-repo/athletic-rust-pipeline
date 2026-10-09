#![forbid(unsafe_code)]

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use crate::cmd::Cmd;

const REVIEW_JSON: &str = "census-completeness-review.json";
const REVIEW_MD: &str = "census-completeness-review.md";
const STORE_DIRS: [&str; 4] = ["fjall", "http", "out", "restate"];

#[derive(serde::Deserialize)]
struct Generation {
    manifest_digest: String,
    now: String,
    corpus: Corpus,
}

#[derive(serde::Deserialize)]
struct Corpus {
    rows: u64,
    entries: u64,
    skipped: u64,
    notes: u64,
}

struct Curated {
    meta: Value,
    matrix: Value,
    gates: Value,
    findings: Value,
    prior_delta: Value,
    limits: Value,
}

struct Facts {
    commit: String,
    subject: String,
    status: Vec<String>,
    run_start: BTreeMap<String, String>,
    binaries: Map<String, Value>,
    submission: Option<Value>,
    deployment_services: u64,
    generation: Generation,
    frame: Value,
    sample: Option<Value>,
    summary: Option<Value>,
    drain: Option<String>,
    store_bytes: Map<String, Value>,
    readback: Option<Value>,
}

pub(super) fn run(dir: &Path, run_dir: &Path, reviewed_at: Option<&str>) -> Result<()> {
    let curated = read_curated(dir)?;
    let facts = read_facts(dir, run_dir)?;
    let run_rel = relative(run_dir);
    let reviewed_at = reviewed_at
        .map(str::to_string)
        .or_else(|| facts.run_start.get("submit").cloned())
        .context("no reviewed-at given and the run records no submit instant")?;
    let doc = assemble(&curated, &facts, &run_rel, &reviewed_at);
    write_json(&dir.join(REVIEW_JSON), &doc)?;
    let md = render_md(&doc);
    fs::write(dir.join(REVIEW_MD), md)
        .with_context(|| format!("writing {}", dir.join(REVIEW_MD).display()))?;
    println!(
        "wrote {} and {}",
        dir.join(REVIEW_JSON).display(),
        dir.join(REVIEW_MD).display()
    );
    println!(
        "families={} armed={} findings={} gates={} verdict={}",
        u(&doc["source_matrix"], "registered_families"),
        u(&doc["source_matrix"], "armed_families"),
        doc["findings"].as_array().map_or(0, Vec::len),
        doc["gates"].as_array().map_or(0, Vec::len),
        s(&doc, "verdict")
    );
    Ok(())
}

fn read_curated(dir: &Path) -> Result<Curated> {
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
    value["matrix_provenance"] = json!({
        "path": relative(&path),
        "producer": producer
    });
    Ok(value)
}

fn read_facts(dir: &Path, run_dir: &Path) -> Result<Facts> {
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

fn assemble(curated: &Curated, facts: &Facts, run_rel: &str, reviewed_at: &str) -> Value {
    let meta = &curated.meta;
    let matrix = &curated.matrix;
    let reviewer = meta
        .get("reviewer")
        .cloned()
        .map_or(Value::Null, core::convert::identity);
    json!({
        "schema_version": s(meta, "schema_version"),
        "repository": "athletic-rust-pipeline",
        "commit": facts.commit,
        "commit_subject": facts.subject,
        "worktree_status": facts.status,
        "reviewed_at": reviewed_at,
        "verdict": s(meta, "verdict"),
        "verdict_scope": s(meta, "verdict_scope"),
        "reviewer": reviewer,
        "scope": scope(meta, facts, run_rel, u(matrix, "jurisdiction_count")),
        "verification": verification(meta, facts),
        "known_denominators": Value::Object(denominators(meta, matrix, facts)),
        "missing_skills": meta.get("missing_skills").cloned().map_or(Value::Null, core::convert::identity),
        "skills_invoked": meta.get("skills_invoked").cloned().map_or(Value::Null, core::convert::identity),
        "findings": curated.findings,
        "gates": curated.gates,
        "source_matrix": source_matrix(matrix),
        "jurisdictions": matrix.get("jurisdictions").cloned().map_or(Value::Null, core::convert::identity),
        "prior_delta": curated.prior_delta,
        "limits": curated.limits,
        "run": run_evidence(facts),
        "bundle_readback": facts.readback.clone().map_or(Value::Null, core::convert::identity)
    })
}

fn denominators(meta: &Value, matrix: &Value, facts: &Facts) -> Map<String, Value> {
    let mut denominators = Map::new();
    denominators.insert(
        "jurisdictions_scope".to_string(),
        json!({"value": u(matrix, "jurisdiction_count"), "method": "UsJurisdiction::CENSUS_SCOPE"}),
    );
    denominators.insert(
        "school_directory_generation".to_string(),
        json!({"value": facts.generation.manifest_digest, "method": "school-address generation built by the serving binary from retained CCD/PSS captures"}),
    );
    denominators.insert(
        "school_directory_frame_rows".to_string(),
        json!({"value": facts.generation.corpus.rows, "method": "admitted NCES CCD + PSS rows in the generation; skipped and noted rows are recorded separately"}),
    );
    denominators.insert(
        "school_directory_website_claims".to_string(),
        json!({"value": facts.frame.get("website_claims").cloned().map_or(Value::Null, core::convert::identity), "method": "generation rows whose WEBSITE cell is an http(s) URL"}),
    );
    denominators.insert(
        "school_directory_rows_with_zip".to_string(),
        json!({"value": facts.frame.get("with_zip").cloned().map_or(Value::Null, core::convert::identity), "method": "generation rows with a non-empty ZIP cell"}),
    );
    denominators.insert(
        "discovered_source_objects".to_string(),
        json!({"value": facts.summary.clone().map_or(Value::Null, |s| s.get("source_objects_owed").cloned().map_or(Value::Null, core::convert::identity)), "method": "open-work source objects that have accepted nothing, at the last sample; not a nationwide denominator"}),
    );
    if let Some(authored) = meta.get("authored_denominators").and_then(Value::as_object) {
        for (key, entry) in authored {
            denominators.insert(key.clone(), entry.clone());
        }
    }
    denominators
}

fn source_matrix(matrix: &Value) -> Value {
    json!({
        "provenance": matrix.get("matrix_provenance").cloned().map_or(Value::Null, core::convert::identity),
        "registered_families": u(matrix, "registered_families"),
        "armed_families": u(matrix, "armed_families"),
        "gap_families": u(matrix, "gap_families"),
        "arms_not_registered": matrix.get("arms_not_registered").cloned().map_or(Value::Null, core::convert::identity),
        "families": matrix.get("families").cloned().map_or(Value::Null, core::convert::identity)
    })
}

fn run_evidence(facts: &Facts) -> Value {
    json!({
        "run_identity": facts.submission.as_ref().and_then(|s| s.get("run_identity")).cloned().map_or(Value::Null, core::convert::identity),
        "started": facts.run_start.get("start").cloned().map_or(Value::Null, Value::String),
        "binaries": Value::Object(facts.binaries.clone()),
        "generation": {"pipeline_report": {"manifest_digest": facts.generation.manifest_digest, "now": facts.generation.now, "corpus": json!({"rows": facts.generation.corpus.rows, "entries": facts.generation.corpus.entries, "skipped": facts.generation.corpus.skipped, "notes": facts.generation.corpus.notes})}},
        "deployment_services": facts.deployment_services,
        "latest_progress": facts.sample.clone().map_or(Value::Null, core::convert::identity),
        "open_work": facts.summary.clone().map_or(Value::Null, core::convert::identity),
        "drain_certificate": facts.drain.clone().map_or(Value::Null, Value::String),
        "store_bytes": Value::Object(facts.store_bytes.clone())
    })
}

fn scope(meta: &Value, facts: &Facts, run_rel: &str, jurisdiction_count: u64) -> Value {
    json!({
        "cohort": u(meta, "cohort"),
        "season": s(meta, "season"),
        "revision": u(meta, "revision"),
        "jurisdictions": jurisdiction_count,
        "surfaces": meta.get("surfaces").cloned().map_or(Value::Null, core::convert::identity),
        "as_of": facts.run_start.get("submit").cloned().map_or(Value::Null, Value::String),
        "run_identity": facts.submission.as_ref().and_then(|s| s.get("run_identity")).cloned().map_or(Value::Null, core::convert::identity),
        "invocation": facts.submission.as_ref().and_then(|s| s.get("invocation")).cloned().map_or(Value::Null, core::convert::identity),
        "run_root": run_rel
    })
}

fn verification(meta: &Value, facts: &Facts) -> Value {
    let authored = meta
        .get("verification")
        .cloned()
        .map_or(Value::Null, core::convert::identity);
    let executed = authored
        .get("executed")
        .cloned()
        .map_or(Value::Null, core::convert::identity);
    let not_executed = authored
        .get("not_executed")
        .cloned()
        .map_or(Value::Null, core::convert::identity);
    json!({
        "executed": executed,
        "not_executed": not_executed,
        "computed": {
            "binaries": Value::Object(facts.binaries.clone()),
            "generation": {"manifest_digest": facts.generation.manifest_digest, "rows": facts.generation.corpus.rows},
            "deployment_services": facts.deployment_services,
            "submission": facts.submission.clone().map_or(Value::Null, core::convert::identity),
            "latest_sample": facts.sample.clone().map_or(Value::Null, core::convert::identity),
            "drain_certificate": facts.drain.clone().map_or(Value::Null, Value::String),
            "readback_artifacts": facts.readback.as_ref().and_then(|r| r.get("artifacts")).and_then(Value::as_array).map_or(Value::Null, |a| json!(a.len())),
            "readback_mismatches": facts.readback.as_ref().and_then(|r| r.get("mismatches")).cloned().map_or(Value::Null, core::convert::identity)
        }
    })
}

fn render_md(doc: &Value) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "# Census completeness review — {}\n",
        s(doc, "reviewed_at")
    );
    md_header(&mut out, doc);
    md_scope(&mut out, doc);
    md_run(&mut out, doc);
    md_matrix(&mut out, doc);
    md_gates(&mut out, doc);
    md_findings(&mut out, doc);
    md_denominators(&mut out, doc);
    md_readback(&mut out, doc);
    md_prior_and_limits(&mut out, doc);
    out
}

fn md_header(out: &mut String, doc: &Value) {
    let _ = writeln!(
        out,
        "**Verdict: {}.** {}\n",
        s(doc, "verdict"),
        s(doc, "verdict_scope")
    );
    let _ = writeln!(
        out,
        "Schema `{}` · repository `{}` · commit `{}` ({})\n",
        s(doc, "schema_version"),
        s(doc, "repository"),
        s(doc, "commit"),
        s(doc, "commit_subject")
    );
    let _ = writeln!(
        out,
        "Reviewer: {} · host-reported model: `{}`\n",
        s(&doc["reviewer"], "identity"),
        s(&doc["reviewer"], "model_reported_by_host")
    );
}

fn md_scope(out: &mut String, doc: &Value) {
    let scope = &doc["scope"];
    let _ = writeln!(out, "## Scope\n");
    let _ = writeln!(
        out,
        "- Cohort {}, season {}, revision {}, {} jurisdictions, surfaces {}\n",
        u(scope, "cohort"),
        s(scope, "season"),
        u(scope, "revision"),
        u(scope, "jurisdictions"),
        list(&scope["surfaces"])
    );
    let _ = writeln!(
        out,
        "- Run `{}` · invocation `{}` · root `{}` · submitted {}\n",
        s(scope, "run_identity"),
        s(scope, "invocation"),
        s(scope, "run_root"),
        s(scope, "as_of")
    );
}

fn md_run(out: &mut String, doc: &Value) {
    let run = &doc["run"];
    let _ = writeln!(out, "## Run state\n");
    let _ = writeln!(
        out,
        "- Binaries: {}",
        object_entries(&run["binaries"], |sha| format!("`{}…`", short(sha)))
    );
    let generation = &run["generation"]["pipeline_report"];
    let _ = writeln!(
        out,
        "- School-directory generation `{}…` — corpus {} — {} deployment services",
        short(s(generation, "manifest_digest")),
        generation
            .get("corpus")
            .map_or_else(String::new, Value::to_string),
        u(run, "deployment_services")
    );
    let sample = &run["latest_progress"];
    let _ = writeln!(
        out,
        "- Latest sample ({}): sweeps owed {} of {}, source objects owed {}",
        s(sample, "sampled_at"),
        sample
            .get("sweeps_owed")
            .map_or_else(|| "?".to_string(), Value::to_string),
        sample
            .get("sweeps_total")
            .map_or_else(|| "?".to_string(), Value::to_string),
        sample
            .get("source_objects_owed")
            .map_or_else(|| "?".to_string(), Value::to_string)
    );
    let _ = writeln!(
        out,
        "- Drain certificate: {}\n",
        dash(&run["drain_certificate"])
    );
}

fn md_matrix(out: &mut String, doc: &Value) {
    let matrix = &doc["source_matrix"];
    let _ = writeln!(out, "## Source matrix\n");
    let _ = writeln!(
        out,
        "- {} registered families, {} with a durable arm, {} explicit engineering gaps, {} arm slugs outside the registry\n",
        u(matrix, "registered_families"),
        u(matrix, "armed_families"),
        u(matrix, "gap_families"),
        matrix["arms_not_registered"].as_array().map_or(0, Vec::len)
    );
    if matrix["provenance"].is_object() {
        let _ = writeln!(
            out,
            "- matrix provenance: `{}` produced by {}\n",
            s(&matrix["provenance"], "path"),
            s(&matrix["provenance"], "producer")
        );
    }
    let _ = writeln!(out, "| family | applicable | strategy | arm or gap |");
    let _ = writeln!(out, "|---|---|---|---|");
    for family in matrix["families"].as_array().map_or(&[][..], Vec::as_slice) {
        let arms = family["arms"].as_array().map_or(0, Vec::len);
        let detail = if arms > 0 {
            list(&family["arms"])
        } else {
            s(family, "gap_reason").to_string()
        };
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            s(family, "slug"),
            family["applicable"].as_array().map_or(0, Vec::len),
            s(family, "strategy"),
            detail
        );
    }
    out.push('\n');
}

fn md_gates(out: &mut String, doc: &Value) {
    let _ = writeln!(out, "## Gates\n");
    for gate in doc["gates"].as_array().map_or(&[][..], Vec::as_slice) {
        let _ = writeln!(
            out,
            "- **{} {} — {}.** {}",
            s(gate, "id"),
            s(gate, "title"),
            s(gate, "state"),
            s(gate, "note")
        );
    }
    out.push('\n');
}

fn md_findings(out: &mut String, doc: &Value) {
    let _ = writeln!(out, "## Findings\n");
    for finding in doc["findings"].as_array().map_or(&[][..], Vec::as_slice) {
        let _ = writeln!(
            out,
            "### {} [{}] {} — {}\n",
            s(finding, "id"),
            s(finding, "severity"),
            s(finding, "title"),
            s(finding, "status")
        );
        let _ = writeln!(
            out,
            "- evidence: {}; runtime reproduced: {}\n",
            s(finding, "evidence_kind"),
            finding
                .get("runtime_reproduced")
                .map_or_else(String::new, Value::to_string)
        );
        for location in finding["locations"]
            .as_array()
            .map_or(&[][..], Vec::as_slice)
        {
            let _ = writeln!(
                out,
                "- location: `{}`",
                location.as_str().map_or("", core::convert::identity)
            );
        }
        let _ = writeln!(out, "- problem: {}", s(finding, "problem"));
        let _ = writeln!(
            out,
            "- counterexample: {}",
            s(finding, "minimal_counterexample")
        );
        let _ = writeln!(out, "- remediation: {}", s(finding, "remediation"));
        let _ = writeln!(out, "- regression: {}\n", s(finding, "future_regression"));
    }
}

fn md_denominators(out: &mut String, doc: &Value) {
    let _ = writeln!(out, "## Known denominators\n");
    if let Some(entries) = doc["known_denominators"].as_object() {
        for (key, entry) in entries {
            let _ = writeln!(
                out,
                "- `{key}` = {} — {}",
                entry
                    .get("value")
                    .map_or_else(String::new, Value::to_string),
                s(entry, "method")
            );
        }
    }
    out.push('\n');
}

fn md_readback(out: &mut String, doc: &Value) {
    let readback = &doc["bundle_readback"];
    if !readback.is_object() {
        return;
    }
    let _ = writeln!(out, "## Bundle readback\n");
    let _ = writeln!(
        out,
        "- Bundle `{}`, generation `{}`; {} artifacts, mismatches {}",
        s(readback, "bundle_dir"),
        short(s(&readback["manifest"], "generation_digest")),
        readback["artifacts"].as_array().map_or(0, Vec::len),
        readback["mismatches"].as_array().map_or(0, Vec::len)
    );
    for sheet in readback["sheets"].as_array().map_or(&[][..], Vec::as_slice) {
        let _ = writeln!(
            out,
            "- sheet `{}`: rows {}, cells {}, semantic digest `{}…`",
            s(sheet, "name"),
            u(sheet, "rows"),
            u(sheet, "cells"),
            short(s(sheet, "semantic_sha256"))
        );
    }
    out.push('\n');
}

fn md_prior_and_limits(out: &mut String, doc: &Value) {
    let _ = writeln!(out, "## Prior delta\n");
    for prior in doc["prior_delta"].as_array().map_or(&[][..], Vec::as_slice) {
        let _ = writeln!(
            out,
            "- {}: {} — {}",
            s(prior, "prior_id"),
            s(prior, "status"),
            s(prior, "note")
        );
    }
    let _ = writeln!(out, "\n## Limits\n");
    for limit in doc["limits"].as_array().map_or(&[][..], Vec::as_slice) {
        let _ = writeln!(
            out,
            "- {}",
            limit.as_str().map_or("", core::convert::identity)
        );
    }
    out.push('\n');
}

fn read_json(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
}

fn read_optional(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).with_context(|| format!("reading {}", path.display())),
    }
}

fn read_optional_json(path: &Path) -> Result<Option<Value>> {
    read_optional(path)?
        .map(|text| {
            serde_json::from_str(&text).with_context(|| format!("parsing {}", path.display()))
        })
        .transpose()
}

fn read_kv(path: &Path) -> Result<BTreeMap<String, String>> {
    let text = read_optional(path)?.unwrap_or_default();
    Ok(text
        .lines()
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect())
}

fn read_binaries(path: &Path) -> Result<Map<String, Value>> {
    let text = read_optional(path)?.unwrap_or_default();
    let mut out = Map::new();
    for line in text.lines() {
        if let Some((sha, path)) = line.split_once("  ") {
            let name = Path::new(path.trim())
                .file_name()
                .and_then(|name| name.to_str())
                .map_or_else(|| path.trim().to_string(), str::to_string);
            out.insert(name, Value::String(sha.trim().to_string()));
        }
    }
    Ok(out)
}

fn read_submission(path: &Path) -> Result<Option<Value>> {
    let Some(text) = read_optional(path)? else {
        return Ok(None);
    };
    let tokens: Vec<&str> = text.split_whitespace().collect();
    match tokens.as_slice() {
        [identity, "invocation", invocation, ..] => Ok(Some(
            json!({"run_identity": identity, "invocation": invocation}),
        )),
        _ => Ok(Some(json!({"raw": text.trim()}))),
    }
}

fn read_services(path: &Path) -> Result<u64> {
    let text = read_optional(path)?.unwrap_or_default();
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Ok(0);
    };
    Ok(value
        .get("services")
        .and_then(Value::as_array)
        .map_or(0, |services| services.len() as u64))
}

fn drain_certificate(log: &str) -> Option<String> {
    log.lines()
        .find_map(|line| line.split("drained: ").nth(1))
        .map(|rest| format!("drained: {rest}"))
}

pub(super) fn latest_sample(progress: &str) -> Option<Value> {
    let block = progress.rsplit("=== ").next()?;
    let sweeps: Vec<u64> = find_line(block, "jurisdiction sweeps owed: ")
        .map(|line| {
            line.split(" of ")
                .filter_map(|part| part.trim().parse().ok())
                .collect()
        })
        .unwrap_or_default();
    let invoations = block
        .lines()
        .find(|line| line.trim_start().starts_with('{'))
        .and_then(|line| serde_json::from_str::<Value>(line).ok());
    let mut owed = Vec::new();
    for line in block.lines() {
        if let Some(rest) = line.trim().split_once("x owes: ") {
            let count: u64 = rest.0.parse().map_or(0, core::convert::identity);
            owed.push(json!({"count": count, "stages": rest.1}));
        }
    }
    let source_objects = find_line(block, "source objects owed: ")
        .and_then(|value| value.trim().parse::<u64>().ok());
    Some(json!({
        "sampled_at": block
            .lines()
            .next()
            .map_or("", str::trim)
            .trim_matches(|character| matches!(character, '=' | ' ' | '"')),
        "sweeps_owed": sweeps.first().copied().map_or(Value::Null, |n| json!(n)),
        "sweeps_total": sweeps.get(1).copied().map_or(Value::Null, |n| json!(n)),
        "source_objects_owed": source_objects.map_or(Value::Null, |n| json!(n)),
        "invocations": invoations.map_or(Value::Null, core::convert::identity),
        "owed_sets": owed
    }))
}

fn find_line<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix(prefix))
}

fn frame_counts(path: &Path) -> Result<Value> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut header: Vec<String> = Vec::new();
    let mut website = usize::MAX;
    let mut zip = usize::MAX;
    let mut rows = 0u64;
    let mut website_claims = 0u64;
    let mut with_zip = 0u64;
    scan_csv(&text, |record, field, value| {
        if record == 0 {
            header.push(value.to_lowercase());
            return;
        }
        if website == usize::MAX {
            website = header
                .iter()
                .position(|name| name == "website")
                .map_or(usize::MAX, core::convert::identity);
            zip = header
                .iter()
                .position(|name| name == "zip")
                .map_or(usize::MAX, core::convert::identity);
        }
        if field == 0 {
            rows += 1;
        }
        if field == website && value.starts_with("http") {
            website_claims += 1;
        }
        if field == zip && !value.trim().is_empty() {
            with_zip += 1;
        }
    });
    Ok(json!({"rows": rows, "website_claims": website_claims, "with_zip": with_zip}))
}

fn scan_csv(text: &str, mut on_field: impl FnMut(usize, usize, &str)) {
    let mut record = 0usize;
    let mut field = 0usize;
    let mut value = String::new();
    let mut quoted = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                value.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => {
                on_field(record, field, &value);
                value.clear();
                field += 1;
            }
            '\n' if !quoted => {
                on_field(record, field, &value);
                value.clear();
                field = 0;
                record += 1;
            }
            '\r' if !quoted => {}
            _ => value.push(c),
        }
    }
    if field > 0 || !value.is_empty() {
        on_field(record, field, &value);
    }
}

fn store_bytes(run_dir: &Path) -> Result<Map<String, Value>> {
    let mut out = Map::new();
    for name in STORE_DIRS {
        let path = run_dir.join(name);
        let bytes = if path.exists() { dir_bytes(&path)? } else { 0 };
        out.insert(name.to_string(), json!(bytes));
    }
    Ok(out)
}

fn dir_bytes(path: &Path) -> Result<u64> {
    let mut total = 0u64;
    let entries = fs::read_dir(path).with_context(|| format!("reading {}", path.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading entry under {}", path.display()))?;
        let metadata = entry
            .metadata()
            .with_context(|| format!("stat {}", entry.path().display()))?;
        if metadata.is_dir() {
            total = total.saturating_add(dir_bytes(&entry.path())?);
        } else {
            total = total.saturating_add(metadata.len());
        }
    }
    Ok(total)
}

fn git(args: &[&str]) -> Result<String> {
    Cmd::new("git")
        .args(args.iter().map(|arg| (*arg).to_string()))
        .output()
        .with_context(|| format!("running git {}", args.join(" ")))
}

fn write_json(path: &Path, value: &Value) -> Result<()> {
    let text = serde_json::to_string_pretty(value)?;
    fs::write(path, format!("{text}\n")).with_context(|| format!("writing {}", path.display()))
}

fn relative(path: &Path) -> String {
    let root = crate::paths::repo_root();
    path.strip_prefix(&root).map_or_else(
        |_| path.display().to_string(),
        |rel| rel.display().to_string(),
    )
}

fn u(value: &Value, key: &str) -> u64 {
    value
        .get(key)
        .and_then(Value::as_u64)
        .map_or(0, core::convert::identity)
}

fn s<'a>(value: &'a Value, key: &str) -> &'a str {
    value
        .get(key)
        .and_then(Value::as_str)
        .map_or("", core::convert::identity)
}

fn short(text: &str) -> String {
    text.chars().take(16).collect()
}

fn list(value: &Value) -> String {
    value
        .as_array()
        .map_or_else(Vec::new, |items| {
            items
                .iter()
                .map(|item| {
                    item.as_str()
                        .map_or_else(|| item.to_string(), str::to_string)
                })
                .collect()
        })
        .join(", ")
}

fn object_entries<F: Fn(&str) -> String>(value: &Value, render: F) -> String {
    value
        .as_object()
        .map_or_else(Vec::new, |map| {
            map.iter()
                .map(|(key, value)| {
                    let shown = value.as_str().map_or_else(String::new, str::to_string);
                    format!("{key} {}", render(&shown))
                })
                .collect()
        })
        .join(", ")
}

fn dash(value: &Value) -> String {
    value
        .as_str()
        .map_or_else(|| "none recorded".to_string(), str::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csv_scanner_handles_quotes_and_newlines() {
        let mut fields: Vec<(usize, usize, String)> = Vec::new();
        scan_csv("a,b\n\"x,y\",\"line\nbreak\"\n", |record, field, value| {
            fields.push((record, field, value.to_string()));
        });
        assert_eq!(fields.len(), 4);
        assert_eq!(fields.get(2), Some(&(1, 0, "x,y".to_string())));
        assert_eq!(fields.get(3), Some(&(1, 1, "line\nbreak".to_string())));
    }

    #[test]
    fn csv_scanner_keeps_trailing_record() {
        let mut records = 0usize;
        scan_csv("a\nb", |record, field, _| {
            if record == 1 && field == 0 {
                records += 1;
            }
        });
        assert_eq!(records, 1);
    }

    #[test]
    fn sample_parses_watcher_blocks() {
        let text = "=== 2026-10-09T15:41:21Z ===\nseason 2026-27 revision 1\njurisdiction sweeps owed: 49 of 49\nsource objects owed: 56227\n{\"rows\":[{\"status\":\"running\",\"n\":33}]}\n       42x owes: teams, rosters\n";
        let sample = latest_sample(text).map_or(Value::Null, core::convert::identity);
        assert_eq!(
            sample.get("sampled_at"),
            Some(&json!("2026-10-09T15:41:21Z"))
        );
        assert_eq!(sample.get("sweeps_owed"), Some(&json!(49)));
        assert_eq!(sample.get("sweeps_total"), Some(&json!(49)));
        assert_eq!(sample.get("source_objects_owed"), Some(&json!(56227)));
        assert_eq!(sample.pointer("/owed_sets/0/count"), Some(&json!(42)));
    }

    #[test]
    fn open_work_summary_is_read_from_the_review_dir_not_the_run_dir() {
        let Ok(dir) = tempfile::tempdir() else {
            return;
        };
        let Ok(run) = tempfile::tempdir() else {
            return;
        };
        let report = run.path().join("school-address/current");
        let frame = run
            .path()
            .join("school-address/current/school_directory.csv");
        if fs::create_dir_all(&report).is_err() {
            return;
        }
        if fs::write(
            report.join("pipeline_report.json"),
            "{\"manifest_digest\":\"d\",\"now\":\"2026-10-09T16:00:00Z\",\"corpus\":{\"rows\":1,\"entries\":2,\"skipped\":0,\"notes\":0}}",
        )
        .is_err()
        {
            return;
        }
        if fs::write(&frame, "school,website,zip\n").is_err() {
            return;
        }
        if fs::write(
            dir.path().join("open-work-summary.json"),
            "{\"sampled_at\":\"2026-10-09T16:11:17Z\",\"source_objects_owed\":63605}",
        )
        .is_err()
        {
            return;
        }
        if fs::write(
            run.path().join("open-work-summary.json"),
            "{\"sampled_at\":\"1970-01-01T00:00:00Z\",\"source_objects_owed\":1}",
        )
        .is_err()
        {
            return;
        }
        let Ok(facts) = read_facts(dir.path(), run.path()) else {
            return;
        };
        let summary = facts.summary.map_or(Value::Null, core::convert::identity);
        assert_eq!(summary.get("source_objects_owed"), Some(&json!(63605)));
        assert_eq!(
            summary.get("sampled_at"),
            Some(&json!("2026-10-09T16:11:17Z"))
        );
    }

    #[test]
    fn submission_tokens_parse() {
        let Ok(dir) = tempfile::tempdir() else {
            return;
        };
        let path = dir.path().join("submission");
        if fs::write(&path, "national:2026-27:abcd:1 invocation inv_123\n").is_err() {
            return;
        }
        let parsed = read_submission(&path)
            .ok()
            .flatten()
            .map_or(Value::Null, core::convert::identity);
        assert_eq!(parsed.get("invocation"), Some(&json!("inv_123")));
    }

    #[test]
    fn drain_certificate_is_named() {
        let log = "something\ndrained: accepted=1 completed=2 cancelled=0 timed_out=0 aborted=0 panicked=0\n";
        assert_eq!(
            drain_certificate(log).as_deref(),
            Some("drained: accepted=1 completed=2 cancelled=0 timed_out=0 aborted=0 panicked=0")
        );
    }
}
