use super::fixture;
use super::TestResult;
use crate::report::CorruptionClass;
use crate::workbook::verify::xlsx::{Archive, Profile};

#[test]
fn xlsx_profile_reads_a_deflated_workbook() -> TestResult {
    let directory = tempfile::tempdir()?;
    let bytes = fixture::deflated_workbook(&fixture::inline_sheet());
    let path = fixture::write(directory.path(), "deflated.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = fixture::walk(&archive, "Sheet1")?;
    check!(eq; walked.cells.len(), 6, "the deflated lane reads every cell");
    check!(eq; walked.rows, Some(3));
    check!(eq; written, Some(3));
    Ok(())
}

#[test]
fn verify_corrupt_deflate_stream_with_a_broken_length_pair_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = fixture::inline_sheet();
    let mut stream = fixture::deflate_stored(sheet.as_bytes());
    match stream.get_mut(3) {
        Some(byte) => *byte ^= 0xFF,
        None => return Err("the deflate stream is trivially short".into()),
    }
    let declared = u32::try_from(sheet.len()).map_or(u32::MAX, |length| length);
    let bytes = fixture::workbook_archive(&stream, 8, declared);
    let path = fixture::write(directory.path(), "complement.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = fixture::failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("does not complement"), "{error}");
    Ok(())
}

#[test]
fn verify_corrupt_deflate_stream_with_a_reserved_block_type_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = fixture::inline_sheet();
    let mut stream = fixture::deflate_stored(sheet.as_bytes());
    match stream.get_mut(0) {
        Some(byte) => *byte = 0x07,
        None => return Err("the deflate stream is trivially short".into()),
    }
    let declared = u32::try_from(sheet.len()).map_or(u32::MAX, |length| length);
    let bytes = fixture::workbook_archive(&stream, 8, declared);
    let path = fixture::write(directory.path(), "reserved.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = fixture::failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("reserved block type"), "{error}");
    Ok(())
}

#[test]
fn verify_corrupt_member_pointing_at_the_wrong_offset_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let mut bytes = fixture::workbook(&fixture::inline_sheet());
    let name = b"xl/workbook.xml";
    let found = bytes
        .windows(name.len())
        .rposition(|window| window == name)
        .ok_or("the member name is missing")?;
    let offset = found.checked_sub(4).ok_or("the central record is short")?;
    match bytes.get_mut(offset..found) {
        Some(field) => field.copy_from_slice(&0_u32.to_le_bytes()),
        None => return Err("the central record is short".into()),
    }
    let path = fixture::write(directory.path(), "mispointed.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = archive
        .sheet_names()
        .err()
        .ok_or("a mispointed member was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("disagrees"), "{error}");
    Ok(())
}
