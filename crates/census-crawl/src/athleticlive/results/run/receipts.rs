use super::super::{DocumentEntities, EffectReceipt, EFFECT_PHASE, PARSER};
use crate::net::cache::CacheMeta;
use crate::{AdapterContext, CrawlResult};
use serde_json::json;

const HISTORICAL_EFFECT_PHASE: &str = "athleticlive_results_effect_v2";
const HISTORICAL_PARSER: &str = "athleticlive_results_v4";

pub(super) fn require_owner(
    ctx: &AdapterContext<'_>,
    path: &str,
    meta: &CacheMeta,
    run: &super::Run,
) -> CrawlResult<()> {
    for (phase, parser) in [
        (HISTORICAL_EFFECT_PHASE, HISTORICAL_PARSER),
        (EFFECT_PHASE, PARSER),
    ] {
        let keys = ctx.store.journal_keys(phase)?;
        if keys.len() > super::MAX_CAPTURE_RECORDS {
            return Err(super::resource("LIVE receipt index", keys.len()));
        }
        let meet = run.target.athleticlive_meet_id.to_string();
        for key in keys {
            let payload = ctx
                .store
                .journal_payload(phase, &key)?
                .ok_or_else(|| super::schema(path, "projection receipt has no payload"))?;
            require_shape(path, &key, &payload, parser)?;
            if (payload.get("path").and_then(serde_json::Value::as_str) == Some(path)
                || payload.get("digest").and_then(serde_json::Value::as_str)
                    == Some(meta.content_digest.as_str()))
                && (payload.get("meet").and_then(serde_json::Value::as_str) != Some(meet.as_str())
                    || payload.get("provider").and_then(serde_json::Value::as_str)
                        != Some(run.target.tenant.as_str()))
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

fn receipt_owner(payload: &serde_json::Value) -> Option<&str> {
    payload
        .get("meet")
        .and_then(serde_json::Value::as_str)
        .filter(|value| !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()))
}

fn receipt_projection(payload: &serde_json::Value) -> Option<&str> {
    payload
        .get("projection")
        .and_then(serde_json::Value::as_str)
        .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
}

fn receipt_fields_present(
    meet: Option<&str>,
    projection: Option<&str>,
    payload: &serde_json::Value,
    parser: &str,
) -> bool {
    meet.is_some()
        && projection.is_some()
        && capture_fields_match(payload)
        && payload.get("parser").and_then(serde_json::Value::as_str) == Some(parser)
        && payload
            .get("complete")
            .and_then(serde_json::Value::as_bool)
            .is_some()
        && payload
            .pointer("/metadata/url")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && payload
            .get("path")
            .and_then(serde_json::Value::as_str)
            .is_some()
        && payload
            .get("provider")
            .and_then(serde_json::Value::as_str)
            .is_some()
}

fn capture_fields_match(payload: &serde_json::Value) -> bool {
    let Some(digest) = payload
        .get("digest")
        .and_then(serde_json::Value::as_str)
        .filter(|digest| {
            digest.len() == 64
                && digest
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
        })
    else {
        return false;
    };
    let Some(bytes) = payload.get("bytes").and_then(serde_json::Value::as_u64) else {
        return false;
    };
    payload
        .pointer("/metadata/content_digest")
        .and_then(serde_json::Value::as_str)
        == Some(digest)
        && payload
            .pointer("/metadata/bytes")
            .and_then(serde_json::Value::as_u64)
            == Some(bytes)
}

fn receipt_identity(payload: &serde_json::Value) -> serde_json::Value {
    json!({
        "role":payload.get("role"),
        "meet":payload.get("meet"),
        "provider":payload.get("provider"),
        "meet_name":payload.get("meet_name"),
        "meet_date":payload.get("meet_date"),
        "state":payload.get("state"),
        "observed_on":payload.get("observed_on"),
        "school_year":payload.get("school_year"),
        "performance_as_of":payload.get("performance_as_of"),
        "parser":payload.get("parser"),
        "bytes":payload.get("bytes"),
        "digest":payload.get("digest"),
        "projection":payload.get("projection"),
        "complete":payload.get("complete"),
        "parsed":payload.get("parsed"),
    })
}

fn receipt_digest(payload: &serde_json::Value) -> CrawlResult<String> {
    census_domain::model::serialized_digest(&receipt_identity(payload)).map_err(|error| {
        crate::CrawlError::Invariant {
            detail: error.to_string(),
        }
    })
}

fn require_shape(
    path: &str,
    key: &str,
    payload: &serde_json::Value,
    parser: &str,
) -> CrawlResult<()> {
    let meet = receipt_owner(payload);
    let projection = receipt_projection(payload);
    if !receipt_fields_present(meet, projection, payload, parser) {
        return Err(super::schema(
            path,
            "ambiguous or incompatible LIVE projection receipt shape",
        ));
    }
    let digest = receipt_digest(payload)?;
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
    let meet = run.target.athleticlive_meet_id.to_string();
    let identity = json!({
        "role":parsed.as_ref().and_then(|value| value.get("role")),
        "meet":meet, "provider":run.target.tenant,
        "meet_name":run.target.name, "meet_date":run.target.date, "state":run.target.state,
        "observed_on":metadata.fetched_at, "school_year":run.school_year,
        "performance_as_of":run.performance_as_of, "parser":PARSER,
        "bytes":metadata.bytes, "digest":metadata.content_digest,
        "projection":projected, "complete":complete, "parsed":parsed,
    });
    let identity_digest = census_domain::model::serialized_digest(&identity).map_err(|error| {
        crate::CrawlError::Invariant {
            detail: error.to_string(),
        }
    })?;
    let role = identity
        .get("role")
        .and_then(serde_json::Value::as_str)
        .map_or("refused", |value| value);
    let mut payload = identity.clone();
    let fields = payload
        .as_object_mut()
        .ok_or_else(|| super::schema(path, "receipt identity is not an object"))?;
    fields.insert("path".to_string(), json!(path));
    fields.insert("metadata".to_string(), json!(metadata));
    Ok(EffectReceipt {
        key: format!(
            "{}:{role}:{identity_digest}",
            run.target.athleticlive_meet_id
        ),
        payload,
    })
}
