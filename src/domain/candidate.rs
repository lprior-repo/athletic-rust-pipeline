pub use super::name::{BioIdentityObservation, CanonicalName, HtmlIdentity, NameExclusion};
use super::{evidence::ProfileEvidence, identity::AthleteId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateCoverage {
    Complete,
    Incomplete,
    NameExcluded,
}

#[derive(Debug, Clone, Copy)]
pub enum CandidateEvidence<'a> {
    Complete(&'a ProfileEvidence),
    Incomplete {
        athlete_id: AthleteId,
        profile: Option<&'a ProfileEvidence>,
    },
    NameExcluded {
        profiles: &'a [ProfileEvidence],
        exclusion: &'a NameExclusion,
    },
}

impl<'a> CandidateEvidence<'a> {
    #[must_use]
    pub fn athlete_id(self) -> AthleteId {
        match self {
            Self::Complete(profile) => profile.athlete_id,
            Self::Incomplete { athlete_id, .. } => athlete_id,
            Self::NameExcluded { exclusion, .. } => exclusion.athlete_id(),
        }
    }

    #[must_use]
    pub fn coverage(self) -> CandidateCoverage {
        match self {
            Self::Complete(_) => CandidateCoverage::Complete,
            Self::Incomplete { .. } => CandidateCoverage::Incomplete,
            Self::NameExcluded { .. } => CandidateCoverage::NameExcluded,
        }
    }

    #[must_use]
    pub fn profiles(self) -> &'a [ProfileEvidence] {
        match self {
            Self::Complete(profile) => std::slice::from_ref(profile),
            Self::Incomplete { profile, .. } => profile.map_or(&[], std::slice::from_ref),
            Self::NameExcluded { profiles, .. } => profiles,
        }
    }

    #[must_use]
    pub fn exclusion(self) -> Option<&'a NameExclusion> {
        match self {
            Self::NameExcluded { exclusion, .. } => Some(exclusion),
            Self::Complete(_) | Self::Incomplete { .. } => None,
        }
    }
}
