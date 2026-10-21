use super::*;
use crate::participation_evidence::roster_source_issuance::manifest::compiled_roster_parser_manifest;
use crate::participation_evidence::roster_source_issuance::source::tests::integration::setup::{
    enrolled_store, install,
};
use census_store::Store;

mod legacy;
mod observed_charge;
mod snapshot;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

#[test]
fn raw_metadata_and_capture_interpretation_keep_exact_legacy_bytes_after_cold_reopen() -> TestResult
{
    let (_directory, store) = enrolled_store()?;
    let (packet, _school, _program, request) = install(&store)?;
    let compiled = compiled_roster_parser_manifest()?;
    let expected = legacy::expected(&packet, &request.root, compiled.canonical_bytes())?;
    let mut retained = Vec::new();
    for row_local in [false, true] {
        let actual = snapshot::read(&store, &request, &compiled, row_local)?;
        check!(eq; actual.metadata, expected.metadata);
        check!(eq; actual.occurrence, hash(&expected.metadata));
        check!(eq; actual.acquired_at, expected.acquired_at);
        legacy::require_interpretation(
            &actual.interpretation,
            &expected.interpretation,
            row_local,
        )?;
        retained.push(actual);
    }
    let root = store.root().to_owned();
    drop(store);
    let (store, _writer) = Store::open_public_identity_qualification_v3(&root)?.into_parts();
    for (row_local, previous) in [false, true].into_iter().zip(retained) {
        let actual = snapshot::read(&store, &request, &compiled, row_local)?;
        check!(eq; actual, previous);
    }
    Ok(())
}

#[test]
fn raw_capture_metadata_precharge_still_refuses_an_exhausted_original_grant() -> TestResult {
    let (_directory, store) = enrolled_store()?;
    let (_packet, _school, _program, request) = install(&store)?;
    let compiled = compiled_roster_parser_manifest()?;
    let mut inspection =
        store.inspect_roster_source_issuance_v1(request, AttachmentReplayGrantV1::new())?;
    let (evidence, grant) = inspection.source_parts()?;
    let parser = super::super::manifest::assess_retained_roster_parser_manifest(
        compiled.canonical_bytes(),
        &compiled.parser_claim()?,
        grant,
    )?;
    let remaining = AttachmentReplayGrantV1::MAX_ENCODED_BYTES
        .checked_sub(grant.usage().encoded_bytes())
        .ok_or("grant usage")?;
    grant.charge_bytes(remaining.checked_sub(1).ok_or("grant remaining")?)?;
    check!(version3::prepare_retained_roster_capture_objects_v3(evidence, &parser, grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}
