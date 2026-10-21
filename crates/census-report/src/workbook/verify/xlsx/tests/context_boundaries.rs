use super::{
    failure, row_with, sheet_with, walk, workbook, write, Archive, Profile, TestResult, Value,
};
use crate::report::CorruptionClass;

const NAMESPACE: &str = "http://schemas.openxmlformats.org/spreadsheetml/2006/main";

fn refuses(sheet: &str) -> TestResult {
    let directory = tempfile::tempdir()?;
    let path = write(directory.path(), "context.xlsx", &workbook(sheet))?;
    let archive = Archive::open(&path, Profile::inline())?;
    let error = failure(&archive, "Sheet1")?;
    check!(eq; error.corruption(), Some(CorruptionClass::ArtifactCorrupt), "{sheet}: {error}");
    Ok(())
}

#[test]
fn xlsx_context_refuses_namespace_overrides_on_semantic_elements() -> TestResult {
    let cells = [
        "<c r=\"A1\" xmlns=\"urn:foreign\"><v>1</v></c>",
        "<c r=\"A1\"><v xmlns=\"urn:foreign\">1</v></c>",
        "<c r=\"A1\" t=\"inlineStr\"><is xmlns=\"\"><t>lost</t></is></c>",
        "<c r=\"A1\" t=\"inlineStr\"><is><r xmlns=\"urn:foreign\"><t>lost</t></r></is></c>",
        "<c r=\"A1\" t=\"inlineStr\"><is><t xmlns=\"\">lost</t></is></c>",
    ];
    for cell in cells {
        refuses(&row_with(cell))?;
    }
    refuses(&sheet_with(
        "A1",
        "<row xmlns=\"urn:foreign\" r=\"1\"><c r=\"A1\"><v>1</v></c></row>",
    ))?;
    Ok(())
}

#[test]
fn xlsx_context_restores_default_namespace_after_ancillary_scopes() -> TestResult {
    let directory = tempfile::tempdir()?;
    for ancillary in [
        "<extLst xmlns=\"urn:foreign\"><ext/></extLst>",
        "<extLst xmlns=\"urn:foreign\"/>",
        "<extLst xmlns=\"\"><ext/></extLst>",
    ] {
        let sheet = format!(
            "<worksheet xmlns=\"{NAMESPACE}\">{ancillary}<sheetData><row r=\"1\"><c r=\"A1\"><v>42</v></c></row></sheetData></worksheet>"
        );
        let path = write(
            directory.path(),
            "namespace-restored.xlsx",
            &workbook(&sheet),
        )?;
        let archive = Archive::open(&path, Profile::inline())?;
        let (walked, written) = walk(&archive, "Sheet1")?;
        check!(eq; written, Some(1), "{ancillary}");
        check!(eq; walked.cells.len(), 1, "{ancillary}");
        check!(eq; walked.cells.get(&(0, 0)), Some(&Value::Number(42.0)), "{ancillary}");
    }
    Ok(())
}

#[test]
fn xlsx_context_restores_namespace_before_rich_text_siblings() -> TestResult {
    let directory = tempfile::tempdir()?;
    let cell = "<c r=\"A1\" t=\"inlineStr\"><is><r><rPr xmlns=\"urn:foreign\"><b/></rPr><t xml:space=\"preserve\">Owen </t></r><r><rPr xmlns=\"\"/><t>Courser</t></r></is></c>";
    let path = write(
        directory.path(),
        "rich-context.xlsx",
        &workbook(&row_with(cell)),
    )?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = walk(&archive, "Sheet1")?;
    check!(eq; written, Some(1));
    check!(eq; walked.cells.len(), 1);
    check!(eq; walked.cells.get(&(0, 0)), Some(&Value::text("Owen Courser")));
    Ok(())
}

#[test]
fn xlsx_context_refuses_mixed_and_self_closing_value_containers() -> TestResult {
    let payloads = [
        "<v>1</v><is><t>2</t></is>",
        "<is><t>1</t></is><v>2</v>",
        "<v/><v>2</v>",
        "<v>1</v><v/>",
        "<is/><is><t>2</t></is>",
        "<is><t>1</t></is><is/>",
        "<v/><is/>",
        "<is/><v/>",
    ];
    for payload in payloads {
        refuses(&row_with(&format!(
            "<c r=\"A1\" t=\"inlineStr\">{payload}</c>"
        )))?;
    }
    Ok(())
}

