use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::schedule::{Cadence, Month, Parity, YearMonth};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ScheduleSource {
    Ccd,
    Pss,
    StateEducationAgency,
    PrivateAssociation,
    AthleticAssociation,
}

impl ScheduleSource {
    pub const ALL: [ScheduleSource; 5] = [
        ScheduleSource::Ccd,
        ScheduleSource::Pss,
        ScheduleSource::StateEducationAgency,
        ScheduleSource::PrivateAssociation,
        ScheduleSource::AthleticAssociation,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Ccd => "nces-ccd",
            Self::Pss => "nces-pss",
            Self::StateEducationAgency => "state-ed",
            Self::PrivateAssociation => "association",
            Self::AthleticAssociation => "athletic-association",
        }
    }

    pub fn cadence(self) -> Option<Cadence> {
        match self {
            Self::Ccd => Some(Cadence::Yearly {
                month: Month::SEPTEMBER,
            }),
            Self::Pss => Some(Cadence::Biennial {
                parity: Parity::Even,
            }),
            Self::StateEducationAgency => Some(Cadence::Semester {
                months: [Month::JANUARY, Month::JULY],
            }),
            Self::PrivateAssociation => Some(Cadence::Quarterly {
                months: [Month::JANUARY, Month::APRIL, Month::JULY, Month::OCTOBER],
            }),
            Self::AthleticAssociation => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ScheduleLedger {
    last: BTreeMap<ScheduleSource, YearMonth>,
}

impl ScheduleLedger {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn last(&self, source: ScheduleSource) -> Option<YearMonth> {
        self.last.get(&source).copied()
    }

    pub fn record(&mut self, source: ScheduleSource, at: YearMonth) {
        self.last.insert(source, at);
    }

    pub fn entries(&self) -> Vec<(ScheduleSource, YearMonth)> {
        self.last
            .iter()
            .map(|(source, at)| (*source, *at))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.last.len()
    }

    pub fn is_empty(&self) -> bool {
        self.last.is_empty()
    }
}
