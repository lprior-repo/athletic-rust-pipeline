use anyhow::{ensure, Result};
use serde_json::json;
use std::path::Path;
use std::sync::atomic::Ordering;
use std::time::Duration;

use super::super::artifacts::{append, now};
use super::Control;

pub(super) fn wait(control: &Control, ledger: &Path, sequence: usize) -> Result<()> {
    let events = ledger.with_file_name("transport-events.jsonl");
    append(
        &events,
        &json!({"event":"held_transport_entered", "sequence":sequence,
        "at":now()?, "hold_budget_ms":90_000, "response_status":null, "forwarded":false}),
    )?;
    let released = (0..9_000).any(|_| {
        if control.released.load(Ordering::Acquire) || control.stopped.load(Ordering::Acquire) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
        false
    });
    append(
        &events,
        &json!({"event":"held_transport_ended", "sequence":sequence,
        "at":now()?, "released_or_stopped":released, "deadline_exhausted":!released}),
    )?;
    ensure!(
        released,
        "owned held transport exceeded its fixed 90-second boundary"
    );
    Ok(())
}
