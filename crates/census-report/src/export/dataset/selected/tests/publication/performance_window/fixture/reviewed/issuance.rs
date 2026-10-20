use super::*;
use census_crawl::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, assess_retained_roster_source_v3,
    manifest::{
        assess_retained_roster_parser_manifest, compiled_roster_parser_manifest,
        CompiledRosterParserManifestV1,
    },
    policies::compiled_roster_interpretation_contracts_v1,
    prepare_retained_roster_capture_objects_v3, RosterSourceEvidenceAssessmentV1,
};
use census_domain::evidence_subject_attachment::AttachmentReplayGrantV1;
use census_domain::model::SourceSubject;
use census_domain::roster_affiliation_authority::{
    decode_roster_issuance_claim, LocallyCheckedRosterIssuanceV1, RosterAuthorityLimitsV1,
    RosterAuthorityTextV1,
};
use census_store::{
    RosterEnrollmentReviewV1, RosterIssuanceInspectionRequestV1, RosterReviewActorV1,
    RosterSourceEvidenceV1, RosterSourceReviewFieldsV1, RosterSourceReviewObjectsV1,
};

pub(in super::super) fn review(store: &Store, capture: &Capture) -> TestResult {
    enroll(store)?;
    let schema = census_store::RetainedCaptureRootSchemaV1::ProviderSchoolPacketV1;
    let retained = store
        .snapshot()
        .read_retained_capture_root_v1(
            schema,
            capture.packet.reference()?.as_str(),
            &mut AttachmentReplayGrantV1::new(),
        )?
        .ok_or("fresh retained roster packet")?;
    let mut inspection = store.inspect_roster_source_issuance_v1(
        RosterIssuanceInspectionRequestV1 {
            root: retained.root().clone(),
            school: capture.school.id.clone(),
            program: capture.program.id.clone(),
        },
        AttachmentReplayGrantV1::new(),
    )?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = proposal(capture, evidence, grant)?;
    let parser = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let source = assess_retained_roster_source_v3(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &proposal,
        grant,
    )?;
    let supplied = supplied(capture, &source, &parser, grant)?;
    let prepared = store.prepare_roster_source_review_v1(
        inspection,
        proposal.into_proposal(),
        fields()?,
        supplied,
    )?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_string();
    check!(eq; hash(&bytes), digest);
    store.apply_roster_source_review_v1(prepared, &bytes, &digest)?;
    Ok(())
}

fn supplied(
    capture: &Capture,
    source: &RosterSourceEvidenceAssessmentV1<'_>,
    parser: &CompiledRosterParserManifestV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<RosterSourceReviewObjectsV1> {
    let binding = assess_existing_roster_binding(source, grant)?;
    let evidence = source.evidence(grant)?;
    let objects = source.capture_objects(grant)?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    Ok(RosterSourceReviewObjectsV1 {
        objects: vec![
            parser.canonical_bytes().to_vec(),
            parser.retained().schema_bytes()?,
            parser.retained().configuration_bytes()?,
            parser.retained().dependencies_bytes()?,
            capture.body.as_bytes().to_vec(),
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
    })
}

fn fields() -> TestResult<RosterSourceReviewFieldsV1> {
    Ok(RosterSourceReviewFieldsV1 {
        operation: RosterAuthorityTextV1::parse("controlled-eligible-pr-reviewed-roster")?,
        actor: RosterReviewActorV1::LocalOperator,
        decision: census_store::RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
        reviewed_at: chrono::DateTime::parse_from_rfc3339(AT)?.to_utc(),
        rationale: RosterAuthorityTextV1::parse(
            "Fresh explicitly synthetic eligible PR roster review; not genuine source qualification or production approval",
        )?,
    })
}

fn enroll(store: &Store) -> TestResult {
    let inputs = store.inspect_roster_source_enrollment_v1()?;
    let prepared = store.prepare_roster_source_enrollment_v1(
        inputs,
        RosterEnrollmentReviewV1 {
            operation: RosterAuthorityTextV1::parse("controlled-eligible-pr-roster-enrollment")?,
            actor: RosterReviewActorV1::LocalOperator,
            reviewed_at: chrono::DateTime::parse_from_rfc3339(AT)?.to_utc(),
            rationale: RosterAuthorityTextV1::parse(
                "Fresh synthetic Report control only; not production enrollment approval",
            )?,
        },
    )?;
    let bytes = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_string();
    store.apply_roster_source_enrollment_v1(prepared, &bytes, &digest)?;
    Ok(())
}

fn object(bytes: &[u8]) -> serde_json::Value {
    json!({"digest":hash(bytes),"bytes":bytes.len()})
}

fn proposal(
    capture: &Capture,
    evidence: &RosterSourceEvidenceV1,
    grant: &mut AttachmentReplayGrantV1,
) -> TestResult<LocallyCheckedRosterIssuanceV1> {
    let parser = compiled_roster_parser_manifest()?;
    let checked = assess_retained_roster_parser_manifest(
        parser.canonical_bytes(),
        &parser.parser_claim()?,
        grant,
    )?;
    let objects = prepare_retained_roster_capture_objects_v3(evidence, &checked, grant)?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let owner = SourceSubject::new(SourceNamespace::MilesplitSchool, "38332".into())?;
    let program = SourceSubject::new(SourceNamespace::MilesplitTeam, "38332".into())?;
    let url = capture.packet.roster().requested_url();
    let value = json!({
        "version":1,
        "scope":{"provider":"milesplit","owner":owner,"sport":capture.program.sport,"category":capture.program.gender,"policy":contracts.scope_reference()?,"relationship":"represents_school_program","temporal":"exact_utc_capture_day"},
        "parser":parser.parser_claim()?,
        "capture":{"occurrence":objects.occurrence(grant)?,"body":object(capture.body.as_bytes()),"manifest":object(objects.metadata_bytes(grant)?),"interpretation":object(objects.interpretation_bytes(grant)?),"acquired_at":objects.acquired_at(grant)?,"observed_day":"2026-10-01","requested_url":url,"final_url":{"status":"observed","url":url},"retained_source_admission":object(&evidence.retained_source_admission_manifest(grant)?),"verification_grant":object(&evidence.verification_grant_manifest(grant)?)},
        "binding":{"school":capture.school.id,"program":capture.program.id,"school_source":owner,"program_source":program,"sport":capture.program.sport,"category":capture.program.gender,"school_year":2026,"year_convention":contracts.year_reference()?,"projection_merge_contract":contracts.projection_merge_reference()?,"school_projection":object(&serde_json::to_vec(&capture.school)?),"program_projection":object(&serde_json::to_vec(&capture.program)?),"provider_packet":object(&capture.packet.encode()?),"physical_projection_inventory":object(&evidence.physical_projection_manifest(grant)?),"store":evidence.store_identity(),"snapshot_revision":evidence.snapshot_sequence()},
        "semantic_review_evidence":object(b"controlled proposal replaced by exact prepared source review")
    });
    Ok(decode_roster_issuance_claim(
        &serde_json::to_vec(&value)?,
        RosterAuthorityLimitsV1::new(64 * 1024, 8 * 1024 * 1024)?,
    )?)
}
