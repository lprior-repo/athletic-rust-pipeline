use super::*;

fn displaced_copy(source: &std::path::Path, target: &std::path::Path) -> TestResult {
    let mut reader: Xlsx<_> = open_workbook(source)?;
    let mut writer = Workbook::new();
    for name in reader.sheet_names() {
        let range = reader.worksheet_range(&name)?;
        let owner = if name == "Athletes" {
            Some(column_of(&range, "Postal Owner ID")?)
        } else {
            None
        };
        let sheet = writer.add_worksheet();
        sheet.set_name(&name)?;
        for (row, cells) in range.rows().enumerate() {
            let destination = if owner.is_some() {
                match row {
                    1 => 2,
                    2 => 1,
                    other => other,
                }
            } else {
                row
            };
            for (column, value) in cells.iter().enumerate() {
                let r = u32::try_from(destination)?;
                let c = u16::try_from(column)?;
                if destination == 1 && owner == Some(column) {
                    sheet.write_string(r, c, "foreign-owner")?;
                    continue;
                }
                match value {
                    Data::Empty => {}
                    Data::Float(value) => {
                        sheet.write_number(r, c, *value)?;
                    }
                    Data::Int(value) => {
                        sheet.write_number(r, c, *value as f64)?;
                    }
                    Data::Bool(value) => {
                        sheet.write_boolean(r, c, *value)?;
                    }
                    _ => {
                        sheet.write_string(r, c, value.to_string())?;
                    }
                }
            }
        }
    }
    writer.save(target)?;
    Ok(())
}

#[test]
fn displaced_athlete_rows_cannot_hide_forged_postal_ownership_from_full_verification() -> TestResult
{
    let directory = tempfile::tempdir()?;
    let store = Store::open(directory.path().join("store"))?;
    let (school, first) = seed(&store)?;
    let mut second = CanonicalAthlete::new(
        &school.id,
        "Another Captured Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399170"),
    );
    second.evidence = first.evidence.clone();
    publish_fixture_cohort(&mut second, "nchsaa", "displaced-postal-row", CAPTURE_DAY);
    store.append(Table::Athletes, &second)?;
    let (source, dataset, options) = published(&store)?;
    let target = directory.path().join("displaced-forgery.xlsx");
    displaced_copy(&source, &target)?;
    let mut reader = open_workbook(&target)?;
    let rows = sheet(&mut reader, "Athletes")?;
    let mut ids = [text(&rows, 1, 0), text(&rows, 2, 0)];
    ids.sort();
    let mut expected = [first.id.to_string(), second.id.to_string()];
    expected.sort();
    check!(eq; ids, expected);
    check!(eq; text(&rows, 1, column_of(&rows, "Postal Owner ID")?),
    "foreign-owner");
    check!(verify_frozen(&target, &dataset, &options).is_err());
    Ok(())
}
