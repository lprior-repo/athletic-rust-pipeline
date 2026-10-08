use anyhow::{ensure, Context, Result};
use census_domain::model::{
    AppliedAthleteIdentity, CanonicalAthlete, CanonicalCoach, CanonicalEvent, CanonicalMeet,
    CanonicalPerformance, CanonicalSchool, CanonicalTeam, CollectionSnapshot, CoverageRow,
    RetainedConflict, ReviewCase, ReviewVerdictRecord, SourceAccessCondition, SourceMeetRef,
    SourceObjectIdentity, SourceObservation,
};
use census_store::{Entity, StoreError, StoreSnapshot, Table};
use serde_json::{json, Value};

pub(in super::super) fn read(snapshot: &StoreSnapshot<'_>) -> Result<Value> {
    let tables = Table::ALL
        .into_iter()
        .map(|table| -> Result<Value> {
            let rows = table_rows(snapshot, table)?;
            Ok(json!({"table":table,"physical_rows":rows}))
        })
        .collect::<Result<Vec<_>>>()?;
    Ok(json!({
        "tables":tables,
        "tables_digest":snapshot.tables_digest(&Table::ALL)?,
        "evidence_generation":snapshot.evidence_generation(),
        "derived_generation":snapshot.derived_generation(),
        "merged_deduplication":false
    }))
}

pub(in super::super) fn reconcile(before: &Value, after: &Value) -> Result<()> {
    let original = before
        .get("tables")
        .and_then(Value::as_array)
        .context("physical tables absent")?;
    let current = after
        .get("tables")
        .and_then(Value::as_array)
        .context("physical tables absent")?;
    ensure!(
        original.len() == Table::ALL.len() && current.len() == Table::ALL.len(),
        "physical census oracle does not enumerate every table"
    );
    Table::ALL.into_iter().try_for_each(|table| -> Result<()> {
        let original = find(original, table)?;
        let current = find(current, table)?;
        ensure!(
            original == current,
            "physical source evidence or decision changed after recovery: {}",
            table.file()
        );
        Ok(())
    })?;
    ensure!(
        before == after,
        "exact physical corpus or generation changed at midnight"
    );
    Ok(())
}

fn find(rows: &[Value], table: Table) -> Result<&Value> {
    rows.iter()
        .find(|row| row.get("table") == Some(&json!(table)))
        .context("physical table absent")
}

fn table_rows(snapshot: &StoreSnapshot<'_>, table: Table) -> Result<Vec<Value>> {
    match table {
        Table::Schools => rows::<CanonicalSchool>(snapshot, table),
        Table::Teams => rows::<CanonicalTeam>(snapshot, table),
        Table::Coaches => rows::<CanonicalCoach>(snapshot, table),
        Table::Athletes => rows::<CanonicalAthlete>(snapshot, table),
        Table::Meets => rows::<CanonicalMeet>(snapshot, table),
        Table::Events => rows::<CanonicalEvent>(snapshot, table),
        Table::Performances => rows::<CanonicalPerformance>(snapshot, table),
        Table::SourceIdentities => rows::<SourceObjectIdentity>(snapshot, table),
        Table::Conflicts => rows::<RetainedConflict>(snapshot, table),
        Table::ReviewCases => rows::<ReviewCase>(snapshot, table),
        Table::Coverage => rows::<CoverageRow>(snapshot, table),
        Table::Snapshots => rows::<CollectionSnapshot>(snapshot, table),
        Table::SourceAccess => rows::<SourceAccessCondition>(snapshot, table),
        Table::IdentityVerdicts => rows::<ReviewVerdictRecord>(snapshot, table),
        Table::SourceMeets => rows::<SourceMeetRef>(snapshot, table),
        Table::SourceObservations => rows::<SourceObservation>(snapshot, table),
        Table::AthleteIdentityDecisions => rows::<AppliedAthleteIdentity>(snapshot, table),
    }
}

fn rows<T: Entity>(snapshot: &StoreSnapshot<'_>, table: Table) -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    let mut bytes = 0_usize;
    snapshot.for_each_observation::<T>(table, |row| {
        let encoded = serde_json::to_string(&row).map_err(|source| StoreError::Invariant {
            detail: source.to_string(),
        })?;
        bytes = bytes
            .checked_add(encoded.len())
            .ok_or(StoreError::CounterOverflow)?;
        if rows.len() >= 10_000 || bytes > 1024 * 1024 {
            return Err(StoreError::Invariant {
                detail: format!(
                    "native physical oracle budget exceeded for {}",
                    table.file()
                ),
            });
        }
        rows.try_reserve(1)
            .map_err(|source| StoreError::Invariant {
                detail: source.to_string(),
            })?;
        rows.push(encoded);
        Ok(())
    })?;
    rows.sort_unstable();
    rows.into_iter()
        .map(|encoded| serde_json::from_str(&encoded).context("decoding canonical physical row"))
        .collect()
}
