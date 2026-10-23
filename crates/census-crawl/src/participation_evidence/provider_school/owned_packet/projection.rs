use super::capture::ObservedCapture;
use super::child::CurrentChildProjection;
use super::OWNED_PROVIDER_SCHOOL_INTERPRETATION;
use super::{
    OwnedProviderSchoolPacketV1, OWNED_PROVIDER_SCHOOL_INTAKE_PHASE,
    OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT,
};
use crate::milesplit::{RosterOwnerWitnessPolicy, Site};
use crate::participation_evidence::invalid;
use crate::participation_evidence::provider_school::{
    CheckedProviderSchoolPacketV1, ProviderSchoolDirectoryProofV1, ProviderSchoolPacketRefV1,
};
use crate::CrawlResult;
use census_domain::model::{
    AcquisitionDigest, AcquisitionOccurrenceRefV1, CanonicalSchool, Evidence,
    NormalizedObservedHttpV1, SourceNamespace, SourceRef, SourceSchoolObservation,
    MAX_CAPTURE_RECORD_BYTES,
};
use census_store::{PublicSchoolIntakeCaptureV1, StoreResult};
use serde::Serialize;

pub(super) fn read(
    packet: &OwnedProviderSchoolPacketV1,
    interpretation: CurrentChildProjection,
    directory: &ObservedCapture,
    roster: &ObservedCapture,
) -> CrawlResult<CheckedProviderSchoolPacketV1> {
    let CurrentChildProjection {
        selected,
        retained_kernel,
        current_kernel,
    } = interpretation;
    let kernels = DirectoryInterpretation {
        retained_child_kernel: &retained_kernel,
        current_kernel: &current_kernel,
    };
    let reference = packet.reference()?;
    let projected = super::super::projection::roster(
        &selected,
        &roster.fetch,
        RosterOwnerWitnessPolicy::CurrentBody,
    )?;
    let mut school = projected.school;
    school.evidence.clear();
    school
        .evidence
        .try_reserve_exact(2)
        .map_err(|_| invalid("owned provider school evidence allocation refused"))?;
    school.evidence.push(evidence(
        packet.directory(),
        directory,
        &selected.site,
        &reference,
        Some(&selected.proof),
        &kernels,
    )?);
    school.evidence.push(evidence(
        packet.roster(),
        roster,
        &selected.site,
        &reference,
        None,
        &kernels,
    )?);
    let observation = school_observation(&school, directory, &selected.proof, &reference)?;
    Ok(CheckedProviderSchoolPacketV1 {
        reference,
        semantic_contract: OWNED_PROVIDER_SCHOOL_SEMANTIC_CONTRACT,
        school,
        observation,
        roster_verdict: projected.verdict,
        directory_proof: selected.proof,
        published_roster_owners: projected.owners,
    })
}

fn school_observation(
    school: &CanonicalSchool,
    directory: &ObservedCapture,
    proof: &ProviderSchoolDirectoryProofV1,
    reference: &ProviderSchoolPacketRefV1,
) -> CrawlResult<SourceSchoolObservation> {
    let mut observation = SourceSchoolObservation::of_school(
        &SourceNamespace::MilesplitSchool,
        school,
        &directory.fetch.fetched_at,
    )
    .ok_or_else(|| invalid("owned provider school projection has no source identity"))?;
    observation.source_row_key = format!(
        "{}:{}:directory:{}:{}:{}",
        OWNED_PROVIDER_SCHOOL_INTAKE_PHASE,
        reference.as_str(),
        proof.row().ordinal,
        proof.row().byte_offset,
        proof.row().byte_length
    );
    Ok(observation)
}

#[derive(Serialize)]
struct DirectoryInterpretation<'a> {
    retained_child_kernel: &'a AcquisitionDigest,
    current_kernel: &'a AcquisitionDigest,
}

#[derive(Serialize)]
struct Provenance<'a> {
    role: &'static str,
    phase: &'static str,
    interpretation: &'static str,
    packet: &'a str,
    capture_sha256: &'a str,
    metadata_sha256: &'a AcquisitionDigest,
    occurrence: &'a AcquisitionOccurrenceRefV1,
    observed_http: &'a NormalizedObservedHttpV1,
    directory_fields: Option<&'a ProviderSchoolDirectoryProofV1>,
    directory_interpretation: &'a DirectoryInterpretation<'a>,
}

fn evidence(
    retained: &PublicSchoolIntakeCaptureV1,
    capture: &ObservedCapture,
    site: &Site,
    reference: &ProviderSchoolPacketRefV1,
    directory: Option<&ProviderSchoolDirectoryProofV1>,
    kernels: &DirectoryInterpretation<'_>,
) -> StoreResult<Evidence> {
    let mut evidence = Evidence::parsed(
        SourceRef::new(site.source_id(), Some(capture.fetch.url.clone())),
        capture.fetch.fetched_at.clone(),
    );
    let provenance = Provenance {
        role: if directory.is_some() {
            "directory_name_city"
        } else {
            "provider_roster_owner"
        },
        phase: OWNED_PROVIDER_SCHOOL_INTAKE_PHASE,
        interpretation: OWNED_PROVIDER_SCHOOL_INTERPRETATION,
        packet: reference.as_str(),
        capture_sha256: &capture.fetch.content_digest,
        metadata_sha256: retained.prepared().metadata_digest(),
        occurrence: retained.occurrence(),
        observed_http: &capture.observed,
        directory_fields: directory,
        directory_interpretation: kernels,
    };
    let encoded =
        crate::participation_evidence::codec::encode(&provenance, MAX_CAPTURE_RECORD_BYTES + 8192)?;
    evidence.note = Some(
        String::from_utf8(encoded)
            .map_err(|_| invalid("owned provider school evidence encoding invalid"))?,
    );
    Ok(evidence)
}
