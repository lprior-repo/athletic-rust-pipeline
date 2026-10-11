use super::format::{dash, list, object_entries, s, short, u};
use serde_json::Value;
use std::fmt::Write as _;

fn field<'a>(value: &'a Value, key: &str) -> &'a Value {
    value.get(key).map_or(&Value::Null, core::convert::identity)
}

pub(super) fn render_md(doc: &Value) -> Result<String, std::fmt::Error> {
    let mut out = String::new();
    writeln!(
        out,
        "# Census completeness review — {}\n",
        s(doc, "reviewed_at")
    )?;
    md_header(&mut out, doc)?;
    md_scope(&mut out, doc)?;
    md_run(&mut out, doc)?;
    md_matrix(&mut out, doc)?;
    md_gates(&mut out, doc)?;
    md_findings(&mut out, doc)?;
    md_denominators(&mut out, doc)?;
    md_readback(&mut out, doc)?;
    md_prior_and_limits(&mut out, doc)?;
    Ok(out)
}

fn md_header(out: &mut String, doc: &Value) -> std::fmt::Result {
    writeln!(
        out,
        "**Verdict: {}.** {}\n",
        s(doc, "verdict"),
        s(doc, "verdict_scope")
    )?;
    writeln!(
        out,
        "Schema `{}` · repository `{}` · commit `{}` ({})\n",
        s(doc, "schema_version"),
        s(doc, "repository"),
        s(doc, "commit"),
        s(doc, "commit_subject")
    )?;
    writeln!(
        out,
        "Reviewer: {} · host-reported model: `{}`\n",
        s(field(doc, "reviewer"), "identity"),
        s(field(doc, "reviewer"), "model_reported_by_host")
    )
}

fn md_scope(out: &mut String, doc: &Value) -> std::fmt::Result {
    let scope = field(doc, "scope");
    writeln!(out, "## Scope\n")?;
    writeln!(
        out,
        "- Cohort {}, season {}, revision {}, {} jurisdictions, surfaces {}\n",
        u(scope, "cohort"),
        s(scope, "season"),
        u(scope, "revision"),
        u(scope, "jurisdictions"),
        list(field(scope, "surfaces"))
    )?;
    writeln!(
        out,
        "- Run `{}` · invocation `{}` · root `{}` · submitted {}\n",
        s(scope, "run_identity"),
        s(scope, "invocation"),
        s(scope, "run_root"),
        s(scope, "as_of")
    )
}

fn md_run(out: &mut String, doc: &Value) -> std::fmt::Result {
    let run = field(doc, "run");
    writeln!(out, "## Run state\n")?;
    writeln!(
        out,
        "- Binaries: {}",
        object_entries(field(run, "binaries"), |sha| format!("`{}…`", short(sha)))
    )?;
    let generation = field(field(run, "generation"), "pipeline_report");
    writeln!(
        out,
        "- School-directory generation `{}…` — corpus {} — {} deployment services",
        short(s(generation, "manifest_digest")),
        generation
            .get("corpus")
            .map_or_else(String::new, Value::to_string),
        u(run, "deployment_services")
    )?;
    let sample = field(run, "latest_progress");
    writeln!(
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
    )?;
    writeln!(
        out,
        "- Drain certificate: {}\n",
        dash(field(run, "drain_certificate"))
    )
}

fn md_matrix(out: &mut String, doc: &Value) -> std::fmt::Result {
    let matrix = field(doc, "source_matrix");
    writeln!(out, "## Source matrix\n")?;
    writeln!(
        out,
        "- {} registered families, {} with a durable arm, {} explicit engineering gaps, {} arm slugs outside the registry\n",
        u(matrix, "registered_families"),
        u(matrix, "armed_families"),
        u(matrix, "gap_families"),
        field(matrix, "arms_not_registered")
            .as_array()
            .map_or(0, Vec::len)
    )?;
    if field(matrix, "provenance").is_object() {
        writeln!(
            out,
            "- matrix provenance: `{}` produced by {}\n",
            s(field(matrix, "provenance"), "path"),
            s(field(matrix, "provenance"), "producer")
        )?;
    }
    writeln!(out, "| family | applicable | strategy | arm or gap |")?;
    writeln!(out, "|---|---|---|---|")?;
    for family in field(matrix, "families").as_array().into_iter().flatten() {
        let arms = field(family, "arms").as_array().map_or(0, Vec::len);
        let detail = if arms > 0 {
            list(field(family, "arms"))
        } else {
            s(family, "gap_reason").to_string()
        };
        writeln!(
            out,
            "| {} | {} | {} | {} |",
            s(family, "slug"),
            field(family, "applicable").as_array().map_or(0, Vec::len),
            s(family, "strategy"),
            detail
        )?;
    }
    out.push('\n');
    Ok(())
}

