#![forbid(unsafe_code)]

mod assemble;
mod format;
mod frame;
mod input;
mod model;
mod readers;
mod render;
mod samples;
mod storage;

pub(super) use samples::latest_sample;

use assemble::assemble;
use format::{display, s, shown, tally};
use input::{read_curated, read_facts};
use render::render_md;
use storage::{relative, write_json};

use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

const REVIEW_JSON: &str = "census-completeness-review.json";
const REVIEW_MD: &str = "census-completeness-review.md";
const STORE_DIRS: [&str; 4] = ["fjall", "http", "out", "restate"];

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
    let source_matrix = doc
        .get("source_matrix")
        .map_or(&serde_json::Value::Null, core::convert::identity);
    let md = render_md(&doc)?;
    fs::write(dir.join(REVIEW_MD), md)
        .with_context(|| format!("writing {}", dir.join(REVIEW_MD).display()))?;
    println!(
        "wrote {} and {}",
        dir.join(REVIEW_JSON).display(),
        dir.join(REVIEW_MD).display()
    );
    println!(
        "families={} armed={} findings={} gates={} verdict={}",
        display(source_matrix.get("registered_families")),
        display(source_matrix.get("armed_families")),
        shown(doc.get("findings").and_then(tally)),
        shown(doc.get("gates").and_then(tally)),
        s(&doc, "verdict")
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::review::build::frame::scan_csv;
    use crate::review::build::readers::{drain_certificate, read_submission};
    use serde_json::{json, Value};

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
