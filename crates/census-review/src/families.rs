//! The families a review pass asks about, and how a pass is run.
//!
//! A family is one retained finding's unresolved field. The set is deliberately small: a family is
//! askable only when a local model can answer it from the row's own text and the answer is checkable
//! without the model — see [`validate`](super::validate).

use census_domain::model::{
    ATHLETE_IDENTITY_FAMILY, UNRESOLVED_SCHOOL_FAMILY, UNRESOLVED_VENUE_FAMILY,
};

/// The field an athlete-identity proposal answers: whether the two rows a case compares are one
/// athlete.
///
/// The field name is the family's contract with the model, so it is stated once and read by the
/// packet that asks for it and the reader that admits it.
pub const IDENTITY_FIELD: &str = "identity";

/// The families a pass may ask about, and the field each leaves unresolved.
///
/// Two of them are the same question — *which jurisdiction does this row belong to* — asked about
/// different kinds of subject. The meet family's label is the workbook's ("Meet venue unresolved"),
/// while the detail the store retains for it is "no evidence placed the venue in a jurisdiction;
/// filed under ??", so the field a proposal must answer is `state` for both.
///
/// The third is a different question with a different kind of answer: two canonical athlete rows the
/// merge kept apart under one key may or may not be the same person, and the answer is a decision
/// ([`AthleteVerdict`](super::AthleteVerdict)) rather than a value, so it is read by its own rule and
/// never by the jurisdiction rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReviewFamily {
    /// A school no source placed in a jurisdiction.
    SchoolJurisdiction,
    /// A meet no source placed in a jurisdiction.
    MeetJurisdiction,
    /// Two canonical athlete rows the merge kept apart under one key.
    AthleteIdentity,
}

impl ReviewFamily {
    /// The label the workbook and the store use for this family.
    pub const fn label(self) -> &'static str {
        match self {
            Self::SchoolJurisdiction => UNRESOLVED_SCHOOL_FAMILY,
            Self::MeetJurisdiction => UNRESOLVED_VENUE_FAMILY,
            Self::AthleteIdentity => ATHLETE_IDENTITY_FAMILY,
        }
    }

    /// The field a proposal for this family must answer.
    pub const fn field(self) -> &'static str {
        match self {
            Self::SchoolJurisdiction | Self::MeetJurisdiction => "state",
            Self::AthleteIdentity => IDENTITY_FIELD,
        }
    }

    /// The family a retained case's label names, if this lane asks about it.
    ///
    /// The other retained families are deliberately not asked about: a cohort claim needs evidence
    /// this store does not hold, and a withheld mailbox is withheld on purpose.
    pub fn from_label(label: &str) -> Option<Self> {
        if label == UNRESOLVED_SCHOOL_FAMILY {
            Some(Self::SchoolJurisdiction)
        } else if label == UNRESOLVED_VENUE_FAMILY {
            Some(Self::MeetJurisdiction)
        } else if label == ATHLETE_IDENTITY_FAMILY {
            Some(Self::AthleteIdentity)
        } else {
            None
        }
    }

    /// The families a pass asks about by default, in the order it asks.
    pub const fn askable() -> [Self; 3] {
        [
            Self::SchoolJurisdiction,
            Self::MeetJurisdiction,
            Self::AthleteIdentity,
        ]
    }

    /// Parse a family from the CLI's spelling: `school-jurisdiction`, `meet_venue`, `athlete-identity`.
    pub fn parse(value: &str) -> Option<Self> {
        let normalized = value
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_'], " ")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        match normalized.as_str() {
            "school jurisdiction unresolved" | "school jurisdiction" | "school" => {
                Some(Self::SchoolJurisdiction)
            }
            "meet venue unresolved" | "meet venue" | "venue" | "meet jurisdiction" | "meet" => {
                Some(Self::MeetJurisdiction)
            }
            "athlete identity" | "athlete" | "identity" => Some(Self::AthleteIdentity),
            _ => None,
        }
    }
}

/// How to run one pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOptions {
    /// Which families to ask about.
    pub families: Vec<ReviewFamily>,
    /// Ask about at most this many cases.
    pub limit: usize,
    /// Ask and validate, but write nothing.
    pub dry_run: bool,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            families: ReviewFamily::askable().to_vec(),
            limit: 25,
            dry_run: false,
        }
    }
}

#[cfg(test)]
#[path = "families_tests.rs"]
mod tests;
