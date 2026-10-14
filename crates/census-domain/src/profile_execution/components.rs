use super::{ContractRoleV1, ProfileExecutionError};
use crate::model::{
    ContractRefV1, EventSemantics, RecruitingCatalogV1, ATHLETE_IDENTITY_POLICY,
    COHORT_ASSESSMENT_POLICY,
};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum PolicyComponentsV1 {
    Cohort {
        assessment_revision: u32,
    },
    Event {
        catalog: ContractRefV1,
    },
    Identity {
        acceptance_revision: u32,
    },
    Affiliation,
    Contact,
    Participation,
    Publication {
        logical_schema: u32,
        semantic_revision: u32,
    },
    SourcePlan,
    Completion,
}

pub(super) fn components_v1(
    role: ContractRoleV1,
) -> Result<PolicyComponentsV1, ProfileExecutionError> {
    match role {
        ContractRoleV1::Cohort => Ok(PolicyComponentsV1::Cohort {
            assessment_revision: COHORT_ASSESSMENT_POLICY,
        }),
        ContractRoleV1::Event => {
            let digest = RecruitingCatalogV1::with_policy(EventSemantics::Legacy18).digest()?;
            Ok(PolicyComponentsV1::Event {
                catalog: ContractRefV1::new(1, 1, &digest)?,
            })
        }
        ContractRoleV1::Identity => Ok(PolicyComponentsV1::Identity {
            acceptance_revision: ATHLETE_IDENTITY_POLICY,
        }),
        ContractRoleV1::Affiliation => Ok(PolicyComponentsV1::Affiliation),
        ContractRoleV1::Contact => Ok(PolicyComponentsV1::Contact),
        ContractRoleV1::Participation => Ok(PolicyComponentsV1::Participation),
        ContractRoleV1::Publication => Ok(PolicyComponentsV1::Publication {
            logical_schema: 3,
            semantic_revision: 18,
        }),
        ContractRoleV1::SourcePlan => Ok(PolicyComponentsV1::SourcePlan),
        ContractRoleV1::Completion => Ok(PolicyComponentsV1::Completion),
    }
}
