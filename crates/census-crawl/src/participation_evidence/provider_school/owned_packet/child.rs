use super::super::directory::Selected;
use super::super::{read_directory_roster_child, ProviderSchoolDirectoryProofV1};
use crate::milesplit::{Site, SourceRowLocator};
use crate::net::FetchOutcome;
use crate::participation_evidence::invalid;
use crate::CrawlResult;
use census_domain::model::{AcquisitionDigest, CaptureDomainError, PublicSourceResourceV1};
use census_domain::roster_affiliation_authority::SourceRowLocator as DomainLocator;
use census_store::{PublicSchoolIntakePacketV1, StoreResult};

pub(super) struct CurrentChildProjection {
    pub(super) selected: Selected,
    pub(super) retained_kernel: AcquisitionDigest,
    pub(super) current_kernel: AcquisitionDigest,
}

pub(super) fn read(
    packet: &PublicSchoolIntakePacketV1,
    directory: &FetchOutcome,
    roster: &FetchOutcome,
) -> CrawlResult<CurrentChildProjection> {
    let site = packet.directory().work().slot().source_site();
    PublicSourceResourceV1::milesplit_team_directory()
        .validate_target(site, &directory.url)
        .map_err(domain)?;
    let target =
        PublicSourceResourceV1::milesplit_team_roster(packet.child_team_id()).map_err(domain)?;
    target.validate_target(site, &roster.url).map_err(domain)?;
    let final_url = roster
        .response_url
        .as_deref()
        .ok_or_else(|| invalid("owned provider school roster final URL absent"))?;
    target.validate_target(site, final_url).map_err(domain)?;
    let (team, proof, kernel) = read_directory_roster_child(directory, packet.child_team_id())?;
    let child = packet.child_evidence();
    if child.team_id() != packet.child_team_id()
        || child.team_id() != team.id
        || child.team_url() != team.url
        || child.roster_url() != roster.url
        || child.roster_url().strip_suffix("/roster") != Some(team.url.as_str())
        || child.parsed_teams() != proof.parsed_teams()
        || child.row_digest().as_str() != proof.row_sha256()
        || proof.capture_sha256() != directory.content_digest
    {
        return Err(invalid("owned provider school directory child projection differs").into());
    }
    require_locators(
        &proof,
        [child.row(), child.name(), child.city(), child.anchor()],
    )?;
    Ok(CurrentChildProjection {
        selected: Selected {
            team,
            proof,
            site: Site::for_jurisdiction(site),
        },
        retained_kernel: child.kernel_digest().clone(),
        current_kernel: kernel,
    })
}

fn require_locators(
    proof: &ProviderSchoolDirectoryProofV1,
    retained: [DomainLocator; 4],
) -> StoreResult<()> {
    let calculated = [proof.row(), proof.name(), proof.city(), proof.anchor()];
    if calculated
        .into_iter()
        .zip(retained)
        .any(|(left, right)| !same(left, right))
    {
        return Err(invalid(
            "owned provider school directory child locators differ",
        ));
    }
    Ok(())
}

fn same(left: SourceRowLocator, right: DomainLocator) -> bool {
    left.ordinal == right.ordinal()
        && left.byte_offset == right.byte_offset()
        && left.byte_length == right.byte_length()
}

fn domain(error: CaptureDomainError) -> census_store::StoreError {
    census_store::CaptureError::Domain(error).into()
}
