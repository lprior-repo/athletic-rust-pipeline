use super::{archive, failure, row_with, walk, workbook, write, TestResult};
use crate::report::CorruptionClass;
use crate::workbook::verify::canonical::Value;
use crate::workbook::verify::xlsx::{Archive, Profile, WORKBOOK, WORKBOOK_RELS};
use std::path::Path;

pub(super) fn sheet(name: &str, id: &str) -> String {
    format!("<sheet name=\"{name}\" r:id=\"{id}\"/>")
}

fn relationship(id: &str, target: &str) -> String {
    format!(
        "<Relationship Id=\"{id}\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet\" Target=\"{target}\"/>"
    )
}

pub(super) fn book(directory: &Path, sheets: &str, relationships: &str) -> TestResult<Archive> {
    let workbook = format!("<workbook><sheets>{sheets}</sheets></workbook>");
    let rels = format!("<Relationships>{relationships}</Relationships>");
    let bytes = archive(&[
        (
            WORKBOOK,
            workbook.as_bytes(),
            0,
            u32::try_from(workbook.len())?,
        ),
        (
            WORKBOOK_RELS,
            rels.as_bytes(),
            0,
            u32::try_from(rels.len())?,
        ),
    ]);
    let path = write(directory, "worksheet-names.xlsx", &bytes)?;
    Ok(Archive::open(&path, Profile::inline())?)
}

#[test]
fn xlsx_profile_refuses_duplicate_worksheet_names() -> TestResult {
    let directory = tempfile::tempdir()?;
    for repeated in ["Athletes", "ATHLETES"] {
        let sheets = format!("{}{}", sheet("Athletes", "rId1"), sheet(repeated, "rId2"));
        let rels = format!(
            "{}{}",
            relationship("rId1", "worksheets/sheet1.xml"),
            relationship("rId2", "worksheets/sheet2.xml")
        );
        let error = book(directory.path(), &sheets, &rels)?
            .sheet_names()
            .err()
            .ok_or("duplicate worksheet names were accepted")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(error.to_string().contains("worksheet name"), "{error}");
        check!(error.to_string().contains("repeated"), "{error}");
    }
    Ok(())
}

#[test]
fn xlsx_profile_refuses_reused_worksheet_relationships() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheets = format!("{}{}", sheet("Athletes", "rId1"), sheet("PRs", "rId1"));
    let rels = relationship("rId1", "worksheets/sheet1.xml");
    let error = book(directory.path(), &sheets, &rels)?
        .sheet_names()
        .err()
        .ok_or("a reused worksheet relationship was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    check!(
        error.to_string().contains("relationship rId1 is reused"),
        "{error}"
    );
    Ok(())
}

#[test]
fn xlsx_profile_refuses_duplicate_worksheet_relationship_targets() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheets = sheet("Athletes", "rId1");
    let rels = format!(
        "{}{}",
        relationship("rId1", "worksheets/unverified.xml"),
        relationship("rId1", "worksheets/verified.xml")
    );
    let error = book(directory.path(), &sheets, &rels)?
        .sheet_names()
        .err()
        .ok_or("an overwritten worksheet relationship was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    check!(
        error.to_string().contains("relationship rId1 is repeated"),
        "{error}"
    );
    Ok(())
}

#[test]
fn xlsx_profile_refuses_relationship_ids_shadowed_by_other_types() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheets = sheet("Athletes", "rId1");
    let worksheet = relationship("rId1", "worksheets/verified.xml");
    let styles = "<Relationship Id=\"rId1\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>";
    for rels in [
        format!("{worksheet}{styles}"),
        format!("{styles}{worksheet}"),
    ] {
        let error = book(directory.path(), &sheets, &rels)?
            .sheet_names()
            .err()
            .ok_or("a shadowed worksheet relationship was accepted")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(
            error.to_string().contains("relationship rId1 is repeated"),
            "{error}"
        );
    }
    Ok(())
}

#[test]
fn xlsx_profile_preserves_distinct_worksheet_names_and_relationships() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheets = format!("{}{}", sheet("Athletes", "rId1"), sheet("PRs", "rId2"));
    let rels = format!(
        "{}{}{}",
        relationship("rId1", "worksheets/sheet1.xml"),
        relationship("rId2", "worksheets/sheet2.xml"),
        "<Relationship Id=\"rId3\" Type=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships/styles\" Target=\"styles.xml\"/>"
    );
    check!(eq; book(directory.path(), &sheets, &rels)?.sheet_names()?, vec!["Athletes", "PRs"]);
    Ok(())
}