fn md_gates(out: &mut String, doc: &Value) -> std::fmt::Result {
    writeln!(out, "## Gates\n")?;
    for gate in field(doc, "gates").as_array().into_iter().flatten() {
        writeln!(
            out,
            "- **{} {} — {}.** {}",
            s(gate, "id"),
            s(gate, "title"),
            s(gate, "state"),
            s(gate, "note")
        )?;
    }
    out.push('\n');
    Ok(())
}

fn md_findings(out: &mut String, doc: &Value) -> std::fmt::Result {
    writeln!(out, "## Findings\n")?;
    for finding in field(doc, "findings").as_array().into_iter().flatten() {
        writeln!(
            out,
            "### {} [{}] {} — {}\n",
            s(finding, "id"),
            s(finding, "severity"),
            s(finding, "title"),
            s(finding, "status")
        )?;
        writeln!(
            out,
            "- evidence: {}; runtime reproduced: {}\n",
            s(finding, "evidence_kind"),
            finding
                .get("runtime_reproduced")
                .map_or_else(String::new, Value::to_string)
        )?;
        for location in field(finding, "locations").as_array().into_iter().flatten() {
            writeln!(
                out,
                "- location: `{}`",
                location.as_str().map_or("", core::convert::identity)
            )?;
        }
        writeln!(out, "- problem: {}", s(finding, "problem"))?;
        writeln!(
            out,
            "- counterexample: {}",
            s(finding, "minimal_counterexample")
        )?;
        writeln!(out, "- remediation: {}", s(finding, "remediation"))?;
        writeln!(out, "- regression: {}\n", s(finding, "future_regression"))?;
    }
    Ok(())
}

fn md_denominators(out: &mut String, doc: &Value) -> std::fmt::Result {
    writeln!(out, "## Known denominators\n")?;
    if let Some(entries) = field(doc, "known_denominators").as_object() {
        for (key, entry) in entries {
            writeln!(
                out,
                "- `{key}` = {} — {}",
                entry
                    .get("value")
                    .map_or_else(String::new, Value::to_string),
                s(entry, "method")
            )?;
        }
    }
    out.push('\n');
    Ok(())
}

fn md_readback(out: &mut String, doc: &Value) -> std::fmt::Result {
    let readback = field(doc, "bundle_readback");
    if !readback.is_object() {
        return Ok(());
    }
    writeln!(out, "## Bundle readback\n")?;
    writeln!(
        out,
        "- Bundle `{}`, generation `{}`; {} artifacts, mismatches {}",
        s(readback, "bundle_dir"),
        short(s(field(readback, "manifest"), "generation_digest")),
        field(readback, "artifacts").as_array().map_or(0, Vec::len),
        field(readback, "mismatches").as_array().map_or(0, Vec::len)
    )?;
    for sheet in field(readback, "sheets").as_array().into_iter().flatten() {
        writeln!(
            out,
            "- sheet `{}`: rows {}, cells {}, semantic digest `{}…`",
            s(sheet, "name"),
            u(sheet, "rows"),
            u(sheet, "cells"),
            short(s(sheet, "semantic_sha256"))
        )?;
    }
    out.push('\n');
    Ok(())
}

fn md_prior_and_limits(out: &mut String, doc: &Value) -> std::fmt::Result {
    writeln!(out, "## Prior delta\n")?;
    for prior in field(doc, "prior_delta").as_array().into_iter().flatten() {
        writeln!(
            out,
            "- {}: {} — {}",
            s(prior, "prior_id"),
            s(prior, "status"),
            s(prior, "note")
        )?;
    }
    writeln!(out, "\n## Limits\n")?;
    for limit in field(doc, "limits").as_array().into_iter().flatten() {
        writeln!(
            out,
            "- {}",
            limit.as_str().map_or("", core::convert::identity)
        )?;
    }
    out.push('\n');
    Ok(())
}