#[test]
fn xlsx_context_refuses_transparent_wrappers_and_scalar_children() -> TestResult {
    let cells = [
        "<c r=\"A1\"><extLst><v>1</v></extLst></c>",
        "<c r=\"A1\"><v>1<ext/>2</v></c>",
        "<c r=\"A1\" t=\"inlineStr\"><is><extLst><t>lost</t></extLst></is></c>",
        "<c r=\"A1\" t=\"inlineStr\"><is><rPr><t>lost</t></rPr></is></c>",
        "<c r=\"A1\" t=\"inlineStr\"><is><t>a<ext/>b</t></is></c>",
        "<c r=\"A1\" t=\"inlineStr\"><r><t>lost</t></r></c>",
        "<c r=\"A1\"><c r=\"B1\"><v>1</v></c></c>",
        "<c r=\"A1\"><dimension ref=\"A1\"/><v>1</v></c>",
    ];
    for cell in cells {
        refuses(&row_with(cell))?;
    }
    let sheet = format!(
        "<worksheet xmlns=\"{NAMESPACE}\"><sheetData><extLst xmlns=\"urn:foreign\"><row xmlns=\"{NAMESPACE}\" r=\"1\"><c r=\"A1\"><v>1</v></c></row></extLst></sheetData></worksheet>"
    );
    refuses(&sheet)
}

#[test]
fn xlsx_context_requires_one_root_and_one_sheet_data() -> TestResult {
    for sheet in [
        format!("<worksheet xmlns=\"{NAMESPACE}\"/>"),
        format!("<worksheet xmlns=\"{NAMESPACE}\"><sheetData/><sheetData/></worksheet>"),
        format!("<worksheet xmlns=\"{NAMESPACE}\"><sheetData/></worksheet><worksheet xmlns=\"{NAMESPACE}\"><sheetData/></worksheet>"),
        format!("<extLst><worksheet xmlns=\"{NAMESPACE}\"><sheetData/></worksheet></extLst>"),
    ] {
        refuses(&sheet)?;
    }
    let directory = tempfile::tempdir()?;
    let sheet = format!("<worksheet xmlns=\"{NAMESPACE}\"><sheetData/></worksheet>");
    let path = write(directory.path(), "empty-context.xlsx", &workbook(&sheet))?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = walk(&archive, "Sheet1")?;
    check!(eq; written, Some(0));
    check!(eq; walked.rows, Some(0));
    check!(eq; walked.cells.len(), 0);
    Ok(())
}

#[test]
fn xlsx_context_refuses_qualified_elements_and_malformed_closing_tags() -> TestResult {
    for cell in [
        "<foreign:c xmlns:foreign=\"urn:foreign\" r=\"A1\"/>",
        "<c r=\"A1\"><v>1</v></foreign:c>",
        "<c r=\"A1\"><v>1</v></c/>",
        "<c r=\"A1\"><v>1</v></c r=\"A1\">",
    ] {
        refuses(&row_with(cell))?;
    }
    Ok(())
}

#[test]
fn xlsx_context_resets_value_guard_only_at_the_next_cell() -> TestResult {
    let directory = tempfile::tempdir()?;
    let sheet = sheet_with(
        "A1:D1",
        "<row r=\"1\"><c r=\"A1\"/><c r=\"B1\"><v/></c><c r=\"C1\" t=\"inlineStr\"><is/></c><c r=\"D1\"><v>42</v></c></row>",
    );
    let path = write(directory.path(), "empty-values.xlsx", &workbook(&sheet))?;
    let archive = Archive::open(&path, Profile::inline())?;
    let (walked, written) = walk(&archive, "Sheet1")?;
    check!(eq; written, Some(1));
    check!(eq; walked.cells.len(), 4);
    check!(eq; walked.cells.get(&(0, 0)), Some(&Value::Empty));
    check!(eq; walked.cells.get(&(0, 1)), Some(&Value::Empty));
    check!(eq; walked.cells.get(&(0, 2)), Some(&Value::Empty));
    check!(eq; walked.cells.get(&(0, 3)), Some(&Value::Number(42.0)));
    Ok(())
}

#[test]
fn xlsx_context_refuses_late_or_repeated_dimensions() -> TestResult {
    for body in [
        "<sheetData><row r=\"3\"><c r=\"C3\"><v>1</v></c></row></sheetData><dimension ref=\"A1\"/>",
        "<dimension ref=\"A1\"/><dimension ref=\"A1\"/><sheetData/>",
    ] {
        refuses(&format!(
            "<worksheet xmlns=\"{NAMESPACE}\">{body}</worksheet>"
        ))?;
    }
    Ok(())
}
