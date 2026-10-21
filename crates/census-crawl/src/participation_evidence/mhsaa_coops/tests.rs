use super::*;
use census_domain::model::{Gender, Sport};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;
const PDF: &[u8] = include_bytes!("tests/fixtures/official-approved-coops-2026-27.pdf");
const PDF_SHA: &str = "80fe9a2f50949b90143212dbc505452f1a4d15f1ccf9ddd52215fbb73008c7d4";

mod refusals;

fn decoded() -> CrawlResult<MhsaaCoopPdfCandidateV1<'static>> {
    decode_mhsaa_coop_candidate_v1(MHSAA_COOPS_SOURCE_URL, PDF)
}

fn locator(document: &MhsaaCoopPdfCandidateV1<'_>, span: &Locator) -> TestResult {
    let raw = document.literal(span).ok_or("missing candidate span")?;
    check!(eq; span.sha256(), hash(raw.as_bytes()));
    check!(eq; span.byte_length(), raw.len());
    check!(eq; span.page(), 24);
    Ok(())
}

#[test]
fn genuine_current_coop_pdf_preserves_original_and_literal_program_candidates() -> TestResult {
    let document = decoded()?;
    check!(std::ptr::eq(
        document.original_bytes().as_ptr(),
        PDF.as_ptr()
    ));
    check!(eq; PDF.len(), 123213);
    check!(eq; document.source_sha256(), PDF_SHA);
    check!(eq; document.source_url(), MHSAA_COOPS_SOURCE_URL);
    check!(eq; document.layout_sha256(), hash(document.layout_text().as_bytes()));
    check!(eq; document.school_year().get(), 2026);
    check!(eq; document.literal(document.school_year_label()), Some("2026-2027"));
    check!(eq; document.literal(document.revision_label()), Some("23-Jun-26"));
    check!(eq; document.primary().name(), "Pickford");
    check!(eq; document.primary().enrollment(), 139);
    check!(eq; document.partner().name(), "Kinross Maplewood Baptist");
    check!(eq; document.partner().enrollment(), 23);
    check!(eq; document.combined_enrollment(), 162);
    check!(eq; document.sport(), Sport::CrossCountry);
    check!(eq; document.category(), Gender::Boys);
    check!(eq; document.peninsula_literal(), "UP");
    check!(eq; document.division(), 3);
    check!(eq; document.literal(document.approval_label()), Some("8/21/2024"));
    let row = document
        .literal(document.program_row())
        .ok_or("missing program row")?;
    check!(row.contains("(3) Cross Country (Boys) - UP"));
    check!(row.ends_with("8/21/2024"));
    for span in [
        document.heading(),
        document.school_year_label(),
        document.revision_label(),
        document.group_header(),
        document.program_row(),
        document.approval_label(),
        document.primary().name_locator(),
        document.primary().enrollment_locator(),
        document.partner().name_locator(),
        document.partner().enrollment_locator(),
    ] {
        locator(&document, span)?;
    }
    Ok(())
}

#[test]
fn actual_converter_rebases_a_shifted_pdf_without_pinning_original_byte_offsets() -> TestResult {
    let original = decoded()?;
    let mut shifted = b"% source candidate offset shift\n".to_vec();
    shifted.extend_from_slice(PDF);
    let document = decode_mhsaa_coop_candidate_v1(MHSAA_COOPS_SOURCE_URL, &shifted)?;
    check!(eq; document.original_bytes(), shifted.as_slice());
    check!(eq; document.source_sha256(), hash(&shifted));
    check!(ne; document.source_sha256(), original.source_sha256());
    check!(eq; document.layout_text(), original.layout_text());
    check!(eq; document.program_row(), original.program_row());
    Ok(())
}

#[test]
fn pure_parser_rebases_layout_shifts_and_flexible_column_spacing() -> TestResult {
    let original = decoded()?;
    let document = document::read(original.layout_text())?;
    let header = original
        .literal(&document.group.header)
        .ok_or("missing header")?;
    let flexible = header.replace("                          ", "  ");
    check!(ne; flexible, header);
    let mut shifted = original.layout_text().to_string();
    shifted.replace_range(document.group.header.range(), &flexible);
    shifted.insert_str(0, "\n\n");
    let parsed = candidate(PDF, shifted)?;
    check!(eq; parsed.primary().enrollment(), 139);
    check!(eq; parsed.partner().enrollment(), 23);
    check!(eq; parsed.literal(parsed.primary().name_locator()), Some("Pickford"));
    check!(eq; parsed.literal(parsed.partner().name_locator()), Some("Kinross Maplewood Baptist"));
    check!(ne; parsed.program_row().byte_offset(), original.program_row().byte_offset());
    locator(&parsed, parsed.program_row())?;
    Ok(())
}

fn changed_span(layout: &str, span: &Locator, old: &str, new: &str) -> TestResult<String> {
    let raw = layout
        .get(span.range())
        .ok_or("mutation span is outside actual layout")?;
    check!(raw.contains(old));
    let mut changed = layout.to_string();
    changed.replace_range(span.range(), &raw.replacen(old, new, 1));
    Ok(changed)
}
