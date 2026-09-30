use crate::UsJurisdiction;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::BTreeSet;

use super::name::AssociationLabel;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub enum SourceLabel {
    Ccd,
    Pss,
    StateEducationAgency { state: UsJurisdiction },
    PrivateAssociation { label: AssociationLabel },
    AthleticAssociation { state: UsJurisdiction },
}

impl SourceLabel {
    pub const fn rank(&self) -> u8 {
        match self {
            Self::Ccd => 0,
            Self::Pss => 1,
            Self::StateEducationAgency { .. } => 2,
            Self::PrivateAssociation { .. } => 3,
            Self::AthleticAssociation { .. } => 4,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Ccd => "nces-ccd".to_string(),
            Self::Pss => "nces-pss".to_string(),
            Self::StateEducationAgency { state } => format!("state-ed:{}", state.code()),
            Self::PrivateAssociation { label } => format!("association:{}", label.as_str()),
            Self::AthleticAssociation { state } => {
                format!("athletic-association:{}", state.code())
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Priority {
    rank: u8,
}

impl Priority {
    const fn new(rank: u8) -> Self {
        Self { rank }
    }

    pub(crate) fn strongest(sources: &BTreeSet<SourceLabel>) -> Self {
        let rank = sources
            .iter()
            .map(SourceLabel::rank)
            .min()
            .unwrap_or(u8::MAX);
        Self::new(rank)
    }
}

impl Ord for Priority {
    fn cmp(&self, other: &Self) -> Ordering {
        other.rank.cmp(&self.rank)
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
