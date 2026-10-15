use super::*;

#[test]
fn participation_readback_rejects_each_tampered_cell() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, Some(0))?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = read(directory.path())?;
    for (column, replacement) in [
        (0, "foreign-athlete"),
        (1, "foreign-performance"),
        (2, "foreign-event"),
        (3, "eligible"),
        (4, "[\"invented_reason\"]"),
        (5, "[\"invented_diagnostic\"]"),
        (6, "[\"invented_proof\"]"),
        (7, "foreign-generation"),
    ] {
        let mut rows = original.clone();
        *rows
            .get_mut(1)
            .and_then(|row| row.get_mut(column))
            .ok_or("cell")? = replacement.into();
        save(directory.path(), &rows)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    }
    Ok(())
}

#[test]
fn participation_readback_rejects_duplicate_extra_and_missing_records() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = read(directory.path())?;
    let first = original.get(1).ok_or("first record")?.clone();
    let mut duplicate = original.clone();
    *duplicate.get_mut(2).ok_or("second record")? = first.clone();
    let mut extra = original.clone();
    extra.push(first);
    let mut missing = original.clone();
    missing.pop().ok_or("last record")?;
    let mut missing_header = original;
    missing_header.remove(0);
    for rows in [duplicate, extra, missing, missing_header] {
        save(directory.path(), &rows)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    }
    Ok(())
}

#[test]
fn participation_readback_rejects_wrong_header_and_record_shapes() -> TestResult {
    let fixture = Fixture::new()?;
    let view = fixture.view()?;
    let outcome = fixture.outcome(&view, None)?;
    let directory = tempfile::tempdir()?;
    write(directory.path(), &fixture.dataset, &view, &outcome)?;
    let original = read(directory.path())?;
    let mut wrong_header = original.clone();
    *wrong_header
        .first_mut()
        .and_then(|row| row.first_mut())
        .ok_or("header")? = "subject".into();
    let mut previous_header = original.clone();
    *previous_header
        .first_mut()
        .and_then(|row| row.first_mut())
        .ok_or("previous header")? = "athlete_id".into();
    let mut short = original.clone();
    short
        .get_mut(1)
        .ok_or("record")?
        .pop()
        .ok_or("last field")?;
    let mut long = original;
    long.get_mut(1).ok_or("record")?.push("unexpected".into());
    for rows in [wrong_header, previous_header, short, long] {
        save(directory.path(), &rows)?;
        check!(verify(directory.path(), &fixture.dataset, &view, &outcome).is_err());
    }
    Ok(())
}

pub(super) fn read(directory: &Path) -> TestResult<Vec<Vec<String>>> {
    csv::ReaderBuilder::new()
        .has_headers(false)
        .from_path(directory.join(FILE))?
        .records()
        .map(|record| Ok(record?.iter().map(str::to_owned).collect()))
        .collect()
}

pub(super) fn save(directory: &Path, rows: &[Vec<String>]) -> TestResult {
    let mut writer = csv::WriterBuilder::new()
        .has_headers(false)
        .flexible(true)
        .from_path(directory.join(FILE))?;
    for row in rows {
        writer.write_record(row)?;
    }
    writer.flush()?;
    Ok(())
}
