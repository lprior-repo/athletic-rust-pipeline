use super::{StatusCase, TestResult};
use census_crawl::athleticlive_athletes::MeetTarget;
use census_domain::model::{CanonicalMeet, CompetitionLevel};
use census_domain::UsJurisdiction;
use serde_json::{json, Value};

pub(super) const XC: &str = "xc";
pub(super) const FIELD: &str = "field";
pub(super) const CAPTURES: [&str; 2] = [XC, FIELD];
pub(super) const OBSERVED_ON: &str = "2026-09-22";

pub(super) fn cases() -> TestResult<Vec<StatusCase>> {
    Ok(serde_json::from_str(include_str!(
        "../../../census-crawl/tests/fixtures/cen17/status-cases.json"
    ))?)
}

pub(super) fn captured(kind: &str) -> TestResult<Value> {
    let body = match kind {
        XC => include_str!(
            "../../../census-crawl/tests/fixtures/athleticlive_results/event-doc-2150205.json"
        ),
        FIELD => include_str!(
            "../../../census-crawl/tests/fixtures/athleticlive_results/event-doc-2254280.json"
        ),
        _ => return Err("unknown owned capture".into()),
    };
    Ok(serde_json::from_str(body)?)
}

pub(super) fn control_row(source: &mut Value) -> TestResult<Value> {
    let id = if source.pointer("/_source/xc") == Some(&json!(true)) {
        26_401_881
    } else {
        17_918_436
    };
    let rows = source
        .pointer_mut("/_source/r")
        .and_then(Value::as_array_mut)
        .ok_or("captured rows")?;
    let index = rows
        .iter()
        .position(|row| row.pointer("/a/ani") == Some(&json!(id)))
        .ok_or("captured cohort control")?;
    rows.get_mut(index)
        .map(Value::take)
        .ok_or_else(|| "captured cohort control".into())
}

pub(super) fn set(target: &mut Value, path: &str, value: Value) -> TestResult {
    *target
        .pointer_mut(path)
        .ok_or_else(|| format!("missing captured field {path}"))? = value;
    Ok(())
}

pub(super) fn challenge(kind: &str, display: Option<String>, validity: Value) -> TestResult<Value> {
    let mut source = captured(kind)?;
    let mut row = control_row(&mut source)?;
    set(
        &mut row,
        "/m",
        display
            .map(Value::String)
            .map_or(Value::Null, core::convert::identity),
    )?;
    set(
        &mut row,
        "/im",
        json!(if kind == FIELD { 99_990_000 } else { 1 }),
    )?;
    set(&mut row, "/vm", validity)?;
    set(&mut source, "/_source/r", Value::Array(vec![row]))?;
    Ok(source)
}

pub(super) fn target(kind: &str) -> TestResult<MeetTarget> {
    let (id, name, state, date) = match kind {
        XC => (
            58_504,
            "Iowa High School State Championships",
            UsJurisdiction::Iowa,
            "2025-10-31",
        ),
        FIELD => (61_710, "MITS 4", UsJurisdiction::Michigan, "2026-02-14"),
        _ => return Err("unknown meet target".into()),
    };
    let meet = CanonicalMeet::new(Some(state), name, date, CompetitionLevel::Invitational);
    Ok(MeetTarget {
        athleticlive_meet_id: id,
        meet_id: meet.id.to_string(),
        tenant: "live_results".to_string(),
        name: name.to_string(),
        state,
        date: date.to_string(),
    })
}

pub(super) fn documents(kind: &str) -> TestResult<(Value, Value)> {
    let mut valid = captured(kind)?;
    let control = control_row(&mut valid)?;
    set(&mut valid, "/_source/r", Value::Array(vec![control]))?;
    let mut invalid = captured(kind)?;
    let mut rows = Vec::new();
    for (index, case) in cases()?.into_iter().enumerate() {
        let mut source = challenge(kind, case.display, case.validity)?;
        let mut row = source
            .pointer_mut("/_source/r/0")
            .map(Value::take)
            .ok_or("challenger")?;
        if index != 0 {
            set(
                &mut row,
                "/a/ani",
                json!(9_900_000_u64
                    .checked_add(u64::try_from(index)?)
                    .ok_or("id overflow")?),
            )?;
            set(
                &mut row,
                "/a/n",
                Value::String(format!("CEN17 {}", case.name)),
            )?;
        }
        rows.push(row);
    }
    set(&mut invalid, "/_source/r", Value::Array(rows))?;
    set(&mut invalid, "/_source/i", json!(9_170_001))?;
    set(&mut invalid, "/_source/runm", json!("Prelims"))?;
    set(&mut invalid, "/_source/rui", json!("cen17-prelims"))?;
    Ok((valid, invalid))
}
