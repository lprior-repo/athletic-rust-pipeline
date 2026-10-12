use super::engine_valid::{valid_date, valid_detail, valid_hex64, valid_id, valid_url};
use super::*;

pub const COACH_LANE_ID_PREFIX: &str = "coach_lane";
pub const COACH_LANE_MAX_ARM_BYTES: usize = 128;
pub const COACH_LANE_MAX_FRESHNESS_DAYS: u32 = 400;

fn coach_lane_id(jurisdiction: UsJurisdiction, season: SchoolYear, revision: u32) -> String {
    format!(
        "{COACH_LANE_ID_PREFIX}:{}:{}:{revision}",
        jurisdiction.code(),
        season.get()
    )
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneStatus {
    Pending,
    Armed,
    AcceptedRefusal,
    Blocked,
    Failed,
    Exhausted,
    SuccessfulEmpty,
    CompleteWithGaps,
}

impl CoachLaneStatus {
    pub const fn slug(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Armed => "armed",
            Self::AcceptedRefusal => "accepted_refusal",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Exhausted => "exhausted",
            Self::SuccessfulEmpty => "successful_empty",
            Self::CompleteWithGaps => "complete_with_gaps",
        }
    }

    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Armed | Self::AcceptedRefusal)
    }

    pub const fn from_stage(stage: CoachLaneStage) -> Self {
        match stage {
            CoachLaneStage::Pending => Self::Pending,
            CoachLaneStage::SuccessfulEmpty => Self::SuccessfulEmpty,
            CoachLaneStage::CompleteWithGaps => Self::CompleteWithGaps,
            CoachLaneStage::Blocked => Self::Blocked,
            CoachLaneStage::Failed => Self::Failed,
            CoachLaneStage::Exhausted => Self::Exhausted,
        }
    }
}

impl fmt::Display for CoachLaneStatus {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.slug())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneStage {
    Pending,
    SuccessfulEmpty,
    CompleteWithGaps,
    Blocked,
    Failed,
    Exhausted,
}

impl CoachLaneStage {
    pub const fn status(self) -> CoachLaneStatus {
        CoachLaneStatus::from_stage(self)
    }

