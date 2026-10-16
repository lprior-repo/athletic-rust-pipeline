use super::*;

#[test]
fn audit_readback_refuses_missing_duplicate_foreign_and_malformed_records() -> TestResult {
    let fixture = fixture::fixture()?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join(FILE);
    for attack in [
        "absent",
        "missing",
        "duplicate",
        "foreign",
        "extra_column",
        "truncated",
    ] {
        write(directory.path(), &fixture.dataset)?;
        if attack == "absent" {
            std::fs::remove_file(&path)?;
        } else if attack == "truncated" {
            std::fs::write(&path, b"retained_candidate_id,\"unterminated")?;
        } else {
            let (header, mut records) = records(&path)?;
            let first = records.first().ok_or("first audit row")?.clone();
            match attack {
                "missing" => {
                    records.remove(0);
                }
                "duplicate" => {
                    records.pop();
                    records.push(first);
                }
                "foreign" => {
                    *records
                        .first_mut()
                        .and_then(|row| row.first_mut())
                        .ok_or("first audit id")? = "foreign-synthetic-athlete".into();
                }
                "extra_column" => {
                    records
                        .first_mut()
                        .ok_or("first audit row")?
                        .push("extra".into());
                }
                _ => return Err("unknown synthetic attack".into()),
            }
            rewrite(&path, &header, &records)?;
        }
        check!(
            verify(directory.path(), &fixture.dataset).is_err(),
            "{attack}"
        );
    }
    Ok(())
}

#[test]
fn audit_readback_refuses_changes_to_every_bound_column() -> TestResult {
    let fixture = fixture::fixture()?;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join(FILE);
    for column in 0..HEADER.len() {
        write(directory.path(), &fixture.dataset)?;
        let (header, mut records) = records(&path)?;
        *records
            .first_mut()
            .and_then(|row| row.get_mut(column))
            .ok_or("audit cell")? = "tampered-synthetic-value".into();
        rewrite(&path, &header, &records)?;
        check!(
            verify(directory.path(), &fixture.dataset).is_err(),
            "column {column}"
        );
    }
    Ok(())
}

fn records(path: &Path) -> TestResult<(csv::StringRecord, Vec<Vec<String>>)> {
    let mut reader = csv::Reader::from_path(path)?;
    let header = reader.headers()?.clone();
    let rows = reader
        .records()
        .map(|row| row.map(|row| row.iter().map(str::to_owned).collect()))
        .collect::<Result<Vec<_>, _>>()?;
    Ok((header, rows))
}

fn rewrite(path: &Path, header: &csv::StringRecord, records: &[Vec<String>]) -> TestResult {
    let mut writer = csv::WriterBuilder::new().flexible(true).from_path(path)?;
    writer.write_record(header)?;
    for record in records {
        writer.write_record(record)?;
    }
    writer.flush()?;
    Ok(())
}
