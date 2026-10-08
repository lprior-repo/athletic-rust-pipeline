use super::readback::{self, Verdict};
use super::{synthetic_corpus, Corpus, TestResult};
use census_domain::model::{CanonicalEvent, ExactSeconds, GradYear, Mark};
use census_store::Table;
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::path::Path;

fn write_snapshot<T: Serialize>(dir: &Path, table: Table, rows: &[T]) -> TestResult {
    let mut body = String::new();
    for row in rows {
        body.push_str(&serde_json::to_string(row)?);
        body.push('\n');
    }
    std::fs::write(readback::snapshot_file(dir, table), body)?;
    Ok(())
}

fn corpus_rows(corpus: &Corpus, table: Table) -> usize {
    match table {
        Table::Schools => corpus.schools.len(),
        Table::Teams => corpus.teams.len(),
        Table::Athletes => corpus.athletes.len(),
        Table::Meets => corpus.meets.len(),
        Table::Events => corpus.events.len(),
        Table::Performances => corpus.performances.len(),
        _ => 0,
    }
}

fn stage(dir: &Path, corpus: &Corpus) -> TestResult {
    for table in Table::ALL {
        write_snapshot(dir, table, &[] as &[Value])?;
    }
    write_snapshot(dir, Table::Schools, &corpus.schools)?;
    write_snapshot(dir, Table::Teams, &corpus.teams)?;
    write_snapshot(dir, Table::Athletes, &corpus.athletes)?;
    write_snapshot(dir, Table::Meets, &corpus.meets)?;
    write_snapshot(dir, Table::Events, &corpus.events)?;
    write_snapshot(dir, Table::Performances, &corpus.performances)?;
    Ok(())
}

fn reply(corpus: &Corpus, schools_rows: usize) -> Value {
    let tables: Vec<Value> = Table::ALL
        .into_iter()
        .map(|table| {
            let rows = if table == Table::Schools {
                schools_rows
            } else {
                corpus_rows(corpus, table)
            };
            json!({ "table": table.file(), "rows": rows })
        })
        .collect();
    json!({ "tables": tables })
}

pub(super) fn rejection(result: Result<(), String>, label: &str) -> TestResult<String> {
    match result {
        Ok(()) => Err(format!("the readback oracle accepted {label}").into()),
        Err(reason) => Ok(reason),
    }
}

