use crate::milesplit::{CurrentRosterAssertion, RosterVerdict};
use crate::participation_evidence::invalid;
use crate::participation_evidence::provider_school::CheckedProviderSchoolPacketV1;
use crate::CrawlResult;
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::roster_affiliation_authority::LocallyCheckedRosterIssuanceV1;
use census_store::StoreError;

pub(in super::super) fn scope(
    checked: &CheckedProviderSchoolPacketV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<usize> {
    let RosterVerdict::Complete { roster } = checked.roster_verdict() else {
        return Err(invalid("roster source incomplete roster").into());
    };
    scope_members(roster, proposal, grant)
}

pub(in super::super) fn scope_row_local(
    checked: &CheckedProviderSchoolPacketV1,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<usize> {
    let roster = super::super::semantic_inventory_v2::checked_roster(checked, grant)?;
    scope_members(roster, proposal, grant)
}

fn scope_members(
    roster: &crate::milesplit::Roster,
    proposal: &LocallyCheckedRosterIssuanceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> CrawlResult<usize> {
    grant
        .charge_steps(roster.athletes.len())
        .map_err(super::domain)?;
    if roster.team.id != proposal.scope().owner().provider_id()
        || proposal.parser().interpretation() != CurrentRosterAssertion::INTERPRETATION_REVISION
    {
        return Err(invalid("roster source owner or interpretation mismatch").into());
    }
    let count = roster.athletes.iter().try_fold(0_usize, |count, member| {
        let assertion = member
            .current_roster()
            .ok_or_else(|| invalid("roster source current statement absent"))?;
        if assertion.owner() != roster.team.id
            || assertion.member() != member.athlete().athlete_id
            || assertion.row() != member.locator()
        {
            return Err(invalid("roster source scoped member mismatch"));
        }
        if member.athlete().gender == proposal.scope().category()
            && assertion.supports(proposal.scope().sport())
        {
            count.checked_add(1).ok_or(StoreError::CounterOverflow)
        } else {
            Ok(count)
        }
    })?;
    if count == 0 {
        return Err(invalid("roster source program scope absent").into());
    }
    Ok(count)
}
