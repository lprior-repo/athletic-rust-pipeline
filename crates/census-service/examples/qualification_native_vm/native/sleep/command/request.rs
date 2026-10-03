use super::super::shape::{self, field, fields, integer, text, variant};
use anyhow::{ensure, Context, Result};
use serde_json::Value;

pub(super) fn parse(value: &Value) -> Result<()> {
    fields(
        value,
        &[
            "invocation_id",
            "invocation_target",
            "span_context",
            "parameter",
            "headers",
            "idempotency_key",
            "completion_retention_duration",
        ],
        &["journal_retention_duration", "limit_key"],
    )?;
    text(field(value, "invocation_id")?)?;
    target(field(value, "invocation_target")?)?;
    span(field(value, "span_context")?)?;
    shape::bytes(field(value, "parameter")?)?;
    shape::headers(field(value, "headers")?)?;
    nullable_text(field(value, "idempotency_key")?)?;
    duration(field(value, "completion_retention_duration")?)?;
    if let Some(retention) = value.get("journal_retention_duration") {
        duration(retention)?;
    }
    if let Some(limit) = value.get("limit_key") {
        text(limit)?;
    }
    Ok(())
}

pub(super) fn attachment(value: &Value) -> Result<()> {
    let (kind, payload) = variant(value, "attachment target")?;
    match kind {
        "InvocationId" => text(payload),
        "Workflow" => {
            fields(
                payload,
                &["service_name", "key", "partition_key", "scope"],
                &[],
            )?;
            text(field(payload, "service_name")?)?;
            text(field(payload, "key")?)?;
            integer(
                field(payload, "partition_key")?,
                u64::MAX,
                "partition key malformed",
            )?;
            nullable_text(field(payload, "scope")?)
        }
        "IdempotentRequest" => idempotent(payload),
        _ => anyhow::bail!("unknown attachment target variant"),
    }
}

fn idempotent(value: &Value) -> Result<()> {
    fields(
        value,
        &[
            "service_name",
            "service_key",
            "service_handler",
            "idempotency_key",
            "scope",
            "partition_key",
        ],
        &[],
    )?;
    ["service_name", "service_handler", "idempotency_key"]
        .iter()
        .try_for_each(|key| text(field(value, key)?))?;
    nullable_text(field(value, "service_key")?)?;
    nullable_text(field(value, "scope")?)?;
    integer(
        field(value, "partition_key")?,
        u64::MAX,
        "partition key malformed",
    )?;
    Ok(())
}

fn nullable_text(value: &Value) -> Result<()> {
    if value.is_null() {
        return Ok(());
    }
    text(value)
}

fn duration(value: &Value) -> Result<()> {
    fields(value, &["secs", "nanos"], &[])?;
    integer(
        field(value, "secs")?,
        u64::MAX,
        "duration seconds malformed",
    )?;
    integer(
        field(value, "nanos")?,
        999_999_999,
        "duration nanos malformed",
    )?;
    Ok(())
}

fn target(value: &Value) -> Result<()> {
    let (kind, payload) = variant(value, "invocation target")?;
    let handler_types: &[&str] = match kind {
        "Service" => &[],
        "VirtualObject" => &["Exclusive", "Shared"],
        "Workflow" => &["Workflow", "Shared"],
        _ => anyhow::bail!("unknown invocation target variant"),
    };
    if kind == "Service" {
        fields(payload, &["name", "handler", "scope"], &[])?;
    } else {
        fields(
            payload,
            &["name", "key", "handler", "handler_ty", "scope"],
            &[],
        )?;
        text(field(payload, "key")?)?;
        let ty = field(payload, "handler_ty")?
            .as_str()
            .context("handler type malformed")?;
        ensure!(handler_types.contains(&ty), "unknown handler type");
    }
    text(field(payload, "name")?)?;
    text(field(payload, "handler")?)?;
    nullable_text(field(payload, "scope")?)
}

fn span(value: &Value) -> Result<()> {
    fields(value, &["span_context", "cause"], &[])?;
    span_context(field(value, "span_context")?)?;
    let cause = field(value, "cause")?;
    if cause.is_null() {
        return Ok(());
    }
    let (kind, payload) = variant(cause, "span cause")?;
    match kind {
        "Parent" => fixed_bytes(payload, 8),
        "Linked" => {
            let pair = payload.as_array().context("linked span malformed")?;
            ensure!(pair.len() == 2, "linked span malformed");
            fixed_bytes(pair.first().context("linked trace absent")?, 16)?;
            fixed_bytes(pair.get(1).context("linked span absent")?, 8)
        }
        _ => anyhow::bail!("unknown span cause variant"),
    }
}

fn span_context(context: &Value) -> Result<()> {
    fields(
        context,
        &[
            "trace_id",
            "span_id",
            "trace_flags",
            "is_remote",
            "trace_state",
        ],
        &[],
    )?;
    fixed_bytes(field(context, "trace_id")?, 16)?;
    fixed_bytes(field(context, "span_id")?, 8)?;
    integer(
        field(context, "trace_flags")?,
        u64::from(u8::MAX),
        "trace flags malformed",
    )?;
    ensure!(
        field(context, "is_remote")?.is_boolean(),
        "trace remote flag malformed"
    );
    text(field(context, "trace_state")?)
}

fn fixed_bytes(value: &Value, size: usize) -> Result<()> {
    shape::bytes(value)?;
    ensure!(
        value.as_array().is_some_and(|array| array.len() == size),
        "trace ID width mismatch"
    );
    Ok(())
}
