use super::*;
use census_crawl::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, assess_retained_roster_source_v2,
    manifest::compiled_roster_parser_manifest,
    policies::compiled_roster_interpretation_contracts_v1,
};
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_claim, RosterAuthorityLimitsV1, RosterAuthorityTextV1,
};
use census_store::{
    RosterIssuanceInspectionRequestV1, RosterReviewActorV1, RosterSourceReviewFieldsV1,
    RosterSourceReviewObjectsV1,
};

pub(super) fn review_forged(
    fixture: &fixture::Fixture,
    capture: &fixture::Capture,
    operation: &str,
    mutation: super::Mutation,
) -> TestResult {
    let schema = census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    let retained = fixture
        .store
        .snapshot()
        .read_retained_capture_root_v1(
            schema,
            capture.packet.reference()?.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("controlled retained packet")?;
    let request = RosterIssuanceInspectionRequestV1 {
        root: retained.root().clone(),
        school: capture.school.id.clone(),
        program: capture.program.id.clone(),
    };
    let mut inspection = fixture
        .store
        .inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = issuance::proposal(capture, evidence, grant)?;
    let parser = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let source = assess_retained_roster_source_v2(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &proposal,
        grant,
    )?;
    let binding = assess_existing_roster_binding(&source, grant)?;
    let objects = source.capture_objects(grant)?;
    let forged = super::wire::mutate(objects.interpretation_bytes(grant)?, mutation)?;
    let mut changed = serde_json::to_value(&proposal)?;
    *changed
        .pointer_mut("/capture/interpretation")
        .ok_or("interpretation reference")? =
        json!({"digest":super::super::super::super::hash(&forged),"bytes":forged.len()});
    let proposal = decode_roster_issuance_claim(
        &serde_json::to_vec(&changed)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?;
    let supplied = RosterSourceReviewObjectsV1 {
        objects: vec![
            parser.canonical_bytes().to_vec(),
            parser.retained().schema_bytes()?,
            parser.retained().configuration_bytes()?,
            parser.retained().dependencies_bytes()?,
            capture.packet.roster().body()?.into_bytes(),
            objects.metadata_bytes(grant)?.to_vec(),
            forged,
            capture.packet.encode()?,
            serde_json::to_vec(binding.school(grant)?)?,
            serde_json::to_vec(binding.program(grant)?)?,
            evidence.physical_projection_manifest(grant)?,
            evidence.retained_source_admission_manifest(grant)?,
            evidence.verification_grant_manifest(grant)?,
        ],
        scope_policy: contracts.scope.to_vec(),
        year_convention: contracts.year.to_vec(),
        projection_merge_contract: contracts.projection_merge.to_vec(),
    };
    let fields = RosterSourceReviewFieldsV1 { operation: RosterAuthorityTextV1::parse(operation)?, actor: RosterReviewActorV1::LocalOperator, decision: census_store::RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation, reviewed_at: fixture::instant()?, rationale: RosterAuthorityTextV1::parse("Controlled Report regression fixture; not production approval or genuine source qualification")? };
    let prepared = fixture
        .store
        .prepare_roster_source_review_v1(inspection, proposal, fields, supplied)?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_string();
    fixture
        .store
        .apply_roster_source_review_v1(prepared, &bytes, &digest)?;
    Ok(())
}
