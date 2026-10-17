use super::*;
use census_crawl::participation_evidence::roster_source_issuance::manifest::{
    compiled_roster_parser_manifest, RosterParserWorkingTreeV1,
};

pub(super) mod files;

#[test]
fn controlled_reviewed_roster_retains_independent_producer_archive() -> TestResult {
    let Some(destination) = std::env::var_os("CENSUS_REVIEWED_ROSTER_RETENTION_DIR") else {
        return Ok(());
    };
    let destination = PathBuf::from(destination);
    files::fresh_destination(&destination)?;
    let commit = files::revision("CENSUS_REVIEWED_ROSTER_REPORT_PRODUCER_COMMIT")?;
    let tree = files::revision("CENSUS_REVIEWED_ROSTER_REPORT_PRODUCER_TREE")?;
    let parser = compiled_roster_parser_manifest()?;
    check!(eq; parser.git_commit(), commit);
    check!(eq; parser.working_tree(), RosterParserWorkingTreeV1::GitReportedClean);
    std::fs::create_dir(&destination)?;
    files::write(
        &destination.join("parser-manifest.json"),
        parser.canonical_bytes(),
    )?;
    files::write(
        &destination.join("parser-vcs.json"),
        parser.vcs_evidence_bytes(),
    )?;
    let cases = [
        ("resolved", Case::Resolved),
        ("conflicted", Case::Conflicted),
        ("missing-owner", Case::MissingOwner),
    ]
    .into_iter()
    .map(|(name, case)| retain_case(&destination, name, case))
    .collect::<TestResult<Vec<_>>>()?;
    let inventory = files::inventory(&destination)?;
    let manifest = json!({
        "schema": "controlled-reviewed-roster-retention/v1",
        "report_producer_commit": commit,
        "report_producer_tree": tree,
        "parser_manifest": "parser-manifest.json",
        "parser_manifest_sha256": files::digest(parser.canonical_bytes()),
        "parser_vcs": "parser-vcs.json",
        "parser_claim": parser.parser_claim()?,
        "cases": cases,
        "files": inventory,
    });
    let bytes = serde_json::to_vec_pretty(&manifest)?;
    check!(bytes.len() <= 32 * 1024 * 1024);
    check!(eq; files::inventory(&destination)?, inventory);
    let path = destination.join("retention.json");
    files::write(&path, &bytes)?;
    eprintln!(
        "CONTROLLED_REVIEWED_ROSTER_RETENTION {}",
        serde_json::to_string(&json!({
            "manifest": path, "sha256": files::digest(&bytes),
        }))?
    );
    Ok(())
}

fn retain_case(destination: &Path, name: &str, case: Case) -> TestResult<Value> {
    let (fixture, expected) = setup::prepare(case)?;
    let objects = tempfile::tempdir()?;
    let archive = tempfile::tempdir()?;
    let output = tempfile::tempdir()?;
    for source in [
        fixture.directory.path(),
        objects.path(),
        archive.path(),
        output.path(),
    ] {
        files::disjoint(destination, source)?;
    }
    let dataset = capture(&fixture, objects.path())?;
    let before = witness(&dataset, &expected)?;
    let lineage = dataset.lineage.clone();
    let digest = dataset.archive_digest()?;
    let store_identity = fixture.store.store_identity()?;
    let store_format = fixture.store.store_format()?;
    let qualification_lifecycle = dataset.selected_authority()?.qualification_lifecycle();
    let frozen = archive.path().join("controlled-reviewed-source.json");
    dataset.save_frozen(&frozen)?;
    let wire = inspect_wire(&frozen)?;
    let workbook = publish(&fixture, &dataset, &output.path().join("publication"))?;
    let verified = published::verify_published(&workbook)?;
    let rows = artifacts::readback(&workbook, &dataset, &expected)?;
    let generation = workbook
        .parent()
        .ok_or("retention publication generation")?;
    let retained = destination.join(name);
    std::fs::create_dir(&retained)?;
    let archive_files = files::copy(archive.path(), &retained.join("archive"))?;
    let publication_root = retained.join("publication");
    std::fs::create_dir(&publication_root)?;
    let retained_generation =
        publication_root.join(generation.file_name().ok_or("retention generation name")?);
    let publication_files = files::copy(generation, &retained_generation)?;
    let retained_frozen = retained.join("archive/controlled-reviewed-source.json");
    let retained_workbook =
        retained_generation.join(workbook.file_name().ok_or("retention workbook name")?);
    drop(dataset);
    remove_sources(fixture, objects)?;
    archive.close()?;
    output.close()?;
    let immutable = files::inventory(&retained)?;
    let reopened = ExportDataset::reopen_frozen_with_digest(&retained_frozen, &digest)?;
    check!(eq; reopened.lineage, lineage);
    check!(eq; reopened.archive_digest()?, digest);
    check!(eq; witness(&reopened, &expected)?, before);
    let publication = published::open_publication(
        &retained_workbook,
        &lineage,
        Scope::AllSources,
        GradYear::CO2027,
    )?;
    check!(eq; witness(&publication.dataset, &expected)?, before);
    check!(eq; artifacts::readback(&retained_workbook, &publication.dataset, &expected)?, rows);
    check!(eq; published::verify_published(&retained_workbook)?.generation_digest, verified.generation_digest);
    check!(eq; files::inventory(&retained)?, immutable);
    Ok(json!({
        "case": name,
        "archive": format!("{name}/archive/controlled-reviewed-source.json"),
        "workbook": retained_workbook.strip_prefix(destination)?.to_str().ok_or("retention workbook UTF-8")?,
        "archive_digest": digest,
        "publication_generation_digest": verified.generation_digest,
        "store_identity": store_identity,
        "source_store_lifecycle": store_format.lifecycle_revision,
        "qualification_lifecycle": qualification_lifecycle,
        "selected_proof_version": wire.pointer("/selected_proofs/version").ok_or("selected proof version")?,
        "affiliation_proof_version": wire.pointer("/selected_proofs/affiliation/version").ok_or("affiliation proof version")?,
        "lineage": lineage,
        "expected_athlete": expected.athlete,
        "expected_school": expected.school(),
        "expected_status": expected.status(),
        "expected_selected_count": expected.selected_count(),
        "observations": before,
        "publication_rows": rows,
        "archive_files": archive_files,
        "publication_files": publication_files,
    }))
}
