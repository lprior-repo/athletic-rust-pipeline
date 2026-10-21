use super::names::{book, sheet};
use super::TestResult;
use crate::report::CorruptionClass;

#[test]
fn xlsx_profile_refuses_external_and_unknown_worksheet_target_modes() -> TestResult {
    let directory = tempfile::tempdir()?;
    for mode in ["External", "", "Unknown", "internal"] {
        let rels = format!(
            "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\" TargetMode=\"{mode}\"/>"
        );
        let error = book(directory.path(), &sheet("Athletes", "rId1"), &rels)?
            .sheet_names()
            .err()
            .ok_or("a noninternal worksheet relationship was accepted as a local member")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(
            error.to_string().contains("internal package member"),
            "{error}"
        );
    }
    Ok(())
}

#[test]
fn xlsx_profile_preserves_internal_worksheets_and_external_nonworksheet_targets() -> TestResult {
    let directory = tempfile::tempdir()?;
    for mode in ["", " TargetMode=\"Internal\""] {
        let rels = format!(
            "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"worksheets/sheet1.xml\"{mode}/><Relationship Id=\"rId2\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/hyperlink\" Target=\"https://fixture.test/source\" TargetMode=\"External\"/>"
        );
        check!(
            eq;
            book(directory.path(), &sheet("Athletes", "rId1"), &rels)?.sheet_names()?,
            vec!["Athletes"]
        );
    }
    Ok(())
}
