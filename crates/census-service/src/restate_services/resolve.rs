use census_report::report::Scope;
use census_store::Table;
use restate_sdk::prelude::TerminalError;

pub(crate) fn resolve_table(name: &str) -> Result<Table, TerminalError> {
    Table::from_wire(name).ok_or_else(|| {
        TerminalError::new(format!(
            "unknown table {name}; expected one of {:?}",
            Table::ALL.map(Table::file)
        ))
    })
}

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
    match grad_year.map(|year| format!("co{year}")) {
        Some(value) => value,
        None => "all".to_string(),
    }
}
