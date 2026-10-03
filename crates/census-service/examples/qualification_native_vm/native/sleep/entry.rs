use super::{command, notification, shape};
use anyhow::{ensure, Context, Result};
use serde_json::Value;

#[derive(Clone, Copy)]
pub(super) enum Entry {
    Sleep { completion: u32, wake: u64 },
    Completion(u32),
    Other,
}

pub(super) fn journal(journal: &[Value], id: &str) -> Result<Vec<Entry>> {
    ensure!(
        journal.len() <= 256,
        "journal witness size outside bounded scope"
    );
    let mut entries = Vec::new();
    entries.try_reserve_exact(journal.len())?;
    journal
        .iter()
        .enumerate()
        .try_fold(entries, |mut entries, (index, row)| {
            ensure!(
                row.get("id").and_then(Value::as_str) == Some(id),
                "foreign journal invocation"
            );
            ensure!(
                row.get("index").and_then(Value::as_u64) == Some(u64::try_from(index)?),
                "incomplete journal indexes"
            );
            ensure!(
                row.get("version").and_then(Value::as_u64) == Some(2),
                "unfinished-Sleep witness requires journal v2"
            );
            let entry = shape::decode(row.get("entry_json").context("journal JSON absent")?)?;
            let row_type = row
                .get("entry_type")
                .and_then(Value::as_str)
                .context("journal type absent")?;
            entries.push(parse(&entry, row_type)?);
            Ok(entries)
        })
}

fn parse(value: &Value, row_type: &str) -> Result<Entry> {
    let (kind, payload) = shape::variant(value, "entry")?;
    match kind {
        "Command" => command::parse(payload, row_type),
        "Notification" => notification::parse(payload, row_type),
        _ => anyhow::bail!("unknown journal entry envelope"),
    }
}
