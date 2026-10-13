use super::{ContractRefV1, ProfileDate, RegionAssertionV1, RegionCompositionV1};
use super::{RegionContextV1, RegionMembershipContractV1, RegionMembershipError, RegionSchoolV1};
use crate::model::TeamId;
use serde::Serialize;

mod applicability;
mod reduce;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegionMembershipTargetV1 {
    School(RegionSchoolV1),
    Program {
        program: TeamId,
        affiliated_school: RegionSchoolV1,
    },
}

impl RegionMembershipTargetV1 {
    pub fn affiliated_school(&self) -> &RegionSchoolV1 {
        match self {
            Self::School(school)
            | Self::Program {
                affiliated_school: school,
                ..
            } => school,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionMembershipQueryV1 {
    target: RegionMembershipTargetV1,
    context: RegionContextV1,
    effective_on: ProfileDate,
}

impl RegionMembershipQueryV1 {
    pub fn new(
        target: RegionMembershipTargetV1,
        context: RegionContextV1,
        effective_on: ProfileDate,
    ) -> Result<Self, RegionMembershipError> {
        context.require_query_context()?;
        if let RegionMembershipTargetV1::Program { program, .. } = &target {
            super::boundary::program_id(program)?;
        }
        Ok(Self {
            target,
            context,
            effective_on,
        })
    }

    pub fn target(&self) -> &RegionMembershipTargetV1 {
        &self.target
    }

    pub fn context(&self) -> RegionContextV1 {
        self.context
    }

    pub fn effective_on(&self) -> &ProfileDate {
        &self.effective_on
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionMembershipVerdictV1 {
    Member,
    ExplicitNonMember,
    Conflicted,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionAssertionDispositionV1 {
    Applied,
    UnknownAssertion,
    OtherSubject,
    OtherSport,
    OtherCategory,
    OtherSchoolYear,
    UnknownSport,
    UnknownCategory,
    Undated,
    OutsideAssertionInterval,
    OutsideContractInterval,
    OutsideDeclaredJurisdictions,
    UnresolvedConstituents,
    ForeignConstituent,
    JurisdictionConflict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegionAssertionTraceV1<'a> {
    pub assertion: &'a RegionAssertionV1,
    pub disposition: RegionAssertionDispositionV1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegionCompositionResolutionV1<'a> {
    NoApplicableProgramComposition,
    Known(&'a RegionCompositionV1),
    Conflicted,
}

#[derive(Debug)]
pub struct RegionMembershipResolutionV1<'a> {
    contract: &'a RegionMembershipContractV1,
    query: &'a RegionMembershipQueryV1,
    verdict: RegionMembershipVerdictV1,
    composition: RegionCompositionResolutionV1<'a>,
}

impl<'a> RegionMembershipResolutionV1<'a> {
    pub fn verdict(&self) -> RegionMembershipVerdictV1 {
        self.verdict
    }

    pub fn composition(&self) -> RegionCompositionResolutionV1<'a> {
        self.composition
    }

    pub fn trace(&self) -> impl ExactSizeIterator<Item = RegionAssertionTraceV1<'a>> + 'a {
        let contract = self.contract;
        let query = self.query;
        contract
            .input()
            .assertions
            .iter()
            .map(move |assertion| RegionAssertionTraceV1 {
                assertion,
                disposition: applicability::classify(contract, query, assertion),
            })
    }
}

pub fn resolve_region_membership_v1<'a>(
    contract: &'a RegionMembershipContractV1,
    query: &'a RegionMembershipQueryV1,
) -> RegionMembershipResolutionV1<'a> {
    let (verdict, composition) = reduce::resolve(contract, query);
    RegionMembershipResolutionV1 {
        contract,
        query,
        verdict,
        composition,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RegionPriorityTierV1 {
    RegionFirst,
    Standard,
    Unresolved,
}

#[derive(Debug)]
pub struct RegionPriorityResolutionV1<'a> {
    pub tier: RegionPriorityTierV1,
    pub membership_evidence: RegionMembershipResolutionV1<'a>,
}

pub fn resolve_region_priority_v1<'a>(
    contract: &'a RegionMembershipContractV1,
    query: &'a RegionMembershipQueryV1,
) -> RegionPriorityResolutionV1<'a> {
    let membership_evidence = resolve_region_membership_v1(contract, query);
    let tier = match membership_evidence.verdict() {
        RegionMembershipVerdictV1::Member => RegionPriorityTierV1::RegionFirst,
        RegionMembershipVerdictV1::ExplicitNonMember => RegionPriorityTierV1::Standard,
        RegionMembershipVerdictV1::Conflicted | RegionMembershipVerdictV1::Unknown => {
            RegionPriorityTierV1::Unresolved
        }
    };
    RegionPriorityResolutionV1 {
        tier,
        membership_evidence,
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BoundRegionContractV1<'a> {
    contract: &'a RegionMembershipContractV1,
}

impl<'a> BoundRegionContractV1<'a> {
    pub fn contract(self) -> &'a RegionMembershipContractV1 {
        self.contract
    }

    pub fn resolve(self, query: &'a RegionMembershipQueryV1) -> RegionMembershipResolutionV1<'a> {
        resolve_region_membership_v1(self.contract, query)
    }
}

pub fn bind_region_contract_v1<'a>(
    reference: &ContractRefV1,
    contract: &'a RegionMembershipContractV1,
) -> Result<BoundRegionContractV1<'a>, RegionMembershipError> {
    if reference != &contract.reference()? {
        return Err(RegionMembershipError::ReferenceMismatch);
    }
    Ok(BoundRegionContractV1 { contract })
}
