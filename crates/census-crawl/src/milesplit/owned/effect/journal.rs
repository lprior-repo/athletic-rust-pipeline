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
    match ctx.store.journal_payload(phase, key)? {
        Some(prior) if &prior == payload => Ok(false),
        Some(_) => Err(conflict(phase, key)),
        None => {
            batch.journal_done(phase, key, payload)?;
            Ok(true)
        }
    }
}

pub(super) fn stage_capture(
    ctx: &AdapterContext<'_>,
    batch: &mut crate::recording::RowBatch<'_>,
    phase: &str,
    key: &str,
    payload: &Value,
) -> CrawlResult<bool> {
    let Some(mut prior) = ctx.store.journal_payload(phase, key)? else {
        batch.journal_done(phase, key, payload)?;
        return Ok(true);
    };
    let flag = prior
        .get_mut("capture")
        .and_then(Value::as_object_mut)
        .and_then(|capture| capture.get_mut("from_cache"))
        .ok_or_else(|| conflict(phase, key))?;
    *flag = payload
        .get("capture")
        .and_then(|capture| capture.get("from_cache"))
        .ok_or_else(|| conflict(phase, key))?
        .clone();
    if &prior != payload {
        return Err(conflict(phase, key));
    }
    Ok(false)
}

fn conflict(phase: &str, key: &str) -> CrawlError {
    CrawlError::Invariant {
        detail: format!("immutable owned receipt changed: {phase}/{key}"),
    }
}
