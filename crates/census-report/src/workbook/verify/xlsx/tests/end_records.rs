use super::{fixture, Archive, CorruptionClass, Profile, TestResult, Value};

#[test]
fn xlsx_profile_refuses_competing_zip_end_records() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let second = fixture::workbook(&fixture::row_with("<c r=\"A1\"><v>999</v></c>"));
    let comment = u16::try_from(second.len().checked_add(8).ok_or("comment overflow")?)?;
    let end = bytes.len();
    bytes
        .get_mut(end.checked_sub(2).ok_or("short end record")?..end)
        .ok_or("missing comment length")?
        .copy_from_slice(&comment.to_le_bytes());
    bytes.extend_from_slice(&second);
    bytes.extend_from_slice(&[0; 8]);
    let path = fixture::write(directory.path(), "competing-ends.xlsx", &bytes)?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("competing central-directory interpretations were accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn xlsx_profile_refuses_unequal_zip_disk_and_total_entry_counts() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let field = bytes.len().checked_sub(14).ok_or("short end record")?;
    bytes
        .get_mut(field..field.checked_add(2).ok_or("count offset overflow")?)
        .ok_or("missing on-disk entry count")?
        .copy_from_slice(&0_u16.to_le_bytes());
    let path = fixture::write(directory.path(), "different-entry-counts.xlsx", &bytes)?;
    let error = Archive::open(&path, Profile::inline())
        .err()
        .ok_or("inconsistent central-directory counts were accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    Ok(())
}

#[test]
fn xlsx_profile_preserves_literal_zip_comment_without_another_end_record() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let comment = b"Source note: PK\x05\x06this is ordinary literal text only";
    let end = bytes.len();
    bytes
        .get_mut(end.checked_sub(2).ok_or("short end record")?..end)
        .ok_or("missing comment length")?
        .copy_from_slice(&u16::try_from(comment.len())?.to_le_bytes());
    bytes.extend_from_slice(comment);
    let path = fixture::write(directory.path(), "literal-comment.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = fixture::walk(&archive, "Sheet1")?;
    check!(eq; written, Some(3));
    check!(eq; walked.cells.get(&(0, 0)), Some(&Value::text("Name")));
    check!(eq; walked.cells.get(&(1, 1)), Some(&Value::Number(11.5)));
    Ok(())
}
