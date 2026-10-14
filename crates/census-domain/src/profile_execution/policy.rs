use super::components::{components_v1, PolicyComponentsV1};
use super::{ContractRoleV1, ProfileExecutionError};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(super) struct ProfilePolicyDescriptorV1 {
    evidence_rule: EvidenceRuleV1,
    component_support: ComponentSupportV1,
    components: PolicyComponentsV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum EvidenceRuleV1 {
    TypedPublishedGraduationAndDatedGradesAfterAliasUnionConflictsWithheld,
    SourceOwnedContextualRecruitingCatalogWalkWheelchairExcluded,
    CapturedPrimaryOrIndependentAttestedPersonKeysHardContradictionsWithheld,
    ResolvedSchoolAtAsOfPopulationGeographyDistinctFromHistoricalResultSchool,
    SameCapturePublishedMailboxRoleProgramCurrentSelectedSchoolYear,
    SourceOwnedIndividualResultsOrExplicitOrderedRelayMembersNoInferredLegs,
    OneFrozenGenerationSharedDerivationScopedCoverageAtomicVerifiedBundle,
    QualifiedPublicSourceObligationsBoundToScopeSeasonAndAcquisitionWindow,
    ExactProfileScopeRevisionAndVerifiedSourceTerminalEvidenceUnknownIsNotComplete,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ComponentSupportV1 {
    ExistingEvidenceComponentsProfileIntegrationPending,
    ExistingCatalogProfileIntegrationPending,
    ExistingAdmissibilityProfileIntegrationPending,
    DatedCurrentAffiliationUnimplemented,
    ExistingTenureComponentsProfileIntegrationPending,
    ExistingOwnedResultTypesProfileIntegrationPending,
    ExistingPolicy18ProfileIntegrationPending,
    ProfileBoundQualifiedSourceGraphUnimplemented,
    ProfileBoundCompletionFenceUnimplemented,
}

pub(super) fn policy_descriptor_v1(
    role: ContractRoleV1,
) -> Result<ProfilePolicyDescriptorV1, ProfileExecutionError> {
    let (evidence_rule, component_support) = evidence_semantics_v1(role);
    Ok(ProfilePolicyDescriptorV1 {
        evidence_rule,
        component_support,
        components: components_v1(role)?,
    })
}

fn evidence_semantics_v1(role: ContractRoleV1) -> (EvidenceRuleV1, ComponentSupportV1) {
    use ComponentSupportV1 as Support;
    use EvidenceRuleV1 as Rule;
    match role {
        ContractRoleV1::Cohort => (
            Rule::TypedPublishedGraduationAndDatedGradesAfterAliasUnionConflictsWithheld,
            Support::ExistingEvidenceComponentsProfileIntegrationPending,
        ),
        ContractRoleV1::Event => (
            Rule::SourceOwnedContextualRecruitingCatalogWalkWheelchairExcluded,
            Support::ExistingCatalogProfileIntegrationPending,
        ),
        ContractRoleV1::Identity => (
            Rule::CapturedPrimaryOrIndependentAttestedPersonKeysHardContradictionsWithheld,
            Support::ExistingAdmissibilityProfileIntegrationPending,
        ),
        ContractRoleV1::Affiliation => (
            Rule::ResolvedSchoolAtAsOfPopulationGeographyDistinctFromHistoricalResultSchool,
            Support::DatedCurrentAffiliationUnimplemented,
        ),
        ContractRoleV1::Contact => (
            Rule::SameCapturePublishedMailboxRoleProgramCurrentSelectedSchoolYear,
            Support::ExistingTenureComponentsProfileIntegrationPending,
        ),
        ContractRoleV1::Participation => (
            Rule::SourceOwnedIndividualResultsOrExplicitOrderedRelayMembersNoInferredLegs,
            Support::ExistingOwnedResultTypesProfileIntegrationPending,
        ),
        ContractRoleV1::Publication => (
            Rule::OneFrozenGenerationSharedDerivationScopedCoverageAtomicVerifiedBundle,
            Support::ExistingPolicy18ProfileIntegrationPending,
        ),
        ContractRoleV1::SourcePlan => (
            Rule::QualifiedPublicSourceObligationsBoundToScopeSeasonAndAcquisitionWindow,
            Support::ProfileBoundQualifiedSourceGraphUnimplemented,
        ),
        ContractRoleV1::Completion => (
            Rule::ExactProfileScopeRevisionAndVerifiedSourceTerminalEvidenceUnknownIsNotComplete,
            Support::ProfileBoundCompletionFenceUnimplemented,
        ),
    }
}
