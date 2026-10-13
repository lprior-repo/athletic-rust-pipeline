use super::applicability::classify;
use super::{
    RegionAssertionDispositionV1 as Disposition, RegionCompositionResolutionV1 as Composition,
};
use super::{
    RegionMembershipContractV1, RegionMembershipQueryV1, RegionMembershipVerdictV1 as Verdict,
};
use crate::model::region_membership::{RegionStatementV1, RegionSubjectV1};

pub(super) fn resolve<'a>(
    contract: &'a RegionMembershipContractV1,
    query: &RegionMembershipQueryV1,
) -> (Verdict, Composition<'a>) {
    let mut verdict = Verdict::Unknown;
    let mut composition = Composition::NoApplicableProgramComposition;
    for assertion in &contract.input().assertions {
        let disposition = classify(contract, query, assertion);
        if retains_composition(disposition) {
            if let RegionSubjectV1::Program(program) = &assertion.input().subject {
                composition = merge_composition(composition, program.composition());
            }
        }
        verdict = match disposition {
            Disposition::Applied => merge_statement(verdict, assertion.input().statement),
            Disposition::JurisdictionConflict => Verdict::Conflicted,
            Disposition::UnknownAssertion
            | Disposition::OtherSubject
            | Disposition::OtherSport
            | Disposition::OtherCategory
            | Disposition::OtherSchoolYear
            | Disposition::UnknownSport
            | Disposition::UnknownCategory
            | Disposition::Undated
            | Disposition::OutsideAssertionInterval
            | Disposition::OutsideContractInterval
            | Disposition::OutsideDeclaredJurisdictions
            | Disposition::UnresolvedConstituents
            | Disposition::ForeignConstituent => verdict,
        };
    }
    if composition == Composition::Conflicted {
        verdict = Verdict::Conflicted;
    }
    (verdict, composition)
}

fn retains_composition(disposition: Disposition) -> bool {
    match disposition {
        Disposition::Applied
        | Disposition::UnknownAssertion
        | Disposition::ForeignConstituent
        | Disposition::JurisdictionConflict => true,
        Disposition::OtherSubject
        | Disposition::OtherSport
        | Disposition::OtherCategory
        | Disposition::OtherSchoolYear
        | Disposition::UnknownSport
        | Disposition::UnknownCategory
        | Disposition::Undated
        | Disposition::OutsideAssertionInterval
        | Disposition::OutsideContractInterval
        | Disposition::OutsideDeclaredJurisdictions
        | Disposition::UnresolvedConstituents => false,
    }
}

fn merge_composition<'a>(
    current: Composition<'a>,
    next: &'a super::RegionCompositionV1,
) -> Composition<'a> {
    match current {
        Composition::NoApplicableProgramComposition => Composition::Known(next),
        Composition::Known(previous) if previous == next => current,
        Composition::Known(_) | Composition::Conflicted => Composition::Conflicted,
    }
}

fn merge_statement(current: Verdict, statement: RegionStatementV1) -> Verdict {
    match (current, statement) {
        (Verdict::Conflicted, _) => Verdict::Conflicted,
        (current, RegionStatementV1::Unknown) => current,
        (Verdict::Unknown | Verdict::Member, RegionStatementV1::Member) => Verdict::Member,
        (Verdict::Unknown | Verdict::ExplicitNonMember, RegionStatementV1::ExplicitNonMember) => {
            Verdict::ExplicitNonMember
        }
        (Verdict::Member, RegionStatementV1::ExplicitNonMember)
        | (Verdict::ExplicitNonMember, RegionStatementV1::Member) => Verdict::Conflicted,
    }
}
