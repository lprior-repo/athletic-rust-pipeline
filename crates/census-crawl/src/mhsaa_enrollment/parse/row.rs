use std::collections::HashSet;
use std::num::{NonZeroU16, NonZeroU32};

use super::super::MAX_ENROLLMENT_NAME_BYTES;
use super::{EnrollmentParseError, EnrollmentSchool};

pub(super) fn school(
    line: &str,
    page: NonZeroU16,
    expected: usize,
    ids: &mut HashSet<u32>,
) -> Result<EnrollmentSchool, EnrollmentParseError> {
    let number = usize::from(page.get());
    let (ordinal, remaining) = front(line);
    let ordinal = u16::try_from(unsigned(ordinal, number, "ordinal")?)
        .ok()
        .and_then(NonZeroU16::new)
        .ok_or(EnrollmentParseError::Row {
            page: number,
            field: "ordinal",
        })?;
    let (id, remaining) = front(remaining);
    let mhsaa_id =
        NonZeroU32::new(unsigned(id, number, "school id")?).ok_or(EnrollmentParseError::Row {
            page: number,
            field: "school id",
        })?;
    let (remaining, classification) = back(remaining);
    let classification_enrollment = unsigned(classification, number, "classification enrollment")?;
    let (name, total) = back(remaining);
    let total_enrollment = unsigned(total, number, "total enrollment")?;
    let name = name.trim();
    if name.is_empty() {
        return Err(EnrollmentParseError::Row {
            page: number,
            field: "school name",
        });
    }
    if name.len() > MAX_ENROLLMENT_NAME_BYTES {
        return Err(EnrollmentParseError::Limit {
            field: "name bytes",
            maximum: MAX_ENROLLMENT_NAME_BYTES,
        });
    }
    let found = usize::from(ordinal.get());
    if found != expected {
        return Err(EnrollmentParseError::Ordinal { expected, found });
    }
    ids.try_reserve(1)?;
    if !ids.insert(mhsaa_id.get()) {
        return Err(EnrollmentParseError::DuplicateSchool { id: mhsaa_id.get() });
    }
    let mut owned_name = String::new();
    owned_name.try_reserve_exact(name.len())?;
    owned_name.push_str(name);
    Ok(EnrollmentSchool {
        ordinal,
        mhsaa_id,
        name: owned_name,
        total_enrollment,
        classification_enrollment,
        page,
    })
}

fn front(line: &str) -> (&str, &str) {
    let line = line.trim_start();
    match line.split_once(char::is_whitespace) {
        Some((token, remaining)) => (token, remaining.trim_start()),
        None => (line, ""),
    }
}

fn back(line: &str) -> (&str, &str) {
    let line = line.trim_end();
    match line.rsplit_once(char::is_whitespace) {
        Some((remaining, token)) => (remaining.trim_end(), token),
        None => ("", line),
    }
}

fn unsigned(token: &str, page: usize, field: &'static str) -> Result<u32, EnrollmentParseError> {
    if token.is_empty() || !token.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(EnrollmentParseError::Row { page, field });
    }
    token
        .parse()
        .map_err(|_| EnrollmentParseError::Row { page, field })
}
