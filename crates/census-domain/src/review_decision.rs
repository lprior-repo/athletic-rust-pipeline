#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Admitted {
    pub field: String,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HardContradiction {
    GradYearEvidenceDiffers,
    GenderDiffers,
    RetainedSourceConflict,
    DistinctProviderObjects,
}

impl HardContradiction {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::GradYearEvidenceDiffers => "grad_year_evidence_differs",
            Self::GenderDiffers => "gender_differs",
            Self::RetainedSourceConflict => "retained_source_conflict",
            Self::DistinctProviderObjects => "distinct_provider_objects",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    WrongField,
    InvalidValue,
    AlreadyResolved,
    UnnamedCase,
    HardContradiction(HardContradiction),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Adjudication {
    Decided(Admitted),
    Undecided,
    Refused(Refusal),
}

impl Adjudication {
    pub const fn admitted(&self) -> Option<&Admitted> {
        match self {
            Self::Decided(admitted) => Some(admitted),
            Self::Undecided | Self::Refused(_) => None,
        }
    }
}
impl Refusal {
    pub const fn rationale(self) -> Option<&'static str> {
        match self {
            Self::HardContradiction(flag) => Some(flag.slug()),
            Self::WrongField | Self::InvalidValue | Self::AlreadyResolved | Self::UnnamedCase => {
                None
            }
        }
    }
}
