#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DirectoryError {
    #[error("{field} is empty")]
    EmptyField { field: &'static str },
    #[error("{field} is {length} characters, over the {limit}-character limit")]
    FieldTooLong {
        field: &'static str,
        limit: usize,
        length: usize,
    },
    #[error("{field} contains a control character")]
    ControlCharacter { field: &'static str },
    #[error("{value:?} is not a five-digit ZIP code")]
    MalformedZip { value: String },
    #[error("{value:?} is not a supported grade")]
    UnsupportedGrade { value: String },
    #[error("grade span {low}..{high} is inverted")]
    GradeSpanInverted { low: String, high: String },
    #[error("{field} {value} is outside its range")]
    CoordinateOutOfRange { field: &'static str, value: i64 },
    #[error("{value:?} is not a decimal coordinate")]
    MalformedCoordinate { value: String },
    #[error("{value} is not a month number")]
    MalformedMonth { value: u8 },
    #[error("{value} is not a four-digit year")]
    MalformedYear { value: i32 },
    #[error("{value:?} is not a calendar month")]
    MalformedYearMonth { value: String },
    #[error("{field} is not an integer: {value:?}")]
    MalformedInteger { field: &'static str, value: String },
    #[error("{field} is not a supported value: {value:?}")]
    UnsupportedValue { field: &'static str, value: String },
    #[error("entry has no recorded sources")]
    MissingEntrySources,
    #[error("field {field} is missing provenance")]
    MissingFieldProvenance { field: &'static str },
    #[error("absent field {field} has provenance")]
    UnexpectedFieldProvenance { field: &'static str },
    #[error("field {field} provenance names an unrecorded source")]
    UnrecordedFieldSource { field: &'static str },
    #[error("{resource} requests {requested}, over the {limit} limit")]
    Capacity {
        resource: &'static str,
        requested: usize,
        limit: usize,
    },
    #[error("allocating {resource} failed")]
    Allocation { resource: &'static str },
    #[error("directory representation failed: {detail}")]
    Representation { detail: String },
}

impl DirectoryError {
    pub fn is_resource(&self) -> bool {
        match self {
            Self::Capacity { .. } | Self::Allocation { .. } | Self::Representation { .. } => true,
            Self::EmptyField { .. }
            | Self::FieldTooLong { .. }
            | Self::ControlCharacter { .. }
            | Self::MalformedZip { .. }
            | Self::UnsupportedGrade { .. }
            | Self::GradeSpanInverted { .. }
            | Self::CoordinateOutOfRange { .. }
            | Self::MalformedCoordinate { .. }
            | Self::MalformedMonth { .. }
            | Self::MalformedYear { .. }
            | Self::MalformedYearMonth { .. }
            | Self::MalformedInteger { .. }
            | Self::UnsupportedValue { .. }
            | Self::MissingEntrySources
            | Self::MissingFieldProvenance { .. }
            | Self::UnexpectedFieldProvenance { .. }
            | Self::UnrecordedFieldSource { .. } => false,
        }
    }
}
