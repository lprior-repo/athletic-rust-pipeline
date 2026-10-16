use super::{check_selection_configuration, ContractRoleV1, ProfileExecutionError};
use crate::model::{
    serialized_digest, ContractRefV1, EventSemantics, QueryProfileV1, RecruitingCatalogV1,
    ATHLETE_IDENTITY_POLICY, COHORT_ASSESSMENT_POLICY,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct Descriptor {
    version: u32,
    revision: u32,
    role: ContractRoleV1,
    scope: Scope,
    component: Component,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Scope {
    ExistingRetainedAuthorityV2QualificationOnly,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum Component {
    PublishedCohortAliasUnionConflictsWithheld { assessment_revision: u32 },
    Catalog21WithSelectedSportAndCompetitionCategory { catalog: ContractRefV1 },
    RetainedAuthorityLifecycle6ReversiblePublicIdentity { acceptance_revision: u32 },
    VerifiedScopedCurrentRosterObservationDayConflictsWithheld { interpretation_revision: u32 },
    RetainedProfessionalRoleAndMailboxSchoolYear,
    SelectedArtifact8AtomicFrozenBundle,
    RetainedInputsOnlyNoAcquisition,
    QualificationOnlyNoCensusCompletionCertification,
}

#[derive(Debug)]
#[must_use]
pub struct SupportedQualificationProfileV1<'a> {
    profile: &'a QueryProfileV1,
}

impl<'a> SupportedQualificationProfileV1<'a> {
    pub fn profile(&self) -> &'a QueryProfileV1 {
        self.profile
    }
}

pub fn selected_qualification_contract_ref_v1(
    role: ContractRoleV1,
) -> Result<ContractRefV1, ProfileExecutionError> {
    let component = match role {
        ContractRoleV1::Cohort => Component::PublishedCohortAliasUnionConflictsWithheld {
            assessment_revision: COHORT_ASSESSMENT_POLICY,
        },
        ContractRoleV1::Event => Component::Catalog21WithSelectedSportAndCompetitionCategory {
            catalog: ContractRefV1::new(
                1,
                3,
                &RecruitingCatalogV1::with_policy(EventSemantics::ResultGroups21).digest()?,
            )?,
        },
        ContractRoleV1::Identity => {
            Component::RetainedAuthorityLifecycle6ReversiblePublicIdentity {
                acceptance_revision: ATHLETE_IDENTITY_POLICY,
            }
        }
        ContractRoleV1::Affiliation => {
            Component::VerifiedScopedCurrentRosterObservationDayConflictsWithheld {
                interpretation_revision: 1,
            }
        }
        ContractRoleV1::Contact => Component::RetainedProfessionalRoleAndMailboxSchoolYear,
        ContractRoleV1::Publication => Component::SelectedArtifact8AtomicFrozenBundle,
        ContractRoleV1::SourcePlan => Component::RetainedInputsOnlyNoAcquisition,
        ContractRoleV1::Completion => Component::QualificationOnlyNoCensusCompletionCertification,
        ContractRoleV1::Participation => {
            return Ok(crate::participation_candidates::participation_policy_v3()?);
        }
    };
    let digest = serialized_digest(&Descriptor {
        version: 1,
        revision: 2,
        role,
        scope: Scope::ExistingRetainedAuthorityV2QualificationOnly,
        component,
    })?;
    Ok(ContractRefV1::new(1, 2, &digest)?)
}

pub fn validate_selected_qualification_profile_v1(
    profile: &QueryProfileV1,
) -> Result<SupportedQualificationProfileV1<'_>, ProfileExecutionError> {
    ContractRoleV1::ALL.into_iter().try_for_each(|role| {
        let expected = selected_qualification_contract_ref_v1(role)?;
        if role.reference(profile.input()) != &expected {
            return Err(ProfileExecutionError::UnsupportedContractReference { role });
        }
        Ok(())
    })?;
    check_selection_configuration(profile)?;
    Ok(SupportedQualificationProfileV1 { profile })
}

#[cfg(test)]
mod tests;
