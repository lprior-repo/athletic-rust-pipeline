use crate::{AdapterContext, CrawlError, CrawlResult};
use serde::Serialize;
use serde_json::Value;

pub(super) fn payload<T: Serialize>(phase: &str, value: &T) -> CrawlResult<Value> {
    serde_json::to_value(value).map_err(|source| CrawlError::Encode {
        table: format!("journal:{phase}"),
        source,
    })
}

pub(super) fn digest<T: Serialize + ?Sized>(value: &T) -> CrawlResult<String> {
    census_domain::model::serialized_digest(value).map_err(|source| CrawlError::Canonical {
        table: "owned acquisition journal".into(),
        source,
    })
}

pub(super) fn stage(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    phase: &str,
    key: &str,
    payload: &Value,
) -> CrawlResult<bool> {
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(phase, key, |prior| matching(prior, phase, key, payload))? {
            return Ok(false);
        }
    }
    let prior = ctx.store.journal_payload(phase, key)?;
    if matching(prior.as_ref(), phase, key, payload)? {
        return Ok(false);
    }
    batch.journal_done(phase, key, payload)?;
    Ok(true)
}

pub(super) fn stage_capture(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    phase: &str,
    key: &str,
    payload: &Value,
) -> CrawlResult<bool> {
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(phase, key, |prior| {
            matching_capture(prior, phase, key, payload)
        })? {
            return Ok(false);
        }
    }
    let prior = ctx.store.journal_payload(phase, key)?;
    if matching_capture(prior.as_ref(), phase, key, payload)? {
        return Ok(false);
    }
    batch.journal_done(phase, key, payload)?;
    Ok(true)
}

pub(super) fn contains(ctx: &AdapterContext<'_>, phase: &str, key: &str) -> CrawlResult<bool> {
    if let Some(recording) = ctx.recording {
        if recording.inspect_journal(phase, key, |prior| Ok(prior.is_some()))? {
            return Ok(true);
        }
    }
    Ok(ctx.store.journal_contains(phase, key)?)
}

fn matching(prior: Option<&Value>, phase: &str, key: &str, payload: &Value) -> CrawlResult<bool> {
    match prior {
        Some(prior) if prior == payload => Ok(true),
        Some(_) => Err(conflict(phase, key)),
        None => Ok(false),
    }
}

fn matching_capture(
    prior: Option<&Value>,
    phase: &str,
    key: &str,
    payload: &Value,
) -> CrawlResult<bool> {
    match prior {
        Some(prior) if same_capture(prior, payload) => Ok(true),
        Some(_) => Err(conflict(phase, key)),
        None => Ok(false),
    }
}

fn same_capture(prior: &Value, payload: &Value) -> bool {
    let (Some(prior), Some(payload)) = (prior.as_object(), payload.as_object()) else {
        return false;
    };
    prior.contains_key("capture")
        && payload.contains_key("capture")
        && prior.len() == payload.len()
        && prior.iter().all(|(key, value)| {
            if key == "capture" {
                payload
                    .get(key)
                    .is_some_and(|next| same_acquisition(value, next))
            } else {
                payload.get(key) == Some(value)
            }
        })
}

fn same_acquisition(prior: &Value, payload: &Value) -> bool {
    let (Some(prior), Some(payload)) = (prior.as_object(), payload.as_object()) else {
        return false;
    };
    prior.get("from_cache").is_some()
        && payload.get("from_cache").is_some()
        && prior.len() == payload.len()
        && prior
            .iter()
            .all(|(key, value)| key == "from_cache" || payload.get(key) == Some(value))
}

fn conflict(phase: &str, key: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("immutable owned receipt changed: {phase}/{key}"),
    }
}
