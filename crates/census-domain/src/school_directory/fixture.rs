use super::{
    CityName, DirectoryError, Grade, GradeSpan, IdentifiedKey, NcesSchoolId, NumberedGrade,
    PostalAddress, SchoolDirectoryEntry, SchoolName, SourceLabel, StateRecordId, StreetLine,
    Website, ZipCode,
};
use crate::UsJurisdiction;

pub fn grade_span(low: u8, high: u8) -> Result<GradeSpan, DirectoryError> {
    GradeSpan::new(
        Grade::Numbered(NumberedGrade::new(low)?),
        Grade::Numbered(NumberedGrade::new(high)?),
    )
}

pub fn postal_address(
    line1: Option<&str>,
    city: &str,
    state: UsJurisdiction,
) -> Result<PostalAddress, DirectoryError> {
    let line1 = match line1 {
        Some(line1) => Some(StreetLine::parse(line1)?),
        None => None,
    };
    PostalAddress::of(
        line1,
        None,
        Some(CityName::parse(city)?),
        Some(state),
        Some(ZipCode::parse("43000")?),
    )
    .ok_or(DirectoryError::EmptyField { field: "address" })
}

pub fn nces_entry(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: &str,
    grades: (u8, u8),
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    nces_entry_with_website(id, name, city, state, line1, "", grades)
}

pub fn nces_entry_with_website(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: &str,
    website: &str,
    grades: (u8, u8),
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::Nces(NcesSchoolId::parse(id)?),
        SourceLabel::Ccd,
        Some(SchoolName::parse(name)?),
    )
    .with_address(Some(postal_address(Some(line1), city, state)?))
    .with_website(Website::parse(website)?)
    .with_grades(Some(grade_span(grades.0, grades.1)?)))
}

pub fn state_record_entry(
    id: &str,
    name: &str,
    city: &str,
    state: UsJurisdiction,
    line1: Option<&str>,
    label: SourceLabel,
) -> Result<SchoolDirectoryEntry, DirectoryError> {
    Ok(SchoolDirectoryEntry::identified(
        IdentifiedKey::StateRecord {
            state,
            id: StateRecordId::parse(id)?,
        },
        label,
        Some(SchoolName::parse(name)?),
    )
    .with_address(Some(postal_address(line1, city, state)?))
    .with_grades(Some(grade_span(9, 12)?)))
}
