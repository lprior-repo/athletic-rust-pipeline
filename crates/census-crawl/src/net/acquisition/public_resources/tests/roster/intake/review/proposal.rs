use super::*;
use crate::participation_evidence::roster_source_issuance::{
    manifest::assess_retained_roster_parser_manifest, prepare_retained_roster_capture_objects_v3,
};
use census_domain::model::SourceSubject;
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_proposal_v1, LocallyCheckedRosterProposalV1, RosterAuthorityLimitsV1,
};
use census_store::{RosterIssuanceInspectionV1, RosterSourceReviewObjectsV1};

mod value;

struct SourceObjects {
    canonical: Vec<u8>,
    schema: Vec<u8>,
    configuration: Vec<u8>,
    dependencies: Vec<u8>,
    body: Vec<u8>,
    metadata: Vec<u8>,
    interpretation: Vec<u8>,
    packet: Vec<u8>,
    school: Vec<u8>,
    program: Vec<u8>,
    inventory: Vec<u8>,
    admission: Vec<u8>,
    verification: Vec<u8>,
}

pub(super) fn derive(
    packet: &PublicSchoolIntakePacketV1,
    school: &CanonicalSchool,
    program: &CanonicalTeam,
    inspection: &mut RosterIssuanceInspectionV1<'_>,
) -> TestResult<(LocallyCheckedRosterProposalV1, RosterSourceReviewObjectsV1)> {
    let (evidence, grant) = inspection.source_parts()?;
    let compiled = compiled_roster_parser_manifest()?;
    let parser = assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(),
        &compiled.parser_claim()?,
        grant,
    )?;
    let capture = prepare_retained_roster_capture_objects_v3(evidence, &parser, grant)?;
    let objects = SourceObjects {
        canonical: compiled.canonical_bytes().to_vec(),
        schema: compiled.retained().schema_bytes()?,
        configuration: compiled.retained().configuration_bytes()?,
        dependencies: compiled.retained().dependencies_bytes()?,
        body: packet.roster().body().to_vec(),
        metadata: capture.metadata_bytes(grant)?.to_vec(),
        interpretation: capture.interpretation_bytes(grant)?.to_vec(),
        packet: packet.encode()?,
        school: serde_json::to_vec(school)?,
        program: serde_json::to_vec(program)?,
        inventory: evidence.physical_projection_manifest(grant)?,
        admission: evidence.retained_source_admission_manifest(grant)?,
        verification: evidence.verification_grant_manifest(grant)?,
    };
    let contracts = compiled_roster_interpretation_contracts_v1();
    let value = value::derive(
        school,
        program,
        &objects,
        evidence,
        &compiled.parser_claim()?,
        &contracts,
        capture.occurrence(grant)?,
        capture.acquired_at(grant)?,
        packet,
    )?;
    let proposal = decode_roster_issuance_proposal_v1(
        &serde_json::to_vec(&value)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?;
    let supplied = supplied(objects, &contracts);
    Ok((proposal, supplied))
}

fn supplied(
    objects: SourceObjects,
    contracts: &crate::participation_evidence::roster_source_issuance::policies::RosterInterpretationContractsV1<'_>,
) -> RosterSourceReviewObjectsV1 {
    RosterSourceReviewObjectsV1 {
        objects: vec![
            objects.canonical,
            objects.schema,
            objects.configuration,
            objects.dependencies,
            objects.body,
            objects.metadata,
            objects.interpretation,
            objects.packet,
            objects.school,
            objects.program,
            objects.inventory,
            objects.admission,
            objects.verification,
        ],
        scope_policy: contracts.scope.to_vec(),
        year_convention: contracts.year.to_vec(),
        projection_merge_contract: contracts.projection_merge.to_vec(),
    }
}
