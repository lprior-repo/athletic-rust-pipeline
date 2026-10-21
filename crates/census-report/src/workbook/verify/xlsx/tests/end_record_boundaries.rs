use super::{fixture, Archive, CorruptionClass, Profile, TestResult, Value};

#[test]
fn xlsx_profile_refuses_nonzero_eocd_disk_number() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let disk_field = bytes
        .len()
        .checked_sub(22)
        .and_then(|base| base.checked_add(4))
        .ok_or("short EOCD record")?;
    bytes
        .get_mut(disk_field..disk_field.checked_add(2).ok_or("disk field overflow")?)
        .ok_or("missing EOCD disk number field")?
        .copy_from_slice(&1_u16.to_le_bytes());
    let path = fixture::write(directory.path(), "disk-number.xlsx", &bytes)?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("nonzero EOCD disk number was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_nonzero_central_directory_disk_number() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let cd_disk_field = bytes
        .len()
        .checked_sub(22)
        .and_then(|base| base.checked_add(6))
        .ok_or("short EOCD record")?;
    bytes
        .get_mut(cd_disk_field..cd_disk_field.checked_add(2).ok_or("field overflow")?)
        .ok_or("missing CD disk number field")?
        .copy_from_slice(&1_u16.to_le_bytes());
    let path = fixture::write(directory.path(), "cd-disk-number.xlsx", &bytes)?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("nonzero central directory disk number was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_data_between_central_directory_and_eocd() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let end = bytes.len().checked_sub(22).ok_or("short EOCD record")?;
    bytes.splice(end..end, [0; 10]);
    let path = fixture::write(directory.path(), "trailing-data.xlsx", &bytes)?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("trailing data between central directory and EOCD was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn xlsx_profile_accepts_maximum_legal_eocd_comment() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let end = bytes.len();
    bytes
        .get_mut(end.checked_sub(2).ok_or("short EOCD")?..end)
        .ok_or("missing comment length field")?
        .copy_from_slice(&u16::MAX.to_le_bytes());
    let comment = vec![b'x'; usize::from(u16::MAX)];
    bytes.extend_from_slice(&comment);
    let path = fixture::write(directory.path(), "max-comment.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = fixture::walk(&archive, "Sheet1")?;
    check!(eq; written, Some(3));
    check!(eq; walked.cells.get(&(0, 0)), Some(&Value::text("Name")));
    check!(eq; walked.cells.get(&(1, 1)), Some(&Value::Number(11.5)));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_eocd_comment_exceeding_u16_length() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let end = bytes.len();
    bytes
        .get_mut(end.checked_sub(2).ok_or("short EOCD")?..end)
        .ok_or("missing comment length field")?
        .copy_from_slice(&u16::MAX.to_le_bytes());
    let comment = vec![b'x'; usize::from(u16::MAX) + 1];
    bytes.extend_from_slice(&comment);
    let path = fixture::write(directory.path(), "overflow-comment.xlsx", &bytes)?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("65536-byte comment exceeding u16 length was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}
