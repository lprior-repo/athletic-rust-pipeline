use census_domain::model::SchoolYear;

use super::{EnrollmentParseError, PageStage};

pub(super) fn title(line: &str, expected: SchoolYear) -> Result<(), EnrollmentParseError> {
    let found = declared_year(line).ok_or(EnrollmentParseError::MissingHeader)?;
    if found != expected {
        return Err(EnrollmentParseError::SchoolYear { expected, found });
    }
    Ok(())
}

fn declared_year(line: &str) -> Option<SchoolYear> {
    let academic = line.strip_suffix(" MHSAA Enrollment List")?;
    let (start, end) = academic.split_once('-')?;
    if start.len() != 4 || end.len() != 2 {
        return None;
    }
    if !start
        .bytes()
        .chain(end.bytes())
        .all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    let start = start.parse::<i16>().ok()?;
    let end = end.parse::<i16>().ok()?;
    if start.checked_add(1)?.checked_rem(100)? != end {
        return None;
    }
    SchoolYear::new(start)
}

pub(super) fn preamble(line: &str, page: usize) -> Result<PageStage, EnrollmentParseError> {
    if line
        .split_ascii_whitespace()
        .eq(["Enrollment", "Enrollment"])
    {
        return Ok(PageStage::Columns);
    }
    if line == "Procedures for determining classification are outlined on pages 104 - 105 of the MHSAA Handbook"
        || class_range(line)
    {
        return Ok(PageStage::Preamble);
    }
    Err(EnrollmentParseError::Layout { page })
}

fn class_range(line: &str) -> bool {
    let Some(class) = line.strip_prefix("Class ") else {
        return false;
    };
    let Some((category, range)) = class.trim_start().split_once(char::is_whitespace) else {
        return false;
    };
    if !matches!(category, "A" | "B" | "C" | "D") {
        return false;
    }
    let range = range.trim();
    let numeric = match range
        .strip_suffix("and above")
        .or_else(|| range.strip_suffix("and below"))
    {
        Some(numeric) => numeric.trim(),
        None => range,
    };
    numeric.bytes().any(|byte| byte.is_ascii_digit())
        && numeric.chars().all(|character| {
            character.is_ascii_digit()
                || character.is_ascii_whitespace()
                || matches!(character, '-' | '–')
        })
}

pub(super) fn columns(line: &str, page: usize) -> Result<PageStage, EnrollmentParseError> {
    if line
        .split_ascii_whitespace()
        .eq(["ID", "School", "Total", "Classification"])
    {
        return Ok(PageStage::Rows);
    }
    Err(EnrollmentParseError::Layout { page })
}
