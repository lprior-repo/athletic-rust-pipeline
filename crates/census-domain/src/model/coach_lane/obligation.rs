use super::*;
use crate::model::engine_valid::{valid_date, valid_detail, valid_id};
use serde::{Deserialize, Deserializer};

fn coach_lane_id(jurisdiction: UsJurisdiction, season: SchoolYear, revision: u32) -> String {
    format!(
        "{COACH_LANE_ID_PREFIX}:{}:{}:{revision}",
        jurisdiction.code(),
        season.get()
    )
}

impl CoachLaneObligation {
    pub fn pending(request: PendingCoachLane) -> Result<Self, CoachLaneError> {
        let obligation = Self {
            id: coach_lane_id(request.jurisdiction, request.season, request.revision),
            jurisdiction: request.jurisdiction,
            season: request.season,
            revision: request.revision,
            arm: request.arm,
            arm_basis: request.arm_basis,
            status: CoachLaneStatus::Pending,
            refusal: None,
            acceptance: None,
            evidence: None,
            resolution: CoachLaneResolution::unresolved(request.observed_on.clone()),
            updated_on: request.observed_on,
        };
        obligation.validate()?;
        Ok(obligation)
    }

    pub fn arm(
        self,
        evidence: CoachLaneEvidence,
        resolution: CoachLaneResolution,
    ) -> Result<Self, CoachLaneError> {
        if self.is_terminal() {
            return Err(CoachLaneError::Unsupported { field: "status" });
        }
        let updated_on = if evidence.observed_on() >= self.updated_on.as_str() {
            evidence.observed_on().to_string()
        } else {
            self.updated_on.clone()
        };
        let candidate = Self {
            status: CoachLaneStatus::Armed,
            refusal: None,
            acceptance: None,
            evidence: Some(evidence),
            resolution,
            updated_on,
            ..self
        };
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn record(
        self,
        stage: CoachLaneStage,
        refusal: Option<CoachLaneRefusal>,
        resolution: CoachLaneResolution,
    ) -> Result<Self, CoachLaneError> {
        if self.is_terminal() {
            return Err(CoachLaneError::Unsupported { field: "status" });
        }
        let refusal = match stage {
            CoachLaneStage::Pending => None,
            _ => refusal,
        };
        let updated_on = match refusal.as_ref() {
            Some(recorded) if recorded.observed_on() >= self.updated_on.as_str() => {
                recorded.observed_on().to_string()
            }
            _ => self.updated_on.clone(),
        };
        let candidate = Self {
            status: stage.status(),
            refusal,
            resolution,
            updated_on,
            ..self
        };
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn accept_refusal(self, acceptance: CoachLaneAcceptance) -> Result<Self, CoachLaneError> {
        if self.refusal.is_none() {
            return Err(CoachLaneError::MissingRefusal);
        }
        if self.is_terminal() {
            return Err(CoachLaneError::Unsupported { field: "status" });
        }
        let updated_on = if acceptance.observed_on() >= self.updated_on.as_str() {
            acceptance.observed_on().to_string()
        } else {
            self.updated_on.clone()
        };
        let candidate = Self {
            status: CoachLaneStatus::AcceptedRefusal,
            acceptance: Some(acceptance),
            evidence: None,
            updated_on,
            ..self
        };
        candidate.validate()?;
        Ok(candidate)
    }

    pub fn id(&self) -> &str {
        self.id.as_str()
    }

    pub fn jurisdiction(&self) -> UsJurisdiction {
        self.jurisdiction
    }

    pub fn season(&self) -> SchoolYear {
        self.season
    }

    pub fn revision(&self) -> u32 {
        self.revision
    }

    pub fn arm_name(&self) -> &str {
        self.arm.as_str()
    }

    pub fn arm_basis(&self) -> &str {
        self.arm_basis.as_str()
    }

    pub fn status(&self) -> CoachLaneStatus {
        self.status
    }

    pub fn refusal(&self) -> Option<&CoachLaneRefusal> {
        self.refusal.as_ref()
    }

    pub fn acceptance(&self) -> Option<&CoachLaneAcceptance> {
        self.acceptance.as_ref()
    }

    pub fn evidence(&self) -> Option<&CoachLaneEvidence> {
        self.evidence.as_ref()
    }

    pub fn resolution(&self) -> &CoachLaneResolution {
        &self.resolution
    }

    pub fn updated_on(&self) -> &str {
        self.updated_on.as_str()
    }

    pub fn is_terminal(&self) -> bool {
        self.status.is_terminal()
    }

    pub fn validate(&self) -> Result<(), CoachLaneError> {
        self.validate_basic_fields()?;
        self.validate_nested_objects()?;
        self.validate_status_consistency()?;
        Ok(())
    }

    fn validate_basic_fields(&self) -> Result<(), CoachLaneError> {
        if !valid_id(&self.id)
            || self.id != coach_lane_id(self.jurisdiction, self.season, self.revision)
        {
            return Err(CoachLaneError::OutOfRange { field: "id" });
        }
        if self.revision == 0 {
            return Err(CoachLaneError::OutOfRange { field: "revision" });
        }
        if !valid_id(&self.arm) || self.arm.len() > COACH_LANE_MAX_ARM_BYTES {
            return Err(CoachLaneError::OutOfRange { field: "arm" });
        }
        if !valid_detail(&self.arm_basis) {
            return Err(CoachLaneError::OutOfRange { field: "arm_basis" });
        }
        if !valid_date(&self.updated_on) {
            return Err(CoachLaneError::OutOfRange {
                field: "updated_on",
            });
        }
        Ok(())
    }

    fn validate_nested_objects(&self) -> Result<(), CoachLaneError> {
        self.resolution.validate()?;
        if let Some(refusal) = self.refusal.as_ref() {
            refusal.validate()?;
            self.refuse_out_of_order(refusal.observed_on(), "refusal.observed_on")?;
        }
        if let Some(acceptance) = self.acceptance.as_ref() {
            acceptance.validate()?;
            self.refuse_out_of_order(acceptance.observed_on(), "acceptance.observed_on")?;
        }
        if let Some(evidence) = self.evidence.as_ref() {
            evidence.validate()?;
            self.refuse_out_of_order(evidence.observed_on(), "evidence.observed_on")?;
        }
        Ok(())
    }

    fn validate_status_consistency(&self) -> Result<(), CoachLaneError> {
        match self.status {
            CoachLaneStatus::Armed => {
                if self.acceptance.is_some() || self.refusal.is_some() {
                    return Err(CoachLaneError::Unsupported { field: "status" });
                }
                if self.evidence.is_none() {
                    return Err(CoachLaneError::MissingEvidence);
                }
            }
            CoachLaneStatus::AcceptedRefusal => {
                if self.evidence.is_some() {
                    return Err(CoachLaneError::Unsupported { field: "evidence" });
                }
                if self.refusal.is_none() {
                    return Err(CoachLaneError::MissingRefusal);
                }
                if self.acceptance.is_none() {
                    return Err(CoachLaneError::UnacceptedRefusal);
                }
            }
            _ => {
                if self.acceptance.is_some() {
                    return Err(CoachLaneError::Unsupported {
                        field: "acceptance",
                    });
                }
                if self.evidence.is_some() {
                    return Err(CoachLaneError::Unsupported { field: "evidence" });
                }
            }
        }
        Ok(())
    }

    fn refuse_out_of_order(&self, date: &str, field: &'static str) -> Result<(), CoachLaneError> {
        if date <= self.updated_on.as_str() {
            Ok(())
        } else {
            Err(CoachLaneError::OutOfRange { field })
        }
    }
}

impl<'de> Deserialize<'de> for CoachLaneObligation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct RawCoachLaneObligation {
            id: String,
            jurisdiction: UsJurisdiction,
            season: SchoolYear,
            revision: u32,
            arm: String,
            arm_basis: String,
            status: CoachLaneStatus,
            #[serde(default)]
            refusal: Option<CoachLaneRefusal>,
            #[serde(default)]
            acceptance: Option<CoachLaneAcceptance>,
            #[serde(default)]
            evidence: Option<CoachLaneEvidence>,
            resolution: CoachLaneResolution,
            updated_on: String,
        }

        let raw = RawCoachLaneObligation::deserialize(deserializer)?;
        let obligation = Self {
            id: raw.id,
            jurisdiction: raw.jurisdiction,
            season: raw.season,
            revision: raw.revision,
            arm: raw.arm,
            arm_basis: raw.arm_basis,
            status: raw.status,
            refusal: raw.refusal,
            acceptance: raw.acceptance,
            evidence: raw.evidence,
            resolution: raw.resolution,
            updated_on: raw.updated_on,
        };
        if let Err(error) = obligation.validate() {
            return Err(<D::Error as serde::de::Error>::custom(error));
        }
        Ok(obligation)
    }
}
