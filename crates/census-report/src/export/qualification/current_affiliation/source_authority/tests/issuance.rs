use super::*;
use census_crawl::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, assess_retained_roster_source,
    capture_objects::prepare_retained_roster_capture_objects,
    manifest::{assess_retained_roster_parser_manifest, compiled_roster_parser_manifest},
    policies::compiled_roster_interpretation_contracts_v1,
};
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_claim, LocallyCheckedRosterIssuanceV1, RosterAuthorityLimitsV1,
    RosterAuthorityTextV1,
};
use census_store::{
    RosterIssuanceInspectionRequestV1, RosterReviewActorV1, RosterSourceReviewFieldsV1,
    RosterSourceReviewObjectsV1,
};

pub(super) fn review(
    fixture: &fixture::Fixture,
    capture: &fixture::Capture,
    operation: &str,
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
    let proposal = proposal(capture, evidence, grant)?;
    let parser = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let source = assess_retained_roster_source(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &proposal,
        grant,
    )?;
    let binding = assess_existing_roster_binding(&source, grant)?;
    let objects = source.capture_objects(grant)?;
    let supplied = RosterSourceReviewObjectsV1 {
        objects: vec![
            parser.canonical_bytes().to_vec(),
            parser.retained().schema_bytes()?,
            parser.retained().configuration_bytes()?,
            parser.retained().dependencies_bytes()?,
            capture.packet.roster().body()?.into_bytes(),
            objects.metadata_bytes(grant)?.to_vec(),
            objects.interpretation_bytes(grant)?.to_vec(),
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

fn object(bytes: &[u8]) -> Value {
    json!({"digest":super::super::hash(bytes),"bytes":bytes.len()})
}

fn proposal(
    capture: &fixture::Capture,
    evidence: &census_store::RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    let parser = compiled_roster_parser_manifest()?;
    let checked = assess_retained_roster_parser_manifest(
        parser.canonical_bytes(),
        &parser.parser_claim()?,
        grant,
    )?;
    let objects = prepare_retained_roster_capture_objects(evidence, &checked, grant)?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let source_id = capture
        .school
        .source_identities
        .iter()
        .find(|identity| identity.namespace == SourceNamespace::MilesplitSchool)
        .ok_or("controlled source owner")?;
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, source_id.id.clone())?;
    let program = SourceSubject::new(SourceNamespace::MilesplitTeam, source_id.id.clone())?;
    let body = capture.packet.roster().body()?;
    let url = capture.packet.roster().requested_url();
    let value = json!({
        "version":1,
        "scope":{"provider":"milesplit","owner":owner,"sport":capture.program.sport,"category":capture.program.gender,"policy":contracts.scope_reference()?,"relationship":"represents_school_program","temporal":"exact_utc_capture_day"},
        "parser":parser.parser_claim()?,
        "capture":{"occurrence":objects.occurrence(grant)?,"body":object(body.as_bytes()),"manifest":object(objects.metadata_bytes(grant)?),"interpretation":object(objects.interpretation_bytes(grant)?),"acquired_at":objects.acquired_at(grant)?,"observed_day":"2026-10-16","requested_url":url,"final_url":{"status":"observed","url":url},"retained_source_admission":object(&evidence.retained_source_admission_manifest(grant)?),"verification_grant":object(&evidence.verification_grant_manifest(grant)?)},
        "binding":{"school":capture.school.id,"program":capture.program.id,"school_source":owner,"program_source":program,"sport":capture.program.sport,"category":capture.program.gender,"school_year":2026,"year_convention":contracts.year_reference()?,"projection_merge_contract":contracts.projection_merge_reference()?,"school_projection":object(&serde_json::to_vec(&capture.school)?),"program_projection":object(&serde_json::to_vec(&capture.program)?),"provider_packet":object(&capture.packet.encode()?),"physical_projection_inventory":object(&evidence.physical_projection_manifest(grant)?),"store":evidence.store_identity(),"snapshot_revision":evidence.snapshot_sequence()},
        "semantic_review_evidence":object(b"controlled proposal placeholder replaced by exact prepared review")
    });
    Ok(decode_roster_issuance_claim(
        &serde_json::to_vec(&value)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?)
}
