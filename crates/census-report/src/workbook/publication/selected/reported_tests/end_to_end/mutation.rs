use super::TestResult;
use calamine::{open_workbook, Data, DataType, Reader, Xlsx};

pub(super) fn change_inventory(
    source: &std::path::Path,
    target: &std::path::Path,
    extra: bool,
) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = rust_xlsxwriter::Workbook::new();
    if extra {
        let range = reader.worksheet_range("Athletes")?;
        let sheet = writer.add_worksheet();
        sheet.set_name("Athletes")?;
        for (row, cells) in range.rows().enumerate() {
            for (column, cell) in cells.iter().enumerate() {
                let row = u32::try_from(row)?;
                let column = u16::try_from(column)?;
                match cell {
                    Data::Empty => {}
                    Data::Float(value) => {
                        sheet.write_number(row, column, *value)?;
                    }
                    Data::Int(value) => {
                        sheet.write_number(row, column, f64::from(i32::try_from(*value)?))?;
                    }
                    _ => {
                        sheet.write_string(row, column, cell.to_string())?;
                    }
                }
            }
        }
    }
    writer.add_worksheet().set_name("Unexpected")?;
    writer.save(target)?;
    Ok(())
}

pub(super) struct WorkbookMutation<'a> {
    pub(super) sheet: &'a str,
    pub(super) header: &'a str,
    pub(super) row_key: &'a str,
    pub(super) value: &'a str,
}

pub(super) fn corrupt_cell(
    source: &std::path::Path,
    target: &std::path::Path,
    mutation: WorkbookMutation<'_>,
) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = rust_xlsxwriter::Workbook::new();
    for name in reader.sheet_names() {
        let range = reader.worksheet_range(&name)?;
        let sheet = writer.add_worksheet();
        sheet.set_name(&name)?;
        let headers = range.rows().next().ok_or("worksheet header")?;
        let selected = if name == mutation.sheet {
            Some(
                headers
                    .iter()
                    .position(|cell| {
                        cell.get_string()
                            .is_some_and(|text| text == mutation.header)
                    })
                    .ok_or_else(|| format!("missing column {}", mutation.header))?,
            )
        } else {
            None
        };
        let key_column = headers
            .iter()
            .position(|cell| {
                cell.get_string()
                    .is_some_and(|text| text == "Performance ID")
            })
            .or_else(|| {
                headers
                    .iter()
                    .position(|cell| cell.get_string().is_some_and(|text| text == "Athlete ID"))
            });
        let mut target_row = None;
        if name == mutation.sheet {
            for (index, row) in range.rows().enumerate().skip(1) {
                if key_column
                    .and_then(|column| row.get(column))
                    .is_some_and(|cell| {
                        cell.get_string()
                            .is_some_and(|text| text == mutation.row_key)
                    })
                {
                    target_row = Some(index);
                    break;
                }
            }
        }
        for (row_index, row) in range.rows().enumerate() {
            for (column_index, value) in row.iter().enumerate() {
                let row_number = u32::try_from(row_index)?;
                let column_number = u16::try_from(column_index)?;
                if selected == Some(column_index) && target_row == Some(row_index) {
                    if mutation.value.is_empty() {
                        continue;
                    }
                    if matches!(mutation.header, "Mark Value" | "Calculated PR") {
                        let number = mutation.value.parse::<f64>()?;
                        sheet.write_number(row_number, column_number, number)?;
                        continue;
                    }
                    sheet.write_string(row_number, column_number, mutation.value)?;
                    continue;
                }
                match value {
                    Data::Empty => {}
                    Data::Float(number) => {
                        sheet.write_number(row_number, column_number, *number)?;
                    }
                    Data::Int(number) => {
                        sheet.write_number(
                            row_number,
                            column_number,
                            f64::from(i32::try_from(*number)?),
                        )?;
                    }
                    _ => {
                        sheet.write_string(row_number, column_number, value.to_string())?;
                    }
                }
            }
        }
        if target_row.is_none() && name == mutation.sheet {
            return Err(format!(
                "missing target row {} in {}",
                mutation.row_key, mutation.sheet
            )
            .into());
        }
    }
    writer.save(target)?;
    Ok(())
}
