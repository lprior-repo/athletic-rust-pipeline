pub(crate) mod acquisition;
mod limits;
mod outcome;
pub use outcome::ReadOutcome;
mod detail;
pub use detail::issue_detail;

use census_domain::school_directory::{
    CityName, Coordinates, DirectoryError, Grade, GradeSpan, PostalAddress, StreetLine, ZipCode,
};
use census_domain::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct RowIssue {
    pub line: usize,
    pub field: &'static str,
    pub detail: String,
}

impl RowIssue {
    pub fn new(
        line: usize,
        field: &'static str,
        detail: impl AsRef<str>,
    ) -> Result<Self, DirectoryError> {
        limits::check("directory issue field bytes", field.len(), 128)?;
        Ok(Self {
            line,
            field,
            detail: limits::text(detail.as_ref())?,
        })
    }

    pub fn render(&self) -> String {
        format!("line {}: {} {}", self.line, self.field, self.detail)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ReadCounts {
    pub entries: usize,
    pub skipped: usize,
    pub notes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFailure {
    pub field: &'static str,
    pub error: DirectoryError,
}

pub fn field<T>(name: &'static str, parsed: Result<T, DirectoryError>) -> Result<T, FieldFailure> {
    parsed.map_err(|error| FieldFailure { field: name, error })
}

pub fn optional<T>(
    name: &'static str,
    parsed: Result<Option<T>, DirectoryError>,
) -> Result<Option<T>, FieldFailure> {
    field(name, parsed)
}

pub fn skip_row<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    field: &'static str,
    parsed: Result<T, DirectoryError>,
) -> Result<Option<T>, DirectoryError> {
    match parsed {
        Ok(value) => Ok(Some(value)),
        Err(error) if error.is_resource() => Err(error),
        Err(error) => {
            outcome.skip(line, field, issue_detail(format_args!("{error}"))?)?;
            Ok(None)
        }
    }
}

pub fn skip_optional<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    field: &'static str,
    parsed: Result<Option<T>, DirectoryError>,
) -> Result<Option<T>, DirectoryError> {
    match parsed {
        Ok(value) => Ok(value),
        Err(error) if error.is_resource() => Err(error),
        Err(error) => {
            outcome.note(line, field, issue_detail(format_args!("{error}"))?)?;
            Ok(None)
        }
    }
}

pub fn skip_failed<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    parsed: Result<T, FieldFailure>,
) -> Result<Option<T>, DirectoryError> {
    match parsed {
        Ok(value) => Ok(Some(value)),
        Err(failure) if failure.error.is_resource() => Err(failure.error),
        Err(failure) => {
            outcome.note(
                line,
                failure.field,
                issue_detail(format_args!("{}", failure.error))?,
            )?;
            Ok(None)
        }
    }
}

pub fn skip_absent<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    parsed: Result<Option<T>, FieldFailure>,
) -> Result<Option<T>, DirectoryError> {
    match parsed {
        Ok(value) => Ok(value),
        Err(failure) if failure.error.is_resource() => Err(failure.error),
        Err(failure) => {
            outcome.note(
                line,
                failure.field,
                issue_detail(format_args!("{}", failure.error))?,
            )?;
            Ok(None)
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct AddressParts<'a> {
    pub street: &'a str,
    pub line2: &'a str,
    pub city: &'a str,
    pub state: &'a str,
    pub zip: &'a str,
    pub plus4: &'a str,
}

pub fn postal_address(parts: AddressParts<'_>) -> Result<Option<PostalAddress>, FieldFailure> {
    let line1 = if parts.street.trim().is_empty() {
        None
    } else {
        Some(field("street", StreetLine::parse(parts.street))?)
    };
    let line2 = if parts.line2.trim().is_empty() {
        None
    } else {
        Some(field("street line 2", StreetLine::parse(parts.line2))?)
    };
    let city = if parts.city.trim().is_empty() {
        None
    } else {
        Some(field("city", CityName::parse(parts.city))?)
    };
    let zip = if parts.zip.trim().is_empty() {
        None
    } else if parts.plus4.trim().is_empty() {
        Some(field("zip", ZipCode::parse(parts.zip))?)
    } else {
        Some(field("zip", ZipCode::of(parts.zip, Some(parts.plus4)))?)
    };
    Ok(PostalAddress::of(
        line1,
        line2,
        city,
        census_state(parts.state),
        zip,
    ))
}

pub fn census_state(raw: &str) -> Option<UsJurisdiction> {
    UsJurisdiction::parse(raw).filter(|state| state.is_in_census_scope())
}

pub fn grade_span(low: &str, high: &str) -> Result<Option<GradeSpan>, FieldFailure> {
    let low = low.trim();
    let high = high.trim();
    if absent_grade(low) || absent_grade(high) {
        return Ok(None);
    }
    let low = field("grades", Grade::parse(low))?;
    let high = field("grades", Grade::parse(high))?;
    match (low, high) {
        (Some(low), Some(high)) => field("grades", GradeSpan::new(low, high)).map(Some),
        (low, high) => Err(FieldFailure {
            field: "grades",
            error: DirectoryError::UnsupportedValue {
                field: "grades",
                value: format!("{}..{}", label(low), label(high)),
            },
        }),
    }
}

pub fn coordinates(latitude: &str, longitude: &str) -> Result<Option<Coordinates>, FieldFailure> {
    if latitude.trim().is_empty() && longitude.trim().is_empty() {
        return Ok(None);
    }
    field("coordinates", Coordinates::parse(latitude, longitude)).map(Some)
}

fn absent_grade(value: &str) -> bool {
    value.is_empty() || value.eq_ignore_ascii_case("n") || value.eq_ignore_ascii_case("m")
}

fn label(grade: Option<Grade>) -> String {
    match grade {
        Some(grade) => grade.label(),
        None => "absent".to_string(),
    }
}

#[path = "directory/artifact.rs"]
mod artifact;

#[path = "directory/pattern.rs"]
mod pattern;

pub use artifact::{cell, first, read_rows, refusal, CsvRow, Header};

pub use pattern::{compile_pattern, group, line_of};

#[cfg(test)]
#[path = "directory/tests.rs"]
mod tests;
