use crate::milesplit::{LocatedRosterAthlete, RosterAthlete, RosterVerdict, SourceRowLocator};
use crate::participation_evidence::provider_school::{
    OwnedProviderSchoolPacketV1, ProviderSchoolPacketRefV1,
};
use crate::participation_evidence::roster_source_issuance::current_roster_semantic_kernel_digest;
use crate::participation_evidence::{hash, invalid};
use crate::CrawlResult;
use census_domain::model::{AcquisitionDigest, CaptureDomainError, PublicSourceResourceV1};
use census_domain::UsJurisdiction;
use census_store::{StoreError, StoreResult};

#[derive(Debug)]
pub struct CheckedOwnedRosterProfileChildV1 {
    packet_reference: ProviderSchoolPacketRefV1,
    site: UsJurisdiction,
    member: LocatedRosterAthlete,
    row_digest: AcquisitionDigest,
    kernel_digest: AcquisitionDigest,
}

impl CheckedOwnedRosterProfileChildV1 {
    pub fn packet_reference(&self) -> &ProviderSchoolPacketRefV1 {
        &self.packet_reference
    }

    pub fn site(&self) -> UsJurisdiction {
        self.site
    }

    pub fn athlete(&self) -> &RosterAthlete {
        self.member.athlete()
    }

    pub fn row(&self) -> SourceRowLocator {
        self.member.locator()
    }

    pub fn row_digest(&self) -> &AcquisitionDigest {
        &self.row_digest
    }

    pub fn kernel_digest(&self) -> &AcquisitionDigest {
        &self.kernel_digest
    }
}

pub fn read_owned_roster_profile_child_v1(
    packet: &OwnedProviderSchoolPacketV1,
    athlete_id: &str,
) -> CrawlResult<CheckedOwnedRosterProfileChildV1> {
    let target = PublicSourceResourceV1::milesplit_athlete_profile(athlete_id).map_err(domain)?;
    let checked = packet.read()?;
    let site = packet.roster().work().slot().source_site();
    let member = selected_member(checked.roster_verdict().clone(), athlete_id)?;
    target
        .validate_target(site, &member.athlete().profile_url)
        .map_err(domain)?;
    let row_digest = row_digest(packet.roster().body(), member.locator())?;
    let kernel_digest =
        AcquisitionDigest::parse(current_roster_semantic_kernel_digest()?).map_err(domain)?;
    Ok(CheckedOwnedRosterProfileChildV1 {
        packet_reference: checked.reference().clone(),
        site,
        member,
        row_digest,
        kernel_digest,
    })
}

fn selected_member(verdict: RosterVerdict, athlete_id: &str) -> StoreResult<LocatedRosterAthlete> {
    let roster = match verdict {
        RosterVerdict::Complete { roster } | RosterVerdict::Partial { roster, .. } => roster,
        RosterVerdict::Quarantined { .. } => {
            return Err(invalid("owned roster profile child roster is quarantined"));
        }
    };
    let mut matching = roster
        .athletes
        .into_iter()
        .filter(|member| member.athlete().athlete_id == athlete_id);
    let member = matching
        .next()
        .ok_or_else(|| invalid("owned roster profile child athlete absent"))?;
    if matching.next().is_some() {
        return Err(invalid("owned roster profile child athlete is ambiguous"));
    }
    Ok(member)
}

fn row_digest(body: &[u8], row: SourceRowLocator) -> StoreResult<AcquisitionDigest> {
    if row.byte_length == 0 {
        return Err(invalid("owned roster profile child row is empty"));
    }
    let end = row
        .byte_offset
        .checked_add(row.byte_length)
        .ok_or(StoreError::CounterOverflow)?;
    let original = body
        .get(row.byte_offset..end)
        .ok_or_else(|| invalid("owned roster profile child row is outside original body"))?;
    AcquisitionDigest::parse(&hash(original)).map_err(domain)
}

fn domain(error: CaptureDomainError) -> StoreError {
    census_store::CaptureError::Domain(error).into()
}
