use census_domain::model::SchoolYear;
use std::collections::HashSet;
use std::num::NonZeroU16;

use super::{
    EnrollmentDocument, EnrollmentParseError, EnrollmentSchool, MAX_ENROLLMENT_LINE_BYTES,
    MAX_ENROLLMENT_PAGES, MAX_ENROLLMENT_ROWS, MAX_ENROLLMENT_TEXT_BYTES,
};

mod header;
mod row;

enum PageStage {
    Title,
    Preamble,
    Columns,
    Rows,
}

pub fn parse_enrollment_text(
    text: &str,
    expected: SchoolYear,
) -> Result<EnrollmentDocument, EnrollmentParseError> {
    let (text, physical_pages) = admit_text(text)?;
    let mut schools = Vec::new();
    let mut ids = HashSet::new();
    text.split('\u{000c}')
        .enumerate()
        .try_for_each(|(index, page)| {
            let number = index.checked_add(1).ok_or(page_limit())?;
            let number = u16::try_from(number)
                .ok()
                .and_then(NonZeroU16::new)
                .ok_or(page_limit())?;
            parse_page(page, number, expected, &mut schools, &mut ids)
        })?;
    if schools.is_empty() {
        return Err(EnrollmentParseError::NoSchools);
    }
    Ok(EnrollmentDocument {
        school_year: expected,
        physical_pages,
        schools,
    })
}

fn admit_text(text: &str) -> Result<(&str, NonZeroU16), EnrollmentParseError> {
    if text.len() > MAX_ENROLLMENT_TEXT_BYTES {
        return Err(EnrollmentParseError::Limit {
            field: "text bytes",
            maximum: MAX_ENROLLMENT_TEXT_BYTES,
        });
    }
    let text = match text.strip_suffix('\u{000c}') {
        Some(terminated) => terminated,
        None => text,
    };
    let (_, pages) = text
        .bytes()
        .try_fold((0usize, 1usize), |(line, pages), byte| {
            if byte == b'\x0c' {
                let pages = pages.checked_add(1).ok_or(page_limit())?;
                if pages > MAX_ENROLLMENT_PAGES {
                    return Err(page_limit());
                }
                return Ok((0, pages));
            }
            if byte == b'\n' {
                return Ok((0, pages));
            }
            let line = line.checked_add(1).ok_or(line_limit())?;
            if line > MAX_ENROLLMENT_LINE_BYTES {
                return Err(line_limit());
            }
            Ok((line, pages))
        })?;
    let physical_pages = u16::try_from(pages)
        .ok()
        .and_then(NonZeroU16::new)
        .ok_or(page_limit())?;
    Ok((
        text.trim_end_matches(|value: char| value.is_ascii_whitespace()),
        physical_pages,
    ))
}

fn page_limit() -> EnrollmentParseError {
    EnrollmentParseError::Limit {
        field: "pages",
        maximum: MAX_ENROLLMENT_PAGES,
    }
}

fn line_limit() -> EnrollmentParseError {
    EnrollmentParseError::Limit {
        field: "line bytes",
        maximum: MAX_ENROLLMENT_LINE_BYTES,
    }
}

fn row_limit() -> EnrollmentParseError {
    EnrollmentParseError::Limit {
        field: "rows",
        maximum: MAX_ENROLLMENT_ROWS,
    }
}

fn parse_page(
    page: &str,
    number: NonZeroU16,
    expected: SchoolYear,
    schools: &mut Vec<EnrollmentSchool>,
    ids: &mut HashSet<u32>,
) -> Result<(), EnrollmentParseError> {
    let stage = page.lines().try_fold(PageStage::Title, |stage, line| {
        let line = line.trim();
        if line.is_empty() {
            return Ok(stage);
        }
        match stage {
            PageStage::Title => {
                header::title(line, expected)?;
                Ok(PageStage::Preamble)
            }
            PageStage::Preamble => header::preamble(line, usize::from(number.get())),
            PageStage::Columns => header::columns(line, usize::from(number.get())),
            PageStage::Rows => {
                append_school(line, number, schools, ids)?;
                Ok(PageStage::Rows)
            }
        }
    })?;
    match stage {
        PageStage::Rows => Ok(()),
        PageStage::Title => Err(EnrollmentParseError::MissingHeader),
        PageStage::Preamble | PageStage::Columns => Err(EnrollmentParseError::Layout {
            page: usize::from(number.get()),
        }),
    }
}

fn append_school(
    line: &str,
    page: NonZeroU16,
    schools: &mut Vec<EnrollmentSchool>,
    ids: &mut HashSet<u32>,
) -> Result<(), EnrollmentParseError> {
    if schools.len() >= MAX_ENROLLMENT_ROWS {
        return Err(row_limit());
    }
    let expected = schools.len().checked_add(1).ok_or(row_limit())?;
    let school = row::school(line, page, expected, ids)?;
    schools.try_reserve(1)?;
    schools.push(school);
    Ok(())
}

#[cfg(test)]
#[path = "parse_tests.rs"]
mod tests;
