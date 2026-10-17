use super::*;
use crate::participation_evidence::roster_source_issuance::{
    assess_existing_roster_binding, manifest::compiled_roster_parser_manifest,
    policies::compiled_roster_interpretation_contracts_v1,
};
use census_domain::roster_affiliation_authority::RosterAuthorityTextV1;
use census_store::{
    Application, PreparedRosterSourceReviewV1, RosterReviewActorV1, RosterSourceReviewFieldsV1,
    RosterSourceReviewObjectsV1, Store,
};

#[path = "source_integration_setup.rs"]
mod setup;
use setup::*;

fn fields(operation: &str) -> TestResult<RosterSourceReviewFieldsV1> {
    Ok(RosterSourceReviewFieldsV1 {
        operation: RosterAuthorityTextV1::parse(operation)?,
        actor: RosterReviewActorV1::LocalOperator,
        decision:
            census_store::RosterSourceReviewDecisionV1::QualifyAndActivateExactCaptureRepresentation,
        reviewed_at: DateTime::parse_from_rfc3339("2026-10-16T00:49:43Z")?.to_utc(),
        rationale: RosterAuthorityTextV1::parse(
            "Controlled local reviewed claims; no authority grant",
        )?,
    })
}

pub(super) fn prepare<'a>(
    store: &'a Store,
    operation: &str,
    extra_bytes: Option<usize>,
) -> TestResult<PreparedRosterSourceReviewV1<'a>> {
    let (packet, school, program, request) = install(store)?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal(&packet, &school, &program, evidence, grant)?;
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
    let capture = source.capture_objects(grant)?;
    let mut objects = vec![
        parser.canonical_bytes().to_vec(),
        parser.retained().schema_bytes()?,
        parser.retained().configuration_bytes()?,
        parser.retained().dependencies_bytes()?,
        packet.roster().body()?.into_bytes(),
        capture.metadata_bytes(grant)?.to_vec(),
        capture.interpretation_bytes(grant)?.to_vec(),
        packet.encode()?,
        serde_json::to_vec(binding.school(grant)?)?,
        serde_json::to_vec(binding.program(grant)?)?,
        evidence.physical_projection_manifest(grant)?,
        evidence.retained_source_admission_manifest(grant)?,
        evidence.verification_grant_manifest(grant)?,
    ];
    if let Some(size) = extra_bytes {
        objects.push(vec![b'x'; size]);
    }
    let supplied = RosterSourceReviewObjectsV1 {
        objects,
        scope_policy: contracts.scope.to_vec(),
        year_convention: contracts.year.to_vec(),
        projection_merge_contract: contracts.projection_merge.to_vec(),
    };
    Ok(
        store.prepare_roster_source_review_v1(
            inspection,
            proposal,
            fields(operation)?,
            supplied,
        )?,
    )
}

#[test]
fn controlled_review_applies_replays_and_reopens_without_authority_claim() -> TestResult {
    let (directory, store) = enrolled_store()?;
    let mut prepared = prepare(&store, "controlled-review", None)?;
    let reviewed = decode(&serde_json::from_slice(prepared.reviewed_proposal_bytes())?)?;
    check!(reviewed.semantic_review_evidence().bytes().get() > 64);
    let parser = compiled_roster_parser_manifest()?;
    let contracts = compiled_roster_interpretation_contracts_v1();
    let (evidence, grant) = prepared.source_parts()?;
    let source = assess_retained_roster_source(
        evidence,
        parser.canonical_bytes(),
        &contracts,
        &reviewed,
        grant,
    )?;
    assess_existing_roster_binding(&source, grant)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let applied = store.apply_roster_source_review_v1(prepared, &approved, &digest)?;
    check!(applied.written());
    check!(eq; applied.receipt().appended, 0);
    check!(!applied.receipt().credited);
    let repeated = store.replay_roster_source_review_v1("controlled-review", &digest)?;
    check!(repeated.repeated());
    check!(eq; repeated.receipt(), applied.receipt());
    check!(store
        .replay_roster_source_review_v1("controlled-review", &"d".repeat(64))
        .is_err());
    let later = prepare(&store, "later-controlled-review", None)?;
    let later_approved = later.approval_bytes().to_vec();
    let later_digest = later.approval_digest().to_owned();
    let later_applied =
        store.apply_roster_source_review_v1(later, &later_approved, &later_digest)?;
    check!(later_applied.written());
    let original = store.replay_roster_source_review_v1("controlled-review", &digest)?;
    check!(original.repeated());
    check!(eq; original.receipt(), applied.receipt());
    drop(store);
    let (reopened, _writer) =
        Store::open_public_identity_qualification_v3(directory.path().join("controlled-roster"))?
            .into_parts();
    let historical = reopened.replay_roster_source_review_v1("controlled-review", &digest)?;
    check!(historical.repeated());
    check!(eq; historical.receipt(), applied.receipt());
    let latest =
        reopened.replay_roster_source_review_v1("later-controlled-review", &later_digest)?;
    check!(latest.repeated());
    check!(eq; latest.receipt(), later_applied.receipt());
    Ok(())
}

#[test]
fn controlled_review_refuses_changed_approval_and_foreign_store() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let prepared = prepare(&store, "wrong-approval", None)?;
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_review_v1(prepared, b"{}", &digest)
        .is_err());
    check!(store
        .replay_roster_source_review_v1("wrong-approval", &digest)
        .is_err());
    let prepared = prepare(&store, "wrong-approved-digest", None)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    check!(store
        .apply_roster_source_review_v1(prepared, &approved, &"d".repeat(64))
        .is_err());
    check!(store
        .replay_roster_source_review_v1("wrong-approved-digest", &digest)
        .is_err());
    let prepared = prepare(&store, "foreign-owner", None)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    let (_foreign_directory, foreign) = enrolled_store()?;
    check!(foreign
        .apply_roster_source_review_v1(prepared, &approved, &digest)
        .is_err());
    check!(store
        .replay_roster_source_review_v1("foreign-owner", &digest)
        .is_err());
    Ok(())
}

#[test]
fn controlled_review_refuses_stale_prepared_basis() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let prepared = prepare(&store, "stale-basis", None)?;
    let approved = prepared.approval_bytes().to_vec();
    let digest = prepared.approval_digest().to_owned();
    install(&store)?;
    check!(store
        .apply_roster_source_review_v1(prepared, &approved, &digest)
        .is_err());
    check!(store
        .replay_roster_source_review_v1("stale-basis", &digest)
        .is_err());
    Ok(())
}

#[test]
fn controlled_review_reopens_supplied_objects_at_and_above_record_limit() -> TestResult {
    for bytes in [64 * 1024, 64 * 1024 + 1] {
        let (directory, store) = enrolled_store()?;
        let prepared = prepare(&store, "large-extra-object", Some(bytes))?;
        let approved = prepared.approval_bytes().to_vec();
        let digest = prepared.approval_digest().to_owned();
        check!(matches!(
            store.apply_roster_source_review_v1(prepared, &approved, &digest)?,
            Application::Written(_)
        ));
        drop(store);
        let (reopened, _writer) = Store::open_public_identity_qualification_v3(
            directory.path().join("controlled-roster"),
        )?
        .into_parts();
        check!(matches!(
            reopened.replay_roster_source_review_v1("large-extra-object", &digest)?,
            Application::Repeated(_)
        ));
    }
    Ok(())
}
