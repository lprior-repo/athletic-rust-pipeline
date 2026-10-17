use super::*;
use authentication::{string, Retained};
use census_crawl::participation_evidence::roster_source_issuance::manifest::{
    assess_retained_roster_parser_manifest, compiled_roster_parser_manifest,
    RetainedRosterParserManifestV1, RosterParserWorkingTreeV1,
};

pub(super) fn different_current_build(retained: &Retained) -> TestResult {
    let current = compiled_roster_parser_manifest()?;
    check!(eq; current.working_tree(), RosterParserWorkingTreeV1::GitReportedClean);
    check!(eq; current.git_commit(), files::revision("CENSUS_REVIEWED_ROSTER_REPORT_CONSUMER_COMMIT")?);
    check!(ne; current.git_commit(), string(&retained.manifest, "report_producer_commit")?);
    check!(ne; current.canonical_bytes(), retained.parser);
    let old = RetainedRosterParserManifestV1::decode(&retained.parser)?;
    let claim = old.parser_claim()?;
    check!(eq; serde_json::to_value(&claim)?, *retained.manifest.get("parser_claim").ok_or("retained parser claim")?);
    check!(current.require_claim(&claim).is_err());
    let original: Value = serde_json::from_slice(&retained.parser)?;
    let compiled: Value = serde_json::from_slice(current.canonical_bytes())?;
    let previous = original
        .get("source_trees")
        .and_then(Value::as_array)
        .ok_or("producer source closure")?;
    let next = compiled
        .get("source_trees")
        .and_then(Value::as_array)
        .ok_or("consumer source closure")?;
    check!(eq; previous.len(), 3);
    check!(eq; next.len(), 3);
    let mut changed = false;
    for (path, (before, after)) in [
        "crates/census-crawl/src",
        "crates/census-domain/src",
        "crates/census-store/src",
    ]
    .into_iter()
    .zip(previous.iter().zip(next))
    {
        check!(eq; string(before, "path")?, path);
        check!(eq; string(after, "path")?, path);
        changed |= string(before, "sha256")? != string(after, "sha256")?;
    }
    check!(
        changed,
        "consumer must have genuinely changed parser source closure"
    );
    let mut grant = AttachmentReplayGrantV1::new();
    check!(assess_retained_roster_parser_manifest(&retained.parser, &claim, &mut grant).is_err());
    check!(grant.ensure_available().is_err());
    Ok(())
}

pub(super) fn old_claim_refused_by_current(archive: &Path, manifest: &Value) -> TestResult {
    let wire = inspect_wire(archive)?;
    let objects = archive.with_file_name(format!(
        "{}.proofs",
        archive
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or("archive filename")?
    ));
    let closure = read_object(
        &objects,
        wire.pointer("/selected_proofs/affiliation/archives")
            .ok_or("source closure")?,
    )?;
    let closure: Value = serde_json::from_slice(&closure)?;
    let reference: census_store::RosterFrozenReviewedClaimsReferenceV1 =
        serde_json::from_value(closure.get("reviewed").ok_or("reviewed reference")?.clone())?;
    let source = read_object(
        &objects,
        closure.get("reviewed").ok_or("reviewed reference")?,
    )?;
    let mut grant = AttachmentReplayGrantV1::new();
    let replayed =
        census_store::replay_frozen_roster_reviewed_claims_v1(&source, &reference, &mut grant)?;
    let claim = replayed
        .claims(&mut grant)?
        .first()
        .ok_or("nonempty original reviewed source")?;
    check!(eq; serde_json::to_value(claim.proposal(&mut grant)?.parser())?, *manifest.get("parser_claim").ok_or("original parser claim")?);
    let result = census_crawl::participation_evidence::roster_source_issuance::assess_reviewed_roster_source_v1(claim, &mut grant);
    let error = match result {
        Ok(_) => return Err("old source provenance accepted by current assessment".into()),
        Err(error) => error,
    };
    check!(
        matches!(error, census_crawl::CrawlError::Store(census_store::StoreError::Refused { detail }) if detail == "retained roster parser source provenance has no supported adapter")
    );
    check!(grant.ensure_available().is_err());
    Ok(())
}

fn read_object(objects: &Path, reference: &Value) -> TestResult<Vec<u8>> {
    let digest = string(reference, "digest")?;
    check!(
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    let bytes = files::read(&authentication::relative(objects, digest)?)?;
    check!(eq; u64::try_from(bytes.len())?, reference.get("bytes").and_then(Value::as_u64).ok_or("object size")?);
    check!(eq; files::digest(&bytes), digest);
    Ok(bytes)
}

pub(super) fn current_qualification() -> TestResult {
    let (fixture, expected) = setup::prepare(Case::Resolved)?;
    let objects = tempfile::tempdir()?;
    let dataset = capture(&fixture, objects.path())?;
    witness(&dataset, &expected)?;
    check!(eq; expected.selected_count(), 1);
    let mut grant = AttachmentReplayGrantV1::new();
    let claims = fixture
        .store
        .snapshot()
        .read_current_roster_reviewed_claims_v1(&mut grant)?;
    check!(eq; claims.claims(&mut grant)?.len(), 1);
    let claim = claims
        .claims(&mut grant)?
        .first()
        .ok_or("fresh current source claim")?;
    let compiled = compiled_roster_parser_manifest()?;
    compiled.require_claim(claim.proposal(&mut grant)?.parser())?;
    Ok(())
}
