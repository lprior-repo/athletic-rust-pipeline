use super::*;

impl CandidateDecisionReference {
    pub fn from_correction(
        application: &AcceptedAthleteIdentity,
        limits: ReplayLimits,
    ) -> Result<Self, HistoryError> {
        Self::correction_record(application.record(), limits)
    }

    pub fn matches_correction(
        &self,
        record: &AppliedAthleteIdentity,
        limits: ReplayLimits,
    ) -> Result<bool, HistoryError> {
        Ok(self == &Self::correction_record(record, limits)?)
    }

    fn correction_record(
        record: &AppliedAthleteIdentity,
        limits: ReplayLimits,
    ) -> Result<Self, HistoryError> {
        super::super::budget::accepted_reference(record, limits)?;
        if record.kind != AppliedIdentityKind::DifferentPerson {
            return Err(HistoryError::EvidenceMismatch);
        }
        Self::from_kind(record, CandidateEvidenceKind::Correction)
    }
}
