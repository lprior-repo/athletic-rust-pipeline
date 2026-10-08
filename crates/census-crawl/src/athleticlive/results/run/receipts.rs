use super::super::{DocumentEntities, EffectReceipt, EFFECT_PHASE, PARSER};
use crate::net::cache::CacheMeta;
use crate::{AdapterContext, CrawlResult};
use serde_json::json;

pub(super) fn require_owner(
    ctx: &AdapterContext<'_>,
    path: &str,
    meta: &CacheMeta,
    run: &super::Run,
) -> CrawlResult<()> {
    let keys = ctx.store.journal_keys(EFFECT_PHASE)?;
    if keys.len() > super::MAX_CAPTURE_RECORDS {
        return Err(super::resource("LIVE receipt index", keys.len()));
    }
    let meet = run.target.athleticlive_meet_id.to_string();
    for key in keys {
        let payload = ctx
            .store
            .journal_payload(EFFECT_PHASE, &key)?
            .ok_or_else(|| super::schema(path, "current receipt has no payload"))?;
        require_shape(path, &key, &payload)?;
        if payload.get("path").and_then(serde_json::Value::as_str) == Some(path)
            || payload.get("digest").and_then(serde_json::Value::as_str)
                == Some(meta.content_digest.as_str())
        {
            if payload.get("meet").and_then(serde_json::Value::as_str) != Some(meet.as_str())
                || payload.get("provider").and_then(serde_json::Value::as_str)
                    != Some(run.target.tenant.as_str())
            {
                return Err(super::schema(
                    path,
                    "capture belongs to another meet/provider; refusing to project",
                ));
            }
        }
    }
    Ok(())
}

fn require_shape(path: &str, key: &str, payload: &serde_json::Value) -> CrawlResult<()> {
    let meet = payload
        .get("meet")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()));
    let projection = payload
        .get("projection")
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()));
    if meet.is_none()
        || projection.is_none()
        || payload.get("parser").and_then(serde_json::Value::as_str) != Some(PARSER)
        || payload
            .get("complete")
            .and_then(serde_json::Value::as_bool)
            .is_none()
        || payload
            .pointer("/metadata/url")
            .and_then(serde_json::Value::as_str)
            .is_none()
        || payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .is_none()
        || payload
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .is_none()
    {
        return Err(super::schema(
            path,
            "ambiguous or incompatible LIVE projection receipt shape",
        ));
    }
    let digest = census_domain::model::serialized_digest(payload).map_err(|error| {
        crate::CrawlError::Invariant {
            detail: error.to_string(),
        }
    })?;
    let role = payload
        .get("role")
        .and_then(serde_json::Value::as_str)
        .map_or("refused", |value| value);
    let owner = meet.ok_or_else(|| super::schema(path, "receipt owner is absent"))?;
    if key != format!("{owner}:{role}:{digest}") {
        return Err(super::schema(
            path,
            "receipt key disagrees with projection payload",
        ));
    }
    Ok(())
}

pub(super) fn receipt(
    run: &super::Run,
    capture: (&str, &str, &CacheMeta),
    entities: &DocumentEntities,
    complete: bool,
    parsed: Option<serde_json::Value>,
) -> CrawlResult<EffectReceipt> {
    let (path, _, metadata) = capture;
    let projected = census_domain::model::serialized_digest(&(
        &entities.meets,
        &entities.events,
        &entities.teams,
        &entities.athletes,
        &entities.performances,
        &entities.review_cases,
        &entities.source_observations,
    ))
    .map_err(|error| crate::CrawlError::Invariant {
        detail: error.to_string(),
    })?;
    let payload = json!({
        "path":path, "role":parsed.as_ref().and_then(|value| value.get("role")),
        "meet":run.target.athleticlive_meet_id.to_string(), "provider":run.target.tenant,
        "meet_name":run.target.name, "meet_date":run.target.date, "state":run.target.state,
        "observed_on":metadata.fetched_at, "school_year":run.school_year,
        "performance_as_of":run.performance_as_of, "parser":PARSER,
        "bytes":metadata.bytes, "digest":metadata.content_digest, "metadata":metadata,
        "projection":projected, "complete":complete, "parsed":parsed,
    });
    let digest = census_domain::model::serialized_digest(&payload).map_err(|error| {
        crate::CrawlError::Invariant {
            detail: error.to_string(),
        }
    })?;
    let role = payload
        .get("role")
        .and_then(serde_json::Value::as_str)
        .map_or("refused", |value| value);
    Ok(EffectReceipt {
        key: format!("{}:{role}:{digest}", run.target.athleticlive_meet_id),
        payload,
    })
}
