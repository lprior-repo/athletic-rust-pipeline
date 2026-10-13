use super::*;
use rust_xlsxwriter::Workbook;

pub(super) fn assert_cells(
    path: &Path,
    athletes: usize,
    schools: usize,
    meets: usize,
    performances: usize,
) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(path)?;
    for (name, expected) in [
        ("Athletes", athletes),
        ("Schools", schools),
        ("Meets", meets),
        ("Performances_001", performances),
    ] {
        let sheet = reader.worksheet_range(name)?;
        check!(eq; sheet.height().checked_sub(1).ok_or("missing header")?, expected);
    }
    let coverage = reader.worksheet_range("Coverage")?;
    check!(coverage.rows().any(|row| row
        .first()
        .is_some_and(|cell| cell.to_string() == "Retained global recruiting event disposition")));
    let global_events = coverage
        .rows()
        .find(|row| {
            row.first()
                .is_some_and(|cell| cell.to_string() == "included")
        })
        .and_then(|row| row.get(1))
        .ok_or("global retained catalog count")?
        .to_string()
        .parse::<u64>()?;
    check!(eq; global_events, 3);
    let published: Vec<_> = coverage
        .rows()
        .skip(1)
        .take_while(|row| row.first().is_some_and(|cell| cell != &Data::Empty))
        .collect();
    let rows = published
        .iter()
        .map(|row| row.first().ok_or("coverage code").map(ToString::to_string))
        .collect::<Result<Vec<_>, _>>()?;
    check!(rows.contains(&"WI".to_string()));
    check!(rows.contains(&JurisdictionBucket::UNPLACED_CODE.to_string()));
    if schools == 2 {
        check!(eq; rows.len(), 2);
        check!(!rows.contains(&"MN".to_string()));
    }
    let columns = coverage.rows().next().ok_or("coverage header")?;
    for (name, expected) in [
        ("Athletes", athletes),
        ("Schools", schools),
        ("Meets", meets),
        ("Performances", performances),
    ] {
        let column = columns
            .iter()
            .position(|cell| cell.to_string() == name)
            .ok_or("coverage counter")?;
        let total = published
            .iter()
            .try_fold(0_u64, |total, row| -> TestResult<u64> {
                let value = row
                    .get(column)
                    .ok_or("coverage value")?
                    .to_string()
                    .parse::<u64>()?;
                total
                    .checked_add(value)
                    .ok_or_else(|| "coverage sum overflow".into())
            })?;
        check!(eq; usize::try_from(total)?, expected);
    }
    Ok(())
}

pub(super) fn tamper_coverage(source: &Path, target: &Path) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = Workbook::new();
    let mut mutated = false;
    for name in reader.sheet_names() {
        let range = reader.worksheet_range(&name)?;
        let sheet = writer.add_worksheet();
        sheet.set_name(&name)?;
        for (row, cells) in range.rows().enumerate() {
            for (column, value) in cells.iter().enumerate() {
                let r = u32::try_from(row)?;
                let c = u16::try_from(column)?;
                if name == "Coverage" && row == 1 && column == 3 {
                    sheet.write_number(r, c, 999.0)?;
                    mutated = true;
                } else {
                    write_cell(sheet, r, c, value)?;
                }
            }
        }
    }
    check!(
        mutated,
        "a scoped coverage athlete count was actually changed"
    );
    writer.save(target)?;
    Ok(())
}

fn write_cell(
    sheet: &mut rust_xlsxwriter::Worksheet,
    row: u32,
    column: u16,
    value: &Data,
) -> TestResult {
    match value {
        Data::Empty => {}
        Data::Float(value) => {
            sheet.write_number(row, column, *value)?;
        }
        Data::Int(value) => {
            sheet.write_number(row, column, value.to_string().parse::<f64>()?)?;
        }
        _ => {
            sheet.write_string(row, column, value.to_string())?;
        }
    }
    Ok(())
}
