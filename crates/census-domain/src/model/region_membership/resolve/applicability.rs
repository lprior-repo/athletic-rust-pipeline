use super::{RegionAssertionDispositionV1 as Disposition, RegionAssertionV1};
use super::{RegionMembershipContractV1, RegionMembershipQueryV1, RegionMembershipTargetV1};
use crate::model::region_membership::{RegionEffectiveV1, RegionStatementV1, RegionSubjectV1};
use crate::model::{Gender, Sport};

pub(super) fn classify(
    contract: &RegionMembershipContractV1,
    query: &RegionMembershipQueryV1,
    assertion: &RegionAssertionV1,
) -> Disposition {
    if !same_subject(query, assertion) {
        return Disposition::OtherSubject;
    }
    if !contract.admits_effective_day(query.effective_on()) {
        return Disposition::OutsideContractInterval;
    }
    if !contract.admits_jurisdiction(query.target().affiliated_school().jurisdiction()) {
        return Disposition::OutsideDeclaredJurisdictions;
    }
    match context_disposition(query, assertion) {
        Disposition::Applied => {}
        disposition => return disposition,
    }
    match &assertion.input().effective {
        RegionEffectiveV1::Undated => return Disposition::Undated,
        RegionEffectiveV1::Inclusive(range) if !range.contains(query.effective_on()) => {
            return Disposition::OutsideAssertionInterval
        }
        RegionEffectiveV1::Inclusive(_) => {}
    }
    match school_binding(query, assertion) {
        Disposition::Applied => {}
        disposition => return disposition,
    }
    match assertion.input().statement {
        RegionStatementV1::Unknown => Disposition::UnknownAssertion,
        RegionStatementV1::Member | RegionStatementV1::ExplicitNonMember => Disposition::Applied,
    }
}

fn same_subject(query: &RegionMembershipQueryV1, assertion: &RegionAssertionV1) -> bool {
    match (query.target(), &assertion.input().subject) {
        (RegionMembershipTargetV1::School(query), RegionSubjectV1::School(claim)) => {
            query.school() == claim.school()
        }
        (RegionMembershipTargetV1::Program { program, .. }, RegionSubjectV1::Program(claim)) => {
            program == claim.program()
        }
        (RegionMembershipTargetV1::School(_), RegionSubjectV1::Program(_))
        | (RegionMembershipTargetV1::Program { .. }, RegionSubjectV1::School(_)) => false,
    }
}

fn context_disposition(
    query: &RegionMembershipQueryV1,
    assertion: &RegionAssertionV1,
) -> Disposition {
    let context = assertion.input().context;
    if context.school_year != query.context().school_year {
        return Disposition::OtherSchoolYear;
    }
    if context.sport != Sport::Unknown && context.sport != query.context().sport {
        return Disposition::OtherSport;
    }
    if context.category != Gender::Unknown && context.category != query.context().category {
        return Disposition::OtherCategory;
    }
    if context.sport == Sport::Unknown {
        return Disposition::UnknownSport;
    }
    if context.category == Gender::Unknown {
        return Disposition::UnknownCategory;
    }
    Disposition::Applied
}

fn school_binding(query: &RegionMembershipQueryV1, assertion: &RegionAssertionV1) -> Disposition {
    let school = query.target().affiliated_school();
    match &assertion.input().subject {
        RegionSubjectV1::School(claim) => {
            if claim.jurisdiction() != school.jurisdiction() {
                return Disposition::JurisdictionConflict;
            }
            Disposition::Applied
        }
        RegionSubjectV1::Program(claim) => {
            if claim.composition().schools().is_empty() {
                return Disposition::UnresolvedConstituents;
            }
            match claim
                .composition()
                .schools()
                .iter()
                .find(|member| member.school() == school.school())
            {
                Some(member) if member.jurisdiction() == school.jurisdiction() => {
                    Disposition::Applied
                }
                Some(_) => Disposition::JurisdictionConflict,
                None => Disposition::ForeignConstituent,
            }
        }
    }
}
