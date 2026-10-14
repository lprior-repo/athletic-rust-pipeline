#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParticipationReason {
    MissingPerformanceDate,
    UnsupportedDatePrecision,
    InvalidPerformanceDate,
    AfterEffectiveCutoff,
    BeforePerformanceWindow,
    AfterPerformanceWindow,
    ResultOwnershipUnresolved,
    ResultOwnershipConflict,
    CohortUnresolved,
    CohortConflict,
    CohortOutsideSelection,
    ParticipationContextUnresolved,
    ParticipationEvidenceConflict,
    HighSchoolStatusMissing,
    StatusNotEstablishedOnPerformanceDate,
    SummerBoundaryUnresolved,
    OutsideParticipationWindow,
    CollegiateParticipation,
    MiddleSchoolParticipation,
    PostGraduationParticipation,
    ParticipationWindowConflict,
}

impl ParticipationReason {
    pub const ALL: [Self; 21] = [
        Self::MissingPerformanceDate,
        Self::UnsupportedDatePrecision,
        Self::InvalidPerformanceDate,
        Self::AfterEffectiveCutoff,
        Self::BeforePerformanceWindow,
        Self::AfterPerformanceWindow,
        Self::ResultOwnershipUnresolved,
        Self::ResultOwnershipConflict,
        Self::CohortUnresolved,
        Self::CohortConflict,
        Self::CohortOutsideSelection,
        Self::ParticipationContextUnresolved,
        Self::ParticipationEvidenceConflict,
        Self::HighSchoolStatusMissing,
        Self::StatusNotEstablishedOnPerformanceDate,
        Self::SummerBoundaryUnresolved,
        Self::OutsideParticipationWindow,
        Self::CollegiateParticipation,
        Self::MiddleSchoolParticipation,
        Self::PostGraduationParticipation,
        Self::ParticipationWindowConflict,
    ];

    const fn flag(self) -> u32 {
        match self {
            Self::MissingPerformanceDate => 1,
            Self::UnsupportedDatePrecision => 1 << 1,
            Self::InvalidPerformanceDate => 1 << 2,
            Self::AfterEffectiveCutoff => 1 << 3,
            Self::BeforePerformanceWindow => 1 << 4,
            Self::AfterPerformanceWindow => 1 << 5,
            Self::ResultOwnershipUnresolved => 1 << 6,
            Self::ResultOwnershipConflict => 1 << 7,
            Self::CohortUnresolved => 1 << 8,
            Self::CohortConflict => 1 << 9,
            Self::CohortOutsideSelection => 1 << 10,
            Self::ParticipationContextUnresolved => 1 << 11,
            Self::ParticipationEvidenceConflict => 1 << 12,
            Self::HighSchoolStatusMissing => 1 << 13,
            Self::StatusNotEstablishedOnPerformanceDate => 1 << 14,
            Self::SummerBoundaryUnresolved => 1 << 15,
            Self::OutsideParticipationWindow => 1 << 16,
            Self::CollegiateParticipation => 1 << 17,
            Self::MiddleSchoolParticipation => 1 << 18,
            Self::PostGraduationParticipation => 1 << 19,
            Self::ParticipationWindowConflict => 1 << 20,
        }
    }

    pub(super) const fn definite(self) -> bool {
        matches!(
            self,
            Self::AfterEffectiveCutoff
                | Self::BeforePerformanceWindow
                | Self::AfterPerformanceWindow
                | Self::CohortOutsideSelection
                | Self::OutsideParticipationWindow
                | Self::CollegiateParticipation
                | Self::MiddleSchoolParticipation
                | Self::PostGraduationParticipation
        )
    }

    pub(super) const fn conflict(self) -> bool {
        matches!(
            self,
            Self::ResultOwnershipConflict
                | Self::CohortConflict
                | Self::ParticipationEvidenceConflict
                | Self::ParticipationWindowConflict
        )
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParticipationReasons(u32);

impl ParticipationReasons {
    pub fn iter(self) -> impl Iterator<Item = ParticipationReason> {
        ParticipationReason::ALL
            .into_iter()
            .filter(move |reason| self.contains(*reason))
    }

    pub const fn contains(self, reason: ParticipationReason) -> bool {
        self.0 & reason.flag() != 0
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub(super) fn add(&mut self, reason: ParticipationReason) {
        self.0 |= reason.flag();
    }
}
