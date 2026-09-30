use census_domain::school_directory::{
    CityName, Coordinates, DirectoryError, Grade, GradeSpan, PostalAddress, SchoolDirectoryEntry,
    StreetLine, ZipCode,
};
use census_domain::UsJurisdiction;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowIssue {
    pub line: usize,
    pub field: &'static str,
    pub detail: String,
}

impl RowIssue {
    pub fn new(line: usize, field: &'static str, detail: impl Into<String>) -> Self {
        Self {
            line,
            field,
            detail: detail.into(),
        }
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReadOutcome {
    entries: Vec<SchoolDirectoryEntry>,
    skipped: Vec<RowIssue>,
    notes: Vec<RowIssue>,
}

impl ReadOutcome {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, entry: SchoolDirectoryEntry) {
        self.entries.push(entry);
    }

    pub fn skip(&mut self, line: usize, field: &'static str, detail: impl Into<String>) {
        self.skipped.push(RowIssue::new(line, field, detail));
    }

    pub fn note(&mut self, line: usize, field: &'static str, detail: impl Into<String>) {
        self.notes.push(RowIssue::new(line, field, detail));
    }

    pub fn entries(&self) -> &[SchoolDirectoryEntry] {
        &self.entries
    }

    pub fn skipped(&self) -> &[RowIssue] {
        &self.skipped
    }

    pub fn notes(&self) -> &[RowIssue] {
        &self.notes
    }

    pub fn into_entries(self) -> Vec<SchoolDirectoryEntry> {
        self.entries
    }

    pub fn counts(&self) -> ReadCounts {
        ReadCounts {
            entries: self.entries.len(),
            skipped: self.skipped.len(),
            notes: self.notes.len(),
        }
    }

    pub fn absorb(&mut self, other: ReadOutcome) {
        self.entries.extend(other.entries);
        self.skipped.extend(other.skipped);
        self.notes.extend(other.notes);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFailure {
    pub field: &'static str,
    pub detail: String,
}

pub fn field<T>(name: &'static str, parsed: Result<T, DirectoryError>) -> Result<T, FieldFailure> {
    parsed.map_err(|error| FieldFailure {
        field: name,
        detail: error.to_string(),
    })
}

pub fn optional<T>(
    name: &'static str,
    parsed: Result<Option<T>, DirectoryError>,
) -> Result<Option<T>, FieldFailure> {
    parsed.map_err(|error| FieldFailure {
        field: name,
        detail: error.to_string(),
    })
}

pub fn skip_row<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    field: &'static str,
    parsed: Result<T, DirectoryError>,
) -> Option<T> {
    match parsed {
        Ok(value) => Some(value),
        Err(error) => {
            outcome.skip(line, field, error.to_string());
            None
        }
    }
}

pub fn skip_optional<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    field: &'static str,
    parsed: Result<Option<T>, DirectoryError>,
) -> Option<T> {
    match parsed {
        Ok(value) => value,
        Err(error) => {
            outcome.note(line, field, error.to_string());
            None
        }
    }
}

pub fn skip_failed<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    parsed: Result<T, FieldFailure>,
) -> Option<T> {
    match parsed {
        Ok(value) => Some(value),
        Err(failure) => {
            outcome.note(line, failure.field, failure.detail);
            None
        }
    }
}

pub fn skip_absent<T>(
    outcome: &mut ReadOutcome,
    line: usize,
    parsed: Result<Option<T>, FieldFailure>,
) -> Option<T> {
    match parsed {
        Ok(value) => value,
        Err(failure) => {
            outcome.note(line, failure.field, failure.detail);
            None
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
        (Some(low), Some(high)) => {
            GradeSpan::new(low, high)
                .map(Some)
                .map_err(|error| FieldFailure {
                    field: "grades",
                    detail: error.to_string(),
                })
        }
        (low, high) => Err(FieldFailure {
            field: "grades",
            detail: format!("{}..{} has no rankable grade", label(low), label(high)),
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

pub use artifact::{cell, first, read_rows, refusal, Header};

pub use pattern::{compile_pattern, group, line_of};

#[cfg(test)]
#[path = "directory/tests.rs"]
mod tests;
