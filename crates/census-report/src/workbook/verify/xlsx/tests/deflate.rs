use super::fixture::{self, failure, write};
use super::TestResult;
use crate::report::CorruptionClass;
use crate::workbook::verify::xlsx::{Archive, Profile};

#[test]
fn verify_corrupt_deflate_code_table_that_is_over_subscribed_is_refused() -> TestResult {
    let directory = tempfile::tempdir()?;
    let stream = [0x04_u8, 0x00, 0x92, 0x00];
    let bytes = fixture::workbook_archive(&stream, 8, 64);
    let path = write(directory.path(), "over-subscribed.xlsx", &bytes)?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{error}");
    check!(error.to_string().contains("over-subscribed"), "{error}");
    Ok(())
}
