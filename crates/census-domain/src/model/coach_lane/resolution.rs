use super::*;
use crate::model::engine_valid::{valid_date, valid_detail, valid_hex64, valid_id, valid_url};
use std::fmt;

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
