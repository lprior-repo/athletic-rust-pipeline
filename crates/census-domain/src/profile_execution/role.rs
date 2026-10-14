use crate::model::{ContractRefV1, QueryProfileInputV1};
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ContractRoleV1 {
    Cohort,
    Event,
    Identity,
    Affiliation,
    Contact,
    Participation,
    Publication,
    SourcePlan,
    Completion,
}

impl ContractRoleV1 {
    pub const ALL: [Self; 9] = [
        Self::Cohort,
        Self::Event,
        Self::Identity,
        Self::Affiliation,
        Self::Contact,
        Self::Participation,
        Self::Publication,
        Self::SourcePlan,
        Self::Completion,
    ];

    pub(super) fn reference(self, input: &QueryProfileInputV1) -> &ContractRefV1 {
        match self {
            Self::Cohort => &input.policies.cohort,
            Self::Event => &input.policies.event,
            Self::Identity => &input.policies.identity,
            Self::Affiliation => &input.policies.affiliation,
            Self::Contact => &input.policies.contact,
            Self::Participation => &input.policies.participation,
            Self::Publication => &input.policies.publication,
            Self::SourcePlan => &input.source_plan,
            Self::Completion => &input.completion_contract,
        }
    }
}
