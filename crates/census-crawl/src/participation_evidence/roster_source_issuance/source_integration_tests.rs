use super::*;
use crate::participation_evidence::roster_source_issuance::assess_existing_roster_binding;
use crate::participation_evidence::roster_source_issuance::manifest::compiled_roster_parser_manifest;
use crate::participation_evidence::roster_source_issuance::policies::compiled_roster_interpretation_contracts_v1;
use census_store::RosterIssuanceInspectionRequestV1;

#[path = "source_integration_setup.rs"]
pub(in crate::participation_evidence::roster_source_issuance) mod setup;
use setup::*;

#[test]
fn controlled_public_inspection_binds_exact_rows_and_refuses_foreign_grant() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal_v3(&packet, &school, &program, evidence, grant)?;
    let compiled = compiled_roster_parser_manifest()?;
    let checked = crate::participation_evidence::roster_source_issuance::manifest::assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(), &compiled.parser_claim()?, grant)?;
    let usage = grant.usage();
    let mut foreign = AttachmentReplayGrantV1::new();
    foreign.charge_bytes(usage.encoded_bytes())?;
    foreign.charge_steps(usage.traversal_steps())?;
    let original = std::mem::replace(grant, foreign);
    check!(crate::participation_evidence::roster_source_issuance::prepare_retained_roster_capture_objects_v3(
        evidence, &checked, grant).is_err());
    check!(assess_retained_roster_source_v3(
        evidence,
        compiled.canonical_bytes(),
        &compiled_roster_interpretation_contracts_v1(),
        &proposal,
        grant
    )
    .is_err());
    *grant = original;
    let source = assess_retained_roster_source_v3(
        evidence,
        compiled.canonical_bytes(),
        &compiled_roster_interpretation_contracts_v1(),
        &proposal,
        grant,
    )?;
    let binding = assess_existing_roster_binding(&source, grant)?;
    check!(eq; binding.school(grant)?.id, school.id);
    check!(eq; binding.program(grant)?.id, program.id);
    check!(std::ptr::eq(binding.evidence(grant)?, evidence));
    Ok(())
}

#[test]
fn controlled_inspection_rejects_substituted_parser_and_context_references() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    for path in [
        "/scope/policy/digest",
        "/binding/year_convention/digest",
        "/binding/projection_merge_contract/digest",
        "/capture/manifest/digest",
        "/capture/interpretation/digest",
        "/capture/occurrence",
        "/capture/retained_source_admission/digest",
        "/capture/verification_grant/digest",
        "/parser/artifact/digest",
        "/parser/schema/digest",
        "/parser/configuration/digest",
        "/parser/dependencies/digest",
    ] {
        let root = request.root.clone();
        let selected = RosterIssuanceInspectionRequestV1 {
            root,
            school: school.id.clone(),
            program: program.id.clone(),
        };
        let mut inspection =
            store.inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())?;
        let (evidence, grant) = inspection.source_parts()?;
        let proposal = actual_proposal_v3(&packet, &school, &program, evidence, grant)?;
        let compiled = compiled_roster_parser_manifest()?;
        let mut value = serde_json::to_value(&proposal)?;
        *value.pointer_mut(path).ok_or("context reference absent")? = json!("d".repeat(64));
        let changed = decode(&value)?;
        check!(assess_retained_roster_source_v3(
            evidence,
            compiled.canonical_bytes(),
            &compiled_roster_interpretation_contracts_v1(),
            &changed,
            grant
        )
        .is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[test]
fn controlled_inspection_refuses_wrong_retained_parser_manifest() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let proposal = actual_proposal_v3(&packet, &school, &program, evidence, grant)?;
    check!(assess_retained_roster_source_v3(
        evidence,
        b"{}",
        &compiled_roster_interpretation_contracts_v1(),
        &proposal,
        grant
    )
    .is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

#[test]
fn controlled_inspection_refuses_changed_requested_school_or_program() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (packet, school, program, request) = install(&store)?;
    for change_school in [true, false] {
        let selected = RosterIssuanceInspectionRequestV1 {
            root: request.root.clone(),
            school: if change_school {
                census_domain::model::SchoolId::mint("sch", &["other"])
            } else {
                school.id.clone()
            },
            program: if change_school {
                program.id.clone()
            } else {
                census_domain::model::TeamId::mint("team", &["other"])
            },
        };
        let mut inspection =
            store.inspect_roster_source_issuance_v1(selected, AttachmentReplayGrantV1::new())?;
        let (evidence, grant) = inspection.source_parts()?;
        let proposal = actual_proposal_v3(&packet, &school, &program, evidence, grant)?;
        let compiled = compiled_roster_parser_manifest()?;
        check!(assess_retained_roster_source_v3(
            evidence,
            compiled.canonical_bytes(),
            &compiled_roster_interpretation_contracts_v1(),
            &proposal,
            grant
        )
        .is_err());
        check!(grant.ensure_available().is_err());
    }
    Ok(())
}

#[path = "source_integration_capture_tests.rs"]
mod capture_objects;
