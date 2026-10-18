use census_store::{ReadBudget, Store, StoreError, StoreResult, Table};

use super::PacingState;

impl PacingState {
    pub fn restore(store: &Store, budget: ReadBudget) -> StoreResult<Self> {
        let mut restored = Self::new();
        let blocks = restored.blocks.get_mut();
        store.snapshot().for_each_merged_bounded(
            Table::SourceAccess,
            budget,
            |row: census_domain::model::SourceAccessCondition, _| {
                validate(&row)?;
                blocks.try_reserve(1).map_err(|_| StoreError::Invariant {
                    detail: "source access restoration allocation failed".into(),
                })?;
                let mut id = String::new();
                id.try_reserve_exact(row.id.len())
                    .map_err(|_| StoreError::Invariant {
                        detail: "source access restoration identity allocation failed".into(),
                    })?;
                id.push_str(&row.id);
                blocks.insert(id, row);
                Ok(())
            },
        )?;
        Ok(restored)
    }
}

fn validate(row: &census_domain::model::SourceAccessCondition) -> StoreResult<()> {
    let identity = row
        .id
        .strip_prefix(row.kind.slug())
        .and_then(|id| id.strip_prefix(':'));
    let host_valid = !row.host.is_empty()
        && row.host == row.host.trim()
        && url::Host::parse(&row.host).is_ok_and(|host| host.to_string() == row.host)
        && !row.host.bytes().any(|byte| {
            byte.is_ascii_uppercase() || byte.is_ascii_whitespace() || byte.is_ascii_control()
        });
    if identity != Some(row.host.as_str())
        || !host_valid
        || !timestamp(&row.observed_at)
        || row
            .cooldown_until
            .as_deref()
            .is_some_and(|until| !timestamp(until))
    {
        return Err(StoreError::Invariant {
            detail: "retained source access identity, host or timestamp is malformed".into(),
        });
    }
    Ok(())
}

fn timestamp(value: &str) -> bool {
    value
        .parse::<chrono::DateTime<chrono::Utc>>()
        .is_ok_and(|time| time.to_rfc3339_opts(chrono::SecondsFormat::Secs, true) == value)
}

#[cfg(test)]
mod tests;
