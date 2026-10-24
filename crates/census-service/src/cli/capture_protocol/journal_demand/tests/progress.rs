use super::super::{run_with_progress, write_progress};
use super::{current, expected, invoke, TestResult};
use census_crawl::milesplit::{OWNED_CAPTURE_PHASE, OWNED_MEET_PHASE};
use census_store::{Store, StoreError};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;

const STAGES: [&str; 9] = [
    "store_open_started",
    "store_open_completed",
    "snapshot_held",
    "capture_phase_started",
    "capture_phase_completed",
    "meet_phase_started",
    "meet_phase_completed",
    "counter_completed",
    "output_completed",
];

mod writes;
use writes::{sentinel, Writer};

#[test]
fn actual_records_are_bounded_exact_and_flushed_before_next_stage_or_stdout() -> TestResult {
    for populated in [false, true] {
        let directory = current()?;
        let root = directory.path().join("store");
        let rows = if populated {
            vec![
                (OWNED_CAPTURE_PHASE, "row/a", Value::Null),
                (OWNED_MEET_PHASE, "row/z", json!({"escape": "\n\\é"})),
            ]
        } else {
            Vec::new()
        };
        {
            let (store, _writer) = Store::open_current_qualification_v3(&root)?.into_parts();
            for (phase, key, value) in &rows {
                store.journal_done(phase, key, value)?;
            }
        }
        let trace = Rc::new(RefCell::new(Vec::new()));
        let mut output = Writer::new("stdout", trace.clone());
        let mut progress = Writer::new("", trace.clone());
        run_with_progress(&root, &mut output, |stage| {
            progress.label = stage.into();
            write_progress(&mut progress, stage)
        })?;
        let lines = progress
            .bytes
            .split_inclusive(|byte| *byte == b'\n')
            .collect::<Vec<_>>();
        check!(eq; lines.len(), 9);
        let mut records = Vec::new();
        for (line, stage) in lines.into_iter().zip(STAGES) {
            check!(line.len() <= 512);
            check!(line.ends_with(b"\n"));
            let record: Value = serde_json::from_slice(line)?;
            check!(eq; record, json!({"schema": "journal-demand-progress-v1", "kind": "journal_demand_progress", "stage": stage}));
            let mut canonical = serde_json::to_vec(&record)?;
            canonical.push(b'\n');
            check!(eq; line, canonical);
            records.push(record);
        }
        check!(eq; records.len(), 9);
        check!(progress.bytes.len() <= 4608);
        let mut expected_trace = STAGES
            .iter()
            .take(8)
            .flat_map(|stage| [format!("{stage}:body"), format!("{stage}:flush")])
            .collect::<Vec<_>>();
        expected_trace.extend([
            "stdout:body".into(),
            "stdout:lf".into(),
            "stdout:flush".into(),
            "output_completed:body".into(),
            "output_completed:flush".into(),
        ]);
        check!(eq; *trace.borrow(), expected_trace);
        check!(eq; serde_json::from_slice::<Value>(&output.bytes)?, expected(&root, &rows)?);
        let mut parsed_output = Vec::new();
        invoke(Some(&root), &mut parsed_output)?;
        check!(eq; output.bytes, parsed_output);
    }
    Ok(())
}

#[test]
fn all_nine_sink_failures_propagate_and_stop_with_correct_stdout_state() -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    for (index, refused) in STAGES.iter().enumerate() {
        let mut attempted = Vec::new();
        let mut output = Vec::new();
        let error = run_with_progress(&root, &mut output, |stage| {
            attempted.push(stage);
            if stage == *refused {
                Err(io::Error::new(io::ErrorKind::BrokenPipe, "stage sentinel"))
            } else {
                Ok(())
            }
        })
        .err()
        .ok_or("sink failure suppressed")?;
        sentinel(&error, "stage sentinel")?;
        check!(eq; attempted, STAGES.iter().take(index.saturating_add(1)).copied().collect::<Vec<_>>());
        if *refused == "output_completed" {
            check!(eq; serde_json::from_slice::<Value>(&output)?, expected(&root, &[])?);
        } else {
            check!(output.is_empty());
        }
    }
    Ok(())
}

#[test]
fn output_body_lf_and_flush_failures_omit_output_completed() -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    for failure in ["body", "lf", "flush"] {
        let mut output = Writer::new("stdout", Rc::new(RefCell::new(Vec::new())));
        output.fail = Some(failure);
        let mut stages = Vec::new();
        let error = run_with_progress(&root, &mut output, |stage| {
            stages.push(stage);
            Ok(())
        })
        .err()
        .ok_or("stdout failure suppressed")?;
        sentinel(&error, "writer sentinel")?;
        check!(eq; stages, STAGES.iter().take(8).copied().collect::<Vec<_>>());
        match failure {
            "body" => check!(output.bytes.is_empty()),
            "lf" => check!(!output.bytes.ends_with(b"\n")),
            _ => check!(eq; serde_json::from_slice::<Value>(&output.bytes)?, expected(&root, &[])?),
        }
    }
    Ok(())
}

#[test]
fn preopen_sink_failure_precedes_missing_root_and_explicit_opener_refusals_stop() -> TestResult {
    let directory = tempfile::tempdir()?;
    let root = directory.path().join("absent");
    let mut output = Vec::new();
    let mut stages = Vec::new();
    let error = run_with_progress(&root, &mut output, |stage| {
        stages.push(stage);
        Err(io::Error::new(
            io::ErrorKind::BrokenPipe,
            "preopen sentinel",
        ))
    })
    .err()
    .ok_or("preopen sink accepted")?;
    sentinel(&error, "preopen sentinel")?;
    check!(eq; stages, vec!["store_open_started"]);
    check!(!root.exists());
    let historical = directory.path().join("historical");
    Store::create_historical_capture_v1(&historical)?;
    for path in [&root, &historical] {
        stages.clear();
        let error = run_with_progress(path, &mut output, |stage| {
            stages.push(stage);
            Ok(())
        })
        .err()
        .ok_or("invalid root accepted")?;
        if path == &root {
            check!(
                matches!(error.downcast_ref::<StoreError>(), Some(StoreError::Io { source, .. }) if source.kind() == io::ErrorKind::NotFound)
            );
        } else {
            check!(matches!(
                error.downcast_ref::<StoreError>(),
                Some(StoreError::Capture(
                    census_store::CaptureError::CaptureProtocolUnsupported
                ))
            ));
        }
        check!(eq; stages, vec!["store_open_started"]);
        check!(output.is_empty());
    }
    Ok(())
}

#[test]
fn actual_measurement_refusal_keeps_cause_and_never_emits_success_bytes() -> TestResult {
    let directory = current()?;
    let root = directory.path().join("store");
    {
        let (store, _writer) = Store::open_current_qualification_v3(&root)?.into_parts();
        store.journal_done(OWNED_MEET_PHASE, &"k".repeat(513), &Value::Null)?;
    }
    let mut output = Vec::new();
    let mut stages = Vec::new();
    let error = run_with_progress(&root, &mut output, |stage| {
        stages.push(stage);
        Ok(())
    })
    .err()
    .ok_or("measurement refusal suppressed")?;
    check!(
        matches!(error.downcast_ref::<StoreError>(), Some(StoreError::Refused { detail }) if detail == "unsupported retained source journal key")
    );
    check!(eq; stages, STAGES.iter().take(6).copied().collect::<Vec<_>>());
    check!(output.is_empty());
    Ok(())
}