#[test]
fn a_snapshot_oracle_rejects_duplicate_missing_changed_malformed_and_absent_rows() -> TestResult {
    let corpus = synthetic_corpus(2, 2)?;
    let dir = tempfile::tempdir()?;
    stage(dir.path(), &corpus)?;
    readback::verify_snapshots(dir.path(), &corpus)?;

    let kept = corpus
        .athletes
        .first()
        .cloned()
        .ok_or("the fixture corpus holds no athlete")?;
    let mut duplicated = corpus.athletes.clone();
    duplicated
        .pop()
        .ok_or("the fixture corpus holds no athlete")?;
    duplicated.push(kept);
    write_snapshot(dir.path(), Table::Athletes, &duplicated)?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "a duplicated athlete with a missing twin at the corpus's line count",
    )?;
    check!(
        reason.contains("athletes"),
        "the refusal must name the athletes table: {reason}"
    );
    stage(dir.path(), &corpus)?;

    let mut cohort = corpus.athletes.clone();
    let athlete = cohort
        .first_mut()
        .ok_or("the fixture corpus holds no athlete")?;
    athlete.grad_year = GradYear::new(2026).ok_or("invalid fixture cohort")?;
    write_snapshot(dir.path(), Table::Athletes, &cohort)?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "a changed athlete cohort",
    )?;
    check!(
        reason.contains("athletes"),
        "the refusal must name the athletes table: {reason}"
    );
    stage(dir.path(), &corpus)?;

    let mut identity = corpus.athletes.clone();
    let twin = identity
        .get(1)
        .map(|athlete| athlete.id.clone())
        .ok_or("the fixture corpus holds no second athlete")?;
    identity
        .first_mut()
        .ok_or("the fixture corpus holds no athlete")?
        .id = twin;
    write_snapshot(dir.path(), Table::Athletes, &identity)?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "a replaced athlete identity at the corpus's line count",
    )?;
    check!(
        reason.contains("athletes"),
        "the refusal must name the athletes table: {reason}"
    );
    stage(dir.path(), &corpus)?;

    let mut tampered = corpus.performances.clone();
    let performance = tampered
        .first_mut()
        .ok_or("the fixture corpus holds no performance")?;
    performance.mark = Mark::TimeSeconds(ExactSeconds::parse("12.70")?);
    performance.source_key.push_str("-tampered");
    write_snapshot(dir.path(), Table::Performances, &tampered)?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "a changed performance mark and provenance",
    )?;
    check!(
        reason.contains("performances"),
        "the refusal must name the performances table: {reason}"
    );
    stage(dir.path(), &corpus)?;

    let line = serde_json::to_string(
        corpus
            .athletes
            .first()
            .ok_or("the fixture corpus holds no athlete")?,
    )?;
    let mut body = String::from(
        line.get(..line.len() / 2)
            .ok_or("the fixture line is too short")?,
    );
    for row in corpus.athletes.iter().skip(1) {
        body.push('\n');
        body.push_str(&serde_json::to_string(row)?);
    }
    body.push('\n');
    std::fs::write(readback::snapshot_file(dir.path(), Table::Athletes), body)?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "a malformed nonblank athlete row",
    )?;
    check!(
        reason.contains("malformed"),
        "the refusal must name the malformed row: {reason}"
    );
    stage(dir.path(), &corpus)?;

    std::fs::write(readback::snapshot_file(dir.path(), Table::Schools), "")?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "an empty non-athlete snapshot",
    )?;
    check!(
        reason.contains("schools"),
        "the refusal must name the schools table: {reason}"
    );
    stage(dir.path(), &corpus)?;
    readback::verify_snapshots(dir.path(), &corpus)?;
    Ok(())
}

#[test]
fn b_status_output_verifier_accepts_only_completed_success_and_a_real_reply() -> TestResult {
    let id = "inv_fixture";
    let succeeded = json!({
        "id": id,
        "status": "completed",
        "completion_result": "success",
        "completion_failure": null,
    });
    check!(eq; readback::invocation_verdict(&succeeded, id), Verdict::Succeeded);

    let failed = json!({
        "id": id,
        "status": "completed",
        "completion_result": "failure",
        "completion_failure": "terminal boom",
    });
    match readback::invocation_verdict(&failed, id) {
        Verdict::Refused(reason) => check!(
            reason.contains("terminal boom"),
            "the refusal must retain the terminal failure: {reason}"
        ),
        other => return Err(format!("a completed failure must be refused, not {other:?}").into()),
    }

    let paused = json!({"id": id, "status": "paused", "completion_result": null});
    match readback::invocation_verdict(&paused, id) {
        Verdict::Refused(reason) => {
            check!(
                reason.contains("paused"),
                "the refusal must retain the paused status: {reason}"
            )
        }
        other => return Err(format!("a paused invocation must be refused, not {other:?}").into()),
    }

    let pending = json!({"id": id, "status": "pending", "completion_result": null});
    match readback::invocation_verdict(&pending, id) {
        Verdict::Pending(reason) => {
            check!(
                reason.contains("pending"),
                "the hold must retain the pending status: {reason}"
            )
        }
        other => return Err(format!("a pending invocation must be held, not {other:?}").into()),
    }

    let running = json!({"id": id, "status": "running", "completion_result": null});
    match readback::invocation_verdict(&running, id) {
        Verdict::Pending(reason) => {
            check!(
                reason.contains("running"),
                "the hold must retain the running status: {reason}"
            )
        }
        other => return Err(format!("a running invocation must be held, not {other:?}").into()),
    }

    check!(
        readback::single_invocation_row(&[], id).is_err(),
        "an absent invocation observation must be refused"
    );
    check!(
        readback::single_invocation_row(std::slice::from_ref(&succeeded), id).is_ok(),
        "one matching invocation observation must be accepted"
    );
    check!(
        readback::single_invocation_row(&[succeeded.clone(), succeeded.clone()], id).is_err(),
        "a duplicated invocation observation must be refused"
    );
    check!(
        readback::single_invocation_row(
            &[json!({"id": "inv_other", "status": "completed", "completion_result": "success"})],
            id,
        )
        .is_err(),
        "a foreign invocation must be refused"
    );
    check!(
        readback::single_invocation_row(&[json!({"status": "completed"})], id).is_err(),
        "an observation without an id must be refused"
    );

    check!(
        readback::require_reply_body(&json!({"message": "not ready"})).is_err(),
        "a not-ready output must be refused"
    );
    check!(
        readback::require_reply_body(&json!({"message": "not found"})).is_err(),
        "a missing output must be refused"
    );
    check!(
        readback::require_reply_body(&json!({"tables": []})).is_ok(),
        "a consolidation reply must be accepted"
    );
    Ok(())
}