#[test]
fn xlsx_profile_refuses_worksheet_metadata_beyond_member_budget() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheets: String = (0..1_025)
        .map(|index| sheet(&format!("Sheet{index}"), &format!("rId{index}")))
        .collect();
    let rels: String = (0..1_025)
        .map(|index| relationship(&format!("rId{index}"), "worksheets/sheet.xml"))
        .collect();
    for (sheets, rels, expected) in [
        (sheets.as_str(), "", "worksheet table"),
        ("", rels.as_str(), "relationship table"),
    ] {
        let error = book(directory.path(), sheets, rels)?
            .sheet_names()
            .err()
            .ok_or("unbounded worksheet metadata was accepted")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(error.to_string().contains(expected), "{error}");
        check!(error.to_string().contains("member budget"), "{error}");
    }
    Ok(())
}

#[test]
fn xlsx_profile_refuses_missing_and_empty_worksheet_identifiers() -> TestResult {
    let directory = tempfile::tempdir()?;
    let rels = relationship("rId1", "worksheets/sheet1.xml");
    for (sheets, expected) in [
        ("<sheet r:id=\"rId1\"/>", "lacks its name attribute"),
        ("<sheet name=\"\" r:id=\"rId1\"/>", "empty name"),
        ("<sheet name=\"Athletes\"/>", "lacks its relationship id"),
        (
            "<sheet name=\"Athletes\" r:id=\"\"/>",
            "lacks its relationship id",
        ),
    ] {
        let error = book(directory.path(), sheets, &rels)?
            .sheet_names()
            .err()
            .ok_or("a worksheet without an identifier was accepted")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(error.to_string().contains(expected), "{error}");
    }
    Ok(())
}

#[test]
fn xlsx_profile_refuses_empty_relationship_identifiers() -> TestResult {
    let directory = tempfile::tempdir()?;
    let rels = relationship("", "worksheets/sheet1.xml");
    let error = book(directory.path(), &sheet("Athletes", "rId1"), &rels)?
        .sheet_names()
        .err()
        .ok_or("an empty relationship identifier was accepted")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
    check!(error.to_string().contains("empty Id"), "{error}");
    Ok(())
}

#[test]
fn xlsx_profile_refuses_repeated_worksheet_attributes() -> TestResult {
    let directory = tempfile::tempdir()?;
    let rels = relationship("rId1", "worksheets/sheet1.xml");
    for sheets in [
        "<sheet name=\"Athletes\" name=\"Unverified\" r:id=\"rId1\"/>",
        "<sheet r:id=\"rId1\" name=\"Athletes\" r:id=\"rId2\"/>",
        "<sheet name=\"Athletes\" name=\"\" r:id=\"rId1\"/>",
    ] {
        let error = book(directory.path(), sheets, &rels)?
            .sheet_names()
            .err()
            .ok_or("a repeated worksheet attribute was accepted")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(error.to_string().contains("XML attribute"), "{error}");
        check!(error.to_string().contains("repeated"), "{error}");
    }
    Ok(())
}

#[test]
fn xlsx_profile_refuses_repeated_cell_attributes() -> TestResult {
    let directory = tempfile::tempdir()?;
    for cell in [
        "<c r=\"A1\" r=\"B1\"><v>1</v></c>",
        "<c t=\"n\" r=\"A1\" t=\"s\"><v>1</v></c>",
        "<c r=\"A1\" r=\"\"><v>1</v></c>",
    ] {
        let path = write(
            directory.path(),
            "duplicate-cell-attributes.xlsx",
            &workbook(&row_with(cell)),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let error = failure(&archive, "Sheet1")?;
        check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt));
        check!(error.to_string().contains("XML attribute"), "{error}");
        check!(error.to_string().contains("repeated"), "{error}");
    }
    Ok(())
}

#[test]
fn xlsx_profile_preserves_distinct_qualified_attributes_and_attribute_order() -> TestResult {
    let directory = tempfile::tempdir()?;
    for cell in [
        "<c r=\"A1\" t=\"n\" xmlns:a=\"urn:a\" xmlns:b=\"urn:b\" a:flag=\"first\" b:flag=\"second\"><v>1</v></c>",
        "<c b:flag=\"second\" a:flag=\"first\" xmlns:b=\"urn:b\" xmlns:a=\"urn:a\" t=\"n\" r=\"A1\"><v>1</v></c>",
    ] {
        let path = write(
            directory.path(),
            "qualified-cell-attributes.xlsx",
            &workbook(&row_with(cell)),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let (walked, written) = walk(&archive, "Sheet1")?;
        check!(eq; written, Some(1));
        check!(eq; walked.cells.len(), 1);
        check!(eq; walked.cells.get(&(0, 0)), Some(&Value::Number(1.0)));
    }
    Ok(())
}
