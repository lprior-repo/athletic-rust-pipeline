use crate::milesplit::SourceRowLocator;
use crate::participation_evidence::result_ownership::{
    read_retained_html_result_rows, CheckedRetainedResultRowV1,
};
use crate::participation_evidence::{
    invalid, DocumentSubjectV1, RetainedHtmlCaptureV1, SubjectOwnershipV1,
};
use census_domain::model::{Grade, ProfileDate};
use census_store::StoreResult;

pub(super) fn read(
    capture: &RetainedHtmlCaptureV1,
    locator: SourceRowLocator,
) -> StoreResult<crate::participation_evidence::status::Status> {
    let document = read_retained_html_result_rows(capture, &[locator]).map_err(ownership_error)?;

    let row = document
        .row(0)
        .ok_or_else(|| invalid("original result document has no selected row"))?;

    let parsed = row.parsed_row();
    let event = row.event();
    if parsed.school != "Detroit Catholic Central" {
        return Err(invalid(
            "original result school is outside the supported DCC status scope",
        ));
    }

    if event.kind.is_relay() {
        return Err(invalid(
            "relay event cannot establish individual student status",
        ));
    }

    let grade = parsed
        .grade
        .ok_or_else(|| invalid("original result row has no explicit student grade"))?;

    let published_start = document.published_start();
    let day = ProfileDate::parse(published_start)
        .map_err(|_| invalid("original result document publication day is unsupported"))?;

    if let Some(end) = document.published_end() {
        if end != published_start {
            return Err(invalid(
                "multiday meet cannot establish a single competition day",
            ));
        }
    }

    let subject = student_statement(&row, grade)?;
    let ownership = SubjectOwnershipV1::DocumentOnly(subject.clone());

    Ok(crate::participation_evidence::status::Status {
        subject,
        ownership,
        day,
        grade,
    })
}

fn student_statement(
    row: &CheckedRetainedResultRowV1<'_>,
    grade: Grade,
) -> StoreResult<DocumentSubjectV1> {
    let original = row.original_row().map_err(ownership_error)?;
    let parsed = row.parsed_row();
    let school_start = original
        .find(&parsed.school)
        .ok_or_else(|| invalid("student school is not literal original result text"))?;
    let name_start = original
        .get(..school_start)
        .and_then(|prefix| prefix.find(&parsed.name))
        .ok_or_else(|| invalid("student name is not literal original result text"))?;
    let name_end = name_start
        .checked_add(parsed.name.len())
        .ok_or_else(|| invalid("student name span overflow"))?;
    let literal_grade = original
        .get(name_end..school_start)
        .and_then(|text| text.trim().parse::<u8>().ok());
    if literal_grade != Some(grade.get()) {
        return Err(invalid("student grade is not literal original result text"));
    }
    let offset = row.locator().byte_offset;
    let start = offset
        .checked_add(name_start)
        .ok_or_else(|| invalid("student statement span overflow"))?;
    let end = offset
        .checked_add(school_start)
        .and_then(|value| value.checked_add(parsed.school.len()))
        .ok_or_else(|| invalid("student statement span overflow"))?;
    let mut subject = row.subject().map_err(ownership_error)?;
    subject.locator = format!("html_bytes:{start}:{end}");
    Ok(subject)
}

fn ownership_error(
    error: crate::participation_evidence::result_ownership::RetainedResultOwnershipError,
) -> census_store::StoreError {
    match error {
        crate::participation_evidence::result_ownership::RetainedResultOwnershipError::Capture(
            error,
        ) => error,
        other => census_store::StoreError::Refused {
            detail: format!("original result student status: {other}"),
        },
    }
}

#[cfg(test)]
mod tests;
