use crate::milesplit::{SourceRowLocator, TeamRef};
use crate::net::CapturedFetch;
use crate::participation_evidence::provider_school::{
    read_directory_roster_child, ProviderSchoolDirectoryProofV1,
};
use crate::{CollectionDisposition, CrawlError, CrawlResult};
use census_domain::model::{
    AcquisitionDigest, CaptureDomainError, PublicSourceResourceKindV1, PublicSourceResourceV1,
};
use census_domain::roster_affiliation_authority::SourceRowLocator as DomainLocator;
use census_store::{
    PublicDirectoryEvidenceV1, PublicDirectoryRosterAssessmentV1, PublicDirectorySemanticContractV1,
};

pub struct CheckedDirectoryRosterWitnessV1<'e> {
    evidence: &'e PublicDirectoryEvidenceV1,
    team: TeamRef,
    proof: ProviderSchoolDirectoryProofV1,
    roster_url: String,
    row_digest: AcquisitionDigest,
    kernel_digest: AcquisitionDigest,
}

pub fn assess_directory_roster_child<'e>(
    evidence: &'e PublicDirectoryEvidenceV1,
    captured: &CapturedFetch,
    team_id: &str,
) -> CrawlResult<CheckedDirectoryRosterWitnessV1<'e>> {
    let resource = PublicSourceResourceV1::milesplit_team_roster(team_id).map_err(domain)?;
    require_original(evidence, captured)?;
    let (team, proof, kernel_digest) = read_directory_roster_child(captured.capture(), team_id)?;
    require_projection(captured, team_id, &team, &proof)?;
    let roster_url = roster_url(&team.url)?;
    resource
        .validate_target(evidence.work().slot().source_site(), &roster_url)
        .map_err(domain)?;
    let row_digest = AcquisitionDigest::parse(proof.row_sha256()).map_err(domain)?;
    Ok(CheckedDirectoryRosterWitnessV1 {
        evidence,
        team,
        proof,
        roster_url,
        row_digest,
        kernel_digest,
    })
}

fn require_original(
    evidence: &PublicDirectoryEvidenceV1,
    captured: &CapturedFetch,
) -> CrawlResult<()> {
    let held = evidence.retained();
    let work = evidence.work();
    let capture = captured.capture();
    if capture.body.is_empty()
        || capture.body.len() > 8 * 1024 * 1024
        || held.reference() != held.body_reference()
        || held.revalidation().is_some()
        || held.response().status() != 200
        || &work.id().map_err(domain)? != held.reference().attempt().work_id()
        || work
            .slot()
            .public_resource()
            .map(PublicSourceResourceV1::kind)
            != Some(PublicSourceResourceKindV1::MileSplitTeamDirectory)
        || capture.url != work.request().original_url()
        || capture.response_url.as_deref() != Some(capture.url.as_str())
        || capture.status != 200
    {
        return Err(refused("owned directory original capture required"));
    }
    let view = captured
        .retained_observed()?
        .ok_or_else(|| refused("owned directory normalized observation required"))?;
    if view.occurrence() != held.reference()
        || view.original() != held.body_reference()
        || view.occurrence() != view.original()
        || view.original_metadata() != held.prepared().metadata_bytes()
        || capture.content_digest != held.prepared().content_digest().as_str()
        || u64::try_from(capture.bytes).map_err(|_| refused("owned directory body length"))?
            != held.prepared().content_bytes()
    {
        return Err(refused("owned directory retained binding differs"));
    }
    require_observed_type(capture, view.observed_http())
}

fn require_observed_type(
    capture: &crate::net::FetchOutcome,
    observed: &census_domain::model::NormalizedObservedHttpV1,
) -> CrawlResult<()> {
    let final_response = observed
        .hops()
        .last()
        .ok_or_else(|| refused("owned directory final observation absent"))?
        .response();
    let mut types = final_response
        .headers()
        .iter()
        .filter(|header| header.name() == "content-type");
    let first = types
        .next()
        .ok_or_else(|| refused("owned directory observed MIME absent"))?;
    let value = std::str::from_utf8(first.value())
        .map_err(|_| refused("owned directory observed MIME is not UTF-8"))?;
    if types.next().is_some() || capture.content_type.as_deref() != Some(value) {
        return Err(refused("owned directory observed MIME differs"));
    }
    Ok(())
}

fn require_projection(
    captured: &CapturedFetch,
    team_id: &str,
    team: &TeamRef,
    proof: &ProviderSchoolDirectoryProofV1,
) -> CrawlResult<()> {
    if team.id != team_id
        || proof.capture_sha256() != captured.capture().content_digest
        || proof.disposition() != CollectionDisposition::Complete
        || proof.errors() != 0
        || !proof.unfinished().is_empty()
        || proof.parsed_teams() == 0
    {
        return Err(refused("owned directory checked projection differs"));
    }
    Ok(())
}

fn roster_url(team_url: &str) -> CrawlResult<String> {
    let required = team_url
        .len()
        .checked_add("/roster".len())
        .ok_or_else(|| refused("owned directory roster URL length"))?;
    if required > 2048 {
        return Err(refused("owned directory roster URL length"));
    }
    let mut result = String::new();
    result
        .try_reserve_exact(required)
        .map_err(|_| refused("owned directory roster URL allocation"))?;
    result.push_str(team_url);
    result.push_str("/roster");
    Ok(result)
}

fn domain(error: CaptureDomainError) -> CrawlError {
    CrawlError::Store(census_store::CaptureError::Domain(error).into())
}

fn refused(detail: &'static str) -> CrawlError {
    CrawlError::Store(census_store::StoreError::Refused {
        detail: detail.into(),
    })
}

fn locator(value: SourceRowLocator) -> DomainLocator {
    DomainLocator::new(value.ordinal, value.byte_offset, value.byte_length)
}

impl PublicDirectoryRosterAssessmentV1 for CheckedDirectoryRosterWitnessV1<'_> {
    fn evidence(&self) -> &PublicDirectoryEvidenceV1 {
        self.evidence
    }

    fn team_id(&self) -> &str {
        &self.team.id
    }

    fn team_url(&self) -> &str {
        &self.team.url
    }

    fn roster_url(&self) -> &str {
        &self.roster_url
    }

    fn row(&self) -> DomainLocator {
        locator(self.proof.row())
    }

    fn name(&self) -> DomainLocator {
        locator(self.proof.name())
    }

    fn city(&self) -> DomainLocator {
        locator(self.proof.city())
    }

    fn anchor(&self) -> DomainLocator {
        locator(self.proof.anchor())
    }

    fn row_digest(&self) -> &AcquisitionDigest {
        &self.row_digest
    }

    fn kernel_digest(&self) -> &AcquisitionDigest {
        &self.kernel_digest
    }

    fn parsed_teams(&self) -> usize {
        self.proof.parsed_teams()
    }

    fn semantic_contract(&self) -> PublicDirectorySemanticContractV1 {
        PublicDirectorySemanticContractV1::MileSplitActiveDirectoryRosterV1
    }
}

#[cfg(test)]
mod tests;