#[test]
fn c_reply_oracle_rejects_overstated_omitted_and_unknown_tables() -> TestResult {
    let corpus = synthetic_corpus(2, 2)?;
    let dir = tempfile::tempdir()?;
    stage(dir.path(), &corpus)?;
    readback::verify_reply_tables(&reply(&corpus, corpus.schools.len()), dir.path())?;

    let overstated = reply(&corpus, corpus.schools.len() + 1);
    let reason = rejection(
        readback::verify_reply_tables(&overstated, dir.path()),
        "a reply that overstates its school rows",
    )?;
    check!(
        reason.contains("schools"),
        "the refusal must name the schools table: {reason}"
    );

    let mut omitted = reply(&corpus, corpus.schools.len());
    let entries = omitted
        .get_mut("tables")
        .and_then(Value::as_array_mut)
        .ok_or("the fixture reply holds no tables")?;
    entries.retain(|entry| entry.get("table").and_then(Value::as_str) != Some("events"));
    rejection(
        readback::verify_reply_tables(&omitted, dir.path()),
        "a reply that omits the events table",
    )?;

    let unknown = json!({"tables": [{"table": "thermometers", "rows": 0}]});
    rejection(
        readback::verify_reply_tables(&unknown, dir.path()),
        "a reply that names an unknown table",
    )?;

    let truncated = reply(&corpus, corpus.schools.len());
    let mut body = serde_json::to_string(&truncated)?;
    body.truncate(body.len() / 2);
    std::fs::write(readback::snapshot_file(dir.path(), Table::Schools), body)?;
    rejection(
        readback::verify_reply_tables(&truncated, dir.path()),
        "a snapshot whose last row is truncated",
    )?;
    Ok(())
}

#[test]
fn f_canonical_event_snapshots_collapse_duplicate_observations_and_still_require_every_event(
) -> TestResult {
    let corpus = synthetic_corpus(4, 5)?;
    let mut seen = BTreeSet::new();
    let canonical: Vec<CanonicalEvent> = corpus
        .events
        .iter()
        .filter(|event| seen.insert(event.id.clone()))
        .cloned()
        .collect();
    check!(
        canonical.len() < corpus.events.len(),
        "the fixture corpus repeats one event observation across the athletes of a gender"
    );
    let dir = tempfile::tempdir()?;
    stage(dir.path(), &corpus)?;
    write_snapshot(dir.path(), Table::Events, &canonical)?;
    readback::verify_snapshots(dir.path(), &corpus)?;

    let mut short = canonical.clone();
    short.pop().ok_or("the fixture corpus holds no event")?;
    write_snapshot(dir.path(), Table::Events, &short)?;
    let reason = rejection(
        readback::verify_snapshots(dir.path(), &corpus),
        "a canonical events snapshot missing a row the corpus observed",
    )?;
    check!(
        reason.contains("events"),
        "the refusal must name the events table: {reason}"
    );
    Ok(())
}
