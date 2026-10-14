use super::*;
use crate::artifacts::Semantics;
use std::collections::BTreeMap;

fn write_rows(path: &Path, rows: &[Vec<String>]) -> Result<()> {
    let mut writer = csv::Writer::from_path(path)?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}

fn semantics(name: &str, rows: Vec<Vec<String>>) -> Semantics {
    Semantics {
        workbook: BTreeMap::new(),
        sidecars: BTreeMap::new(),
        csv: BTreeMap::from([(name.to_owned(), rows)]),
    }
}

#[test]
fn rebuilt_csv_families_compare_every_fact_except_their_verified_local_generation() -> Result<()> {
    let directory = tempfile::tempdir()?;
    for name in FAMILIES {
        let path = directory.path().join(name);
        let mut rows = vec![
            vec![
                "entity_id".into(),
                "input_generation".into(),
                "retained_fact".into(),
            ],
            vec![
                "source-owned-id".into(),
                "first".into(),
                "{\"input_generation\":\"source-fact\"}".into(),
            ],
        ];
        write_rows(&path, &rows)?;
        let first = semantics(name, read(&path, name, "first")?);
        *rows
            .get_mut(1)
            .and_then(|row| row.get_mut(1))
            .context("binding cell")? = "second".into();
        write_rows(&path, &rows)?;
        first.compare(&semantics(name, read(&path, name, "second")?))?;
        for index in [0, 2] {
            let mut changed = rows.clone();
            *changed
                .get_mut(1)
                .and_then(|row| row.get_mut(index))
                .context("fact cell")? = "changed fact".into();
            write_rows(&path, &changed)?;
            let error = first
                .compare(&semantics(name, read(&path, name, "second")?))
                .err()
                .context("fact mutation escaped comparison")?;
            check!(eq; error.to_string(), "rebuilt publication changed artifact names, row counts or CSV facts");
        }
        rows.push(rows.get(1).context("source row")?.clone());
        write_rows(&path, &rows)?;
        check!(
            first
                .compare(&semantics(name, read(&path, name, "second")?))
                .is_err(),
            "duplicate {name} row escaped comparison"
        );
    }
    Ok(())
}

#[test]
fn incorrect_csv_generation_cannot_be_hidden_by_rebuild_normalization() -> Result<()> {
    let directory = tempfile::tempdir()?;
    for name in FAMILIES {
        let path = directory.path().join(name);
        write_rows(
            &path,
            &[
                vec!["entity_id".into(), "input_generation".into()],
                vec!["id".into(), "foreign".into()],
            ],
        )?;
        let error = read(&path, name, "expected")
            .err()
            .context("foreign binding accepted")?;
        check!(eq; error.to_string(), format!("{name} row 1: input generation \"foreign\" differs from manifest \"expected\""));
    }
    Ok(())
}

#[test]
fn legacy_csv_generation_named_facts_remain_exact() -> Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("recruiting.csv");
    write_rows(
        &path,
        &[
            vec!["input_generation".into()],
            vec!["source-owned-fact".into()],
        ],
    )?;
    check!(eq; read(&path, "recruiting.csv", "manifest")?, vec![vec!["input_generation".to_owned()], vec!["source-owned-fact".to_owned()]]);
    Ok(())
}

#[test]
fn missing_normalized_family_refuses_even_when_both_rebuilt_inventories_omit_it() -> Result<()> {
    let inventory: Map<String, Value> = FAMILIES
        .iter()
        .map(|name| ((*name).into(), Value::Null))
        .collect();
    require_families(&inventory)?;
    for name in FAMILIES {
        let mut missing = inventory.clone();
        missing.remove(name).context("family entry")?;
        let error = require_families(&missing)
            .err()
            .context("missing family accepted")?;
        check!(eq; error.to_string(), format!("missing normalized family {name}"));
    }
    Ok(())
}