    pub const fn slug(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::SuccessfulEmpty => "successful_empty",
            Self::CompleteWithGaps => "complete_with_gaps",
            Self::Blocked => "blocked",
            Self::Failed => "failed",
            Self::Exhausted => "exhausted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneRole {
    Published,
    Derived,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneTenure {
    Current,
    Former,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoachLaneMailbox {
    Published,
    Unpublished,
    DirectoryOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneResolution {
    pub role: CoachLaneRole,
    pub tenure: CoachLaneTenure,
    pub mailbox: CoachLaneMailbox,
    pub retrieved_on: String,
    #[serde(default)]
    pub freshness_days: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evidence: Option<String>,
}

impl CoachLaneResolution {
    pub fn unresolved(observed_on: impl Into<String>) -> Self {
        Self {
            role: CoachLaneRole::Derived,
            tenure: CoachLaneTenure::Unknown,
            mailbox: CoachLaneMailbox::Unpublished,
            retrieved_on: observed_on.into(),
            freshness_days: 0,
            evidence: None,
        }
    }

    pub fn resolved(
        role: CoachLaneRole,
        tenure: CoachLaneTenure,
        mailbox: CoachLaneMailbox,
        observed_on: impl Into<String>,
        freshness_days: u32,
        evidence: impl Into<String>,
    ) -> Result<Self, CoachLaneError> {
        let resolution = Self {
            role,
            tenure,
            mailbox,
            retrieved_on: observed_on.into(),
            freshness_days,
            evidence: Some(evidence.into()),
        };
        resolution.validate()?;
        Ok(resolution)
    }

    pub fn validate(&self) -> Result<(), CoachLaneError> {
        if !valid_date(&self.retrieved_on) {
            return Err(CoachLaneError::OutOfRange {
                field: "resolution.retrieved_on",
            });
        }
        if self.freshness_days > COACH_LANE_MAX_FRESHNESS_DAYS {
            return Err(CoachLaneError::OutOfRange {
                field: "resolution.freshness_days",
            });
        }
        if let Some(evidence) = self.evidence.as_deref() {
            if !valid_detail(evidence) {
                return Err(CoachLaneError::OutOfRange {
                    field: "resolution.evidence",
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneRefusal {
    source: SourceRef,
    reason: String,
    observed_on: String,
}

impl CoachLaneRefusal {
    pub fn new(
        source: SourceRef,
        reason: impl Into<String>,
        observed_on: impl Into<String>,
    ) -> Result<Self, CoachLaneError> {
        let refusal = Self {
            source,
            reason: reason.into(),
            observed_on: observed_on.into(),
        };
        refusal.validate()?;
        Ok(refusal)
    }

    pub fn source(&self) -> &SourceRef {
        &self.source
    }

    pub fn reason(&self) -> &str {
        self.reason.as_str()
    }

    pub fn observed_on(&self) -> &str {
        self.observed_on.as_str()
    }

    pub fn validate(&self) -> Result<(), CoachLaneError> {
        if !valid_id(&self.source.id) {
            return Err(CoachLaneError::OutOfRange {
                field: "refusal.source",
            });
        }
        if let Some(url) = self.source.url.as_deref() {
            if !valid_url(url) {
                return Err(CoachLaneError::OutOfRange {
                    field: "refusal.source.url",
                });
            }
        }
        if !valid_detail(&self.reason) {
            return Err(CoachLaneError::OutOfRange {
                field: "refusal.reason",
            });
        }
        if !valid_date(&self.observed_on) {
            return Err(CoachLaneError::OutOfRange {
                field: "refusal.observed_on",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneAcceptance {
    actor: String,
    event: String,
    observed_on: String,
}

impl CoachLaneAcceptance {
    pub fn new(
        actor: impl Into<String>,
        event: impl Into<String>,
        observed_on: impl Into<String>,
    ) -> Result<Self, CoachLaneError> {
        let acceptance = Self {
            actor: actor.into(),
            event: event.into(),
            observed_on: observed_on.into(),
        };
        acceptance.validate()?;
        Ok(acceptance)
    }

    pub fn actor(&self) -> &str {
        self.actor.as_str()
    }

    pub fn event(&self) -> &str {
        self.event.as_str()
    }

    pub fn observed_on(&self) -> &str {
        self.observed_on.as_str()
    }

    pub fn validate(&self) -> Result<(), CoachLaneError> {
        if !valid_id(&self.actor) {
            return Err(CoachLaneError::OutOfRange {
                field: "acceptance.actor",
            });
        }
        if !valid_id(&self.event) {
            return Err(CoachLaneError::OutOfRange {
                field: "acceptance.event",
            });
        }
        if !valid_date(&self.observed_on) {
            return Err(CoachLaneError::OutOfRange {
                field: "acceptance.observed_on",
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoachLaneEvidence {
    schools: u32,
    contacts: u32,
    digest: String,
    observed_on: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_url: Option<String>,
}

impl CoachLaneEvidence {
    pub fn new(
        schools: u32,
        contacts: u32,
        digest: impl Into<String>,
        observed_on: impl Into<String>,
    ) -> Result<Self, CoachLaneError> {
        let evidence = Self {
            schools,
            contacts,
            digest: digest.into(),
            observed_on: observed_on.into(),
            source_url: None,
        };
        evidence.validate()?;
        Ok(evidence)
    }

    pub fn with_source_url(mut self, url: impl Into<String>) -> Result<Self, CoachLaneError> {
        self.source_url = Some(url.into());
        self.validate()?;
        Ok(self)
    }

    pub fn schools(&self) -> u32 {
        self.schools
    }

    pub fn contacts(&self) -> u32 {
        self.contacts
    }

    pub fn digest(&self) -> &str {
        self.digest.as_str()
    }

    pub fn observed_on(&self) -> &str {
        self.observed_on.as_str()
    }

    pub fn source_url(&self) -> Option<&str> {
        self.source_url.as_deref()
    }

    pub fn validate(&self) -> Result<(), CoachLaneError> {
        if self.schools == 0 {
            return Err(CoachLaneError::OutOfRange {
                field: "evidence.schools",
            });
        }
        if self.contacts == 0 {
            return Err(CoachLaneError::OutOfRange {
                field: "evidence.contacts",
            });
        }
        if !valid_hex64(&self.digest) {
            return Err(CoachLaneError::OutOfRange {
                field: "evidence.digest",
            });
        }
        if !valid_date(&self.observed_on) {
            return Err(CoachLaneError::OutOfRange {
                field: "evidence.observed_on",
            });
        }
        if let Some(url) = self.source_url.as_deref() {
            if !valid_url(url) {
                return Err(CoachLaneError::OutOfRange {
                    field: "evidence.source_url",
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingCoachLane {
    pub jurisdiction: UsJurisdiction,
    pub season: SchoolYear,
    pub revision: u32,
    pub arm: String,
    pub arm_basis: String,
    pub observed_on: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CoachLaneObligation {
    id: String,
    jurisdiction: UsJurisdiction,
    season: SchoolYear,
    revision: u32,
    arm: String,
    arm_basis: String,
    status: CoachLaneStatus,
    refusal: Option<CoachLaneRefusal>,
    acceptance: Option<CoachLaneAcceptance>,
    evidence: Option<CoachLaneEvidence>,
    resolution: CoachLaneResolution,
    updated_on: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoachLaneMatrix {
    rows: Vec<CoachLaneObligation>,
}

impl CoachLaneMatrix {
    pub fn new(rows: Vec<CoachLaneObligation>) -> Self {
        Self { rows }
    }

    pub fn rows(&self) -> &[CoachLaneObligation] {
        self.rows.as_slice()
    }

    pub fn verify(&self) -> Result<(), CoachLaneError> {
        for &jurisdiction in UsJurisdiction::CENSUS_SCOPE.iter() {
            let mut found: Option<&CoachLaneObligation> = None;
            let mut duplicated = false;
            for row in self
                .rows
                .iter()
                .filter(|row| row.jurisdiction() == jurisdiction)
            {
                if found.is_some() {
                    duplicated = true;
                }
                found = Some(row);
            }
            let Some(row) = found else {
                return Err(CoachLaneError::MissingJurisdiction { jurisdiction });
            };
            if duplicated {
                return Err(CoachLaneError::DuplicateJurisdiction { jurisdiction });
            }
            row.validate()?;
            if !row.is_terminal() {
                return Err(CoachLaneError::LaneNotClosed {
                    jurisdiction,
                    status: row.status(),
                });
            }
        }
        self.rows.iter().try_for_each(|row| {
            if row.jurisdiction().is_in_census_scope() {
                Ok(())
            } else {
                Err(CoachLaneError::OutsideCensusScope {
                    jurisdiction: row.jurisdiction(),
                })
            }
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoachLaneError {
    #[error("{field} is outside its permitted range")]
    OutOfRange { field: &'static str },
    #[error("{field} is not permitted in this state")]
    Unsupported { field: &'static str },
    #[error("an armed coach lane requires per-school output evidence")]
    MissingEvidence,
    #[error("an accepted refusal requires a recorded refusal first")]
    MissingRefusal,
    #[error("a recorded refusal cannot close a coach lane without an explicit acceptance event")]
    UnacceptedRefusal,
    #[error("jurisdiction {jurisdiction} has no coach-lane row")]
    MissingJurisdiction { jurisdiction: UsJurisdiction },
    #[error("jurisdiction {jurisdiction} has more than one coach-lane row")]
    DuplicateJurisdiction { jurisdiction: UsJurisdiction },
    #[error("jurisdiction {jurisdiction} is outside the census scope")]
    OutsideCensusScope { jurisdiction: UsJurisdiction },
    #[error("the coach lane for {jurisdiction} is not closed: {status}")]
    LaneNotClosed {
        jurisdiction: UsJurisdiction,
        status: CoachLaneStatus,
    },
}

#[cfg(test)]
#[path = "coach_lane_tests.rs"]
mod tests;
