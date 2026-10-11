use super::format::{count, number, s, u_opt};
use super::model::{Curated, Facts};
use serde_json::{json, Map, Value};

pub(super) fn assemble(
    curated: &Curated,
    facts: &Facts,
    run_rel: &str,
    reviewed_at: &str,
) -> Value {
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
        "scope": scope(meta, facts, run_rel, u_opt(matrix, "jurisdiction_count")),
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
        json!({"value": count(matrix, "jurisdiction_count"), "method": "UsJurisdiction::CENSUS_SCOPE"}),
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
        "registered_families": count(matrix, "registered_families"),
        "armed_families": count(matrix, "armed_families"),
        "gap_families": count(matrix, "gap_families"),
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

fn scope(meta: &Value, facts: &Facts, run_rel: &str, jurisdiction_count: Option<u64>) -> Value {
    json!({
        "cohort": count(meta, "cohort"),
        "season": s(meta, "season"),
        "revision": count(meta, "revision"),
        "jurisdictions": number(jurisdiction_count),
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
