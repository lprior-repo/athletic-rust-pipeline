use super::*;

fn displaced_copy(source: &std::path::Path, target: &std::path::Path) {
    let mut reader: Xlsx<_> = open_workbook(source).unwrap();
    let mut writer = Workbook::new();
    for name in reader.sheet_names() {
        let range = reader.worksheet_range(&name).unwrap();
        let owner = (name == "Athletes").then(|| column_of(&range, "Postal Owner ID"));
        let sheet = writer.add_worksheet();
        sheet.set_name(&name).unwrap();
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
                let r = u32::try_from(destination).unwrap();
                let c = u16::try_from(column).unwrap();
                if destination == 1 && owner == Some(column) {
                    sheet.write_string(r, c, "foreign-owner").unwrap();
                    continue;
                }
                match value {
                    Data::Empty => {}
                    Data::Float(value) => {
                        sheet.write_number(r, c, *value).unwrap();
                    }
                    Data::Int(value) => {
                        sheet.write_number(r, c, *value as f64).unwrap();
                    }
                    Data::Bool(value) => {
                        sheet.write_boolean(r, c, *value).unwrap();
                    }
                    _ => {
                        sheet.write_string(r, c, value.to_string()).unwrap();
                    }
                }
            }
        }
    }
    writer.save(target).unwrap();
}

#[test]
fn displaced_athlete_rows_cannot_hide_forged_postal_ownership_from_full_verification() {
    let directory = tempfile::tempdir().unwrap();
    let store = Store::open(directory.path().join("store")).unwrap();
    let (school, first) = seed(&store);
    let mut second = CanonicalAthlete::new(
        &school.id,
        "Another Captured Runner",
        GradYear::CO2027,
        Gender::Boys,
        SourceIdentity::new(SourceNamespace::MilesplitAthlete, "14399170"),
    );
    second.evidence = first.evidence.clone();
    store.append(Table::Athletes, &second).unwrap();
    let (source, dataset, options) = published(&store);
    let target = directory.path().join("displaced-forgery.xlsx");
    displaced_copy(&source, &target);
    let mut reader = open_workbook(&target).unwrap();
    let rows = sheet(&mut reader, "Athletes");
    let mut ids = [text(&rows, 1, 0), text(&rows, 2, 0)];
    ids.sort();
    let mut expected = [first.id.to_string(), second.id.to_string()];
    expected.sort();
    assert_eq!(ids, expected);
    assert_eq!(
        text(&rows, 1, column_of(&rows, "Postal Owner ID")),
        "foreign-owner"
    );
    assert!(verify_frozen(&target, &dataset, &options).is_err());
}
