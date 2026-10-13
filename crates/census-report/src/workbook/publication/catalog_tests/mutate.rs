use super::fixture::TestResult;
use calamine::{open_workbook, Data, Reader, Xlsx};
use rust_xlsxwriter::Workbook;
use serde_json::Value;
use std::path::Path;

pub(super) fn assert_cell(path: &Path, sheet: &str, header: &str, expected: &str) -> TestResult {
    let mut book: Xlsx<_> = open_workbook(path)?;
    let range = book.worksheet_range(sheet)?;
    let mut rows = range.rows();
    let headings = rows.next().ok_or("sheet header")?;
    let column = headings
        .iter()
        .position(|cell| cell.to_string() == header)
        .ok_or("column")?;
    let row = rows.next().ok_or("athlete row")?;
    assert_eq!(row.get(column).ok_or("athlete cell")?.to_string(), expected);
    Ok(())
}

pub(super) fn write_lines(path: &Path, rows: &[Value]) -> TestResult {
    let mut text = String::new();
    for row in rows {
        text.push_str(&serde_json::to_string(row)?);
        text.push('\n');
    }
    std::fs::write(path, text)?;
    Ok(())
}

pub(super) fn workbook(original: &Path, target: &Path, mutation: &str) -> TestResult {
    let mut source: Xlsx<_> = open_workbook(original)?;
    let mut book = Workbook::new();
    for name in source.sheet_names() {
        let range = source.worksheet_range(&name)?;
        let mut rows: Vec<Vec<Data>> = range.rows().map(<[Data]>::to_vec).collect();
        mutate_rows(&name, mutation, &mut rows)?;
        let sheet = book.add_worksheet();
        sheet.set_name(&name)?;
        for (row, values) in rows.iter().enumerate() {
            for (column, value) in values.iter().enumerate() {
                let row = u32::try_from(row)?;
                let column = u16::try_from(column)?;
                match value {
                    Data::Empty => {}
                    Data::Int(number) => {
                        sheet.write_number(row, column, i32::try_from(*number)?)?;
                    }
                    Data::Float(number) => {
                        sheet.write_number(row, column, *number)?;
                    }
                    _ => {
                        sheet.write_string(row, column, value.to_string())?;
                    }
                }
            }
        }
    }
    book.save(target)?;
    Ok(())
}

fn mutate_rows(name: &str, mutation: &str, rows: &mut Vec<Vec<Data>>) -> TestResult {
    match (name, mutation) {
        ("Review", "Review") => rows.retain(|row| {
            row.first()
                .is_none_or(|cell| cell.to_string() != "Recruiting event unresolved")
        }),
        ("Relays", "Relays") => rows.push(rows.get(1).ok_or("positive team relay")?.clone()),
        ("Relays", "relay_missing_members") => {
            let row = rows.get_mut(1).ok_or("positive team relay")?;
            *row.get_mut(9).ok_or("published members column")? = Data::Empty;
        }
        ("Relays", "relay_swapped_fields") => {
            let row = rows.get_mut(1).ok_or("positive team relay")?;
            let team = row.get(1).ok_or("team column")?.clone();
            let event = row.get(2).ok_or("event column")?.clone();
            *row.get_mut(1).ok_or("team column")? = event;
            *row.get_mut(2).ok_or("event column")? = team;
        }
        _ => {}
    }
    Ok(())
}
