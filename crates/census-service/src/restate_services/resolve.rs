//! Wire names to store tables, scope selectors to the report scope, graduation years to cohort
//! labels.
//!
//! Every unknown name here is terminal: a typo is a wrong request, and a table nobody scans would
//! hide that mistake from every surface that reads the store afterwards.

use census_report::report::Scope;
use census_store::Table;
use restate_sdk::prelude::TerminalError;

/// Resolve a requested table name. Unknown names are terminal: a retry cannot fix a typo, and
/// silently creating a table nobody scans would hide the mistake.
pub(crate) fn resolve_table(name: &str) -> Result<Table, TerminalError> {
    Table::from_wire(name).ok_or_else(|| {
        TerminalError::new(format!(
            "unknown table {name}; expected one of {:?}",
            Table::ALL.map(Table::file)
        ))
    })
}

/// Resolve a requested table list; an empty list means every table, in [`Table::ALL`] order.
pub(crate) fn resolve_tables(names: &[String]) -> Result<Vec<Table>, TerminalError> {
    let mut tables = Vec::with_capacity(names.len());
    for name in names {
        let table = resolve_table(name)?;
        if !tables.contains(&table) {
            tables.push(table);
        }
    }
    if tables.is_empty() {
        return Ok(Table::ALL.to_vec());
    }
    Ok(tables)
}

/// Resolve the scope selector used by the report, bests, and workbook surfaces.
pub(crate) fn resolve_scope(name: Option<&str>) -> Result<Scope, TerminalError> {
    match name {
        None | Some("all_sources") => Ok(Scope::AllSources),
        Some("core") => Ok(Scope::Core),
        Some(other) => Err(TerminalError::new(format!(
            "unknown scope {other}; expected core or all_sources"
        ))),
    }
}

pub(crate) fn cohort_label(grad_year: Option<i16>) -> String {
    grad_year
        .map(|year| format!("co{year}"))
        .unwrap_or_else(|| "all".to_string())
}
