use serde_json::Value;

use super::super::super::http::Observation;
use super::super::super::scenario::Interrupted;

pub(super) fn matches(witness: &Interrupted) -> bool {
    let sequence = witness
        .armed
        .get("boundary_sequence")
        .and_then(Value::as_u64);
    witness.armed.get("key").and_then(Value::as_str) == Some(witness.key.as_str())
        && sequence.is_some()
        && witness.physical_at_witness.iter().any(|row| {
            row.get("sequence").and_then(Value::as_u64) == sequence
                && row.get("held").and_then(Value::as_bool) == Some(true)
                && row
                    .get("tls_handshake_record_observed")
                    .and_then(Value::as_bool)
                    == Some(true)
                && row.get("response_status").is_some_and(Value::is_null)
                && row.get("refused_at").is_some_and(Value::is_null)
                && row.get("forwarded").and_then(Value::as_bool) == Some(false)
                && ordered(
                    row,
                    &witness.progress,
                    &witness.cleanup,
                    &witness.release,
                    &witness.restart,
                )
        })
}

pub(super) fn ordered(
    held: &Value,
    inspection: &Observation,
    cleanup: &Value,
    release: &Value,
    restart: &Value,
) -> bool {
    let term = cleanup.get("endpoint_term_reap");
    let drained = cleanup.get("drain_certificate");
    let certified = term.is_some_and(|value| {
        value.get("success").and_then(Value::as_bool) == Some(true)
            && value.get("reaped").and_then(Value::as_bool) == Some(true)
    }) && drained
        .is_some_and(|value| value.get("balanced").and_then(Value::as_bool) == Some(true))
        && cleanup.get("sigkill_used").and_then(Value::as_bool) == Some(false)
        && restart
            .get("configuration_changed")
            .and_then(Value::as_bool)
            == Some(false)
        && restart
            .get("native_node_restarted")
            .and_then(Value::as_bool)
            == Some(false)
        && restart.get("old_owner_reaped") == Some(cleanup);
    let times = [
        held.get("accepted_at").and_then(Value::as_str),
        Some(inspection.sent_at.as_str()),
        Some(inspection.received_at.as_str()),
        term.and_then(|value| value.get("term_at"))
            .and_then(Value::as_str),
        cleanup.get("reaped_at").and_then(Value::as_str),
        release.get("at").and_then(Value::as_str),
        restart.get("at").and_then(Value::as_str),
    ]
    .into_iter()
    .map(|time| time.and_then(timestamp))
    .collect::<Option<Vec<_>>>();
    certified
        && times.is_some_and(|times| {
            times.windows(2).all(|pair| match pair {
                [before, after] => before <= after,
                _ => false,
            })
        })
}

fn timestamp(raw: &str) -> Option<u128> {
    let raw = raw.strip_suffix("Z-unix")?;
    let (seconds, nanos) = raw.split_once('.')?;
    if nanos.len() != 9
        || !seconds
            .bytes()
            .chain(nanos.bytes())
            .all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    seconds
        .parse::<u128>()
        .ok()?
        .checked_mul(1_000_000_000)?
        .checked_add(nanos.parse::<u128>().ok()?)
}
