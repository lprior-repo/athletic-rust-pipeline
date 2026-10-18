use std::env;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use std::process;

use census_crawl::milesplit::SourceRowLocator;
use census_crawl::participation_evidence::recorded::discover_frozen_html_results;
use census_crawl::participation_evidence::result_ownership::school_correspondence::read;
use census_domain::evidence_subject_attachment::{
    AttachmentObjectReferenceV1, AttachmentReplayGrantV1,
};
use census_domain::model::{SchoolId, StoreIdentity};
use census_store::{replay_result_school_source_proof_v1, ReplayedResultSchoolCorrespondencesV1};
use serde::Deserialize;

const MANIFEST_MAX: usize = 64 * 1024;
const PROOF_MAX: usize = 4 * 1024 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u64,
    stage: String,
    store: StoreIdentity,
    snapshot_sequence: u64,
    proof_ref: AttachmentObjectReferenceV1,
    proof_file: String,
    expected_school: Option<SchoolId>,
    result_context: String,
    result_locator: SourceRowLocator,
}

fn validate_manifest(manifest: &Manifest) -> Result<(), String> {
    if manifest.version != 1 {
        return Err(format!("unsupported manifest version {}", manifest.version));
    }
    match (manifest.stage.as_str(), &manifest.expected_school) {
        ("accepted", Some(_)) | ("revoked", None) => {}
        _ => return Err("stage and expected school disagree".to_string()),
    }
    let name = &manifest.proof_file;
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err("proof_file must be a non-trivial basename".to_string());
    }
    Ok(())
}

fn read_bounded(path: &Path, limit: usize) -> Result<Vec<u8>, String> {
    let mut file = File::open(path).map_err(|error| format!("open failed: {error}"))?;
    let metadata = file
        .metadata()
        .map_err(|error| format!("metadata failed: {error}"))?;
    if !metadata.is_file() {
        return Err("input is not a regular file".to_string());
    }
    let length = usize::try_from(metadata.len()).map_err(|_| "file length overflow".to_string())?;
    if length > limit {
        return Err(format!("file length {length} exceeds limit {limit}"));
    }
    let mut buffer = Vec::new();
    buffer
        .try_reserve_exact(length)
        .map_err(|error| format!("reserve failed: {error}"))?;
    buffer.resize(length, 0);
    file.read_exact(&mut buffer)
        .map_err(|error| format!("read failed: {error}"))?;
    let mut extra = [0u8; 1];
    if file
        .read(&mut extra)
        .map_err(|error| format!("growth check failed: {error}"))?
        != 0
    {
        return Err("file grew during read".to_string());
    }
    Ok(buffer)
}

fn replay_manifest(path: &Path) -> Result<Option<SchoolId>, String> {
    let bytes = read_bounded(path, MANIFEST_MAX)?;
    let manifest: Manifest = serde_json::from_slice(&bytes)
        .map_err(|error| format!("manifest decode failed: {error}"))?;
    validate_manifest(&manifest)?;
    let proof_path = path
        .parent()
        .ok_or("manifest has no directory")?
        .join(&manifest.proof_file);
    let proof_bytes = read_bounded(&proof_path, PROOF_MAX)?;
    let mut grant = AttachmentReplayGrantV1::new();
    let current = replay_result_school_source_proof_v1(
        &proof_bytes,
        &manifest.proof_ref,
        &manifest.store,
        manifest.snapshot_sequence,
        &mut grant,
    )
    .map_err(|error| format!("replay failed: {error}"))?;
    let observed = selected_school(&current, &manifest, &mut grant)?;
    if manifest.expected_school.as_ref() != observed.as_ref() {
        return Err(format!(
            "school mismatch: observed {observed:?}, expected {:?}",
            manifest.expected_school
        ));
    }
    println!(
        "stage={} school={observed:?} grant={:?}",
        manifest.stage,
        grant.usage()
    );
    Ok(observed)
}

fn selected_school(
    current: &ReplayedResultSchoolCorrespondencesV1<'_>,
    manifest: &Manifest,
    grant: &mut AttachmentReplayGrantV1,
) -> Result<Option<SchoolId>, String> {
    let attachments = current
        .attachments(grant)
        .map_err(|error| format!("attachment owner failed: {error}"))?;
    let plan = discover_frozen_html_results(attachments, grant)
        .map_err(|error| format!("frozen discovery failed: {error}"))?;
    let mut row_found = false;
    let mut observed = None;
    for group in 0..plan.group_count() {
        let result = plan
            .with_group(group, grant, |linked, grant| {
                let mut found_index = None;
                for index in 0..linked.row_count() {
                    if let Some(row) = linked.row(index) {
                        let checked = row.checked_row();
                        if checked.locator() == manifest.result_locator
                            && checked.context_reference() == manifest.result_context
                        {
                            if found_index.is_some() {
                                return Err(census_crawl::CrawlError::Schema {
                                    url: manifest.result_context.clone(),
                                    detail: "duplicate retained locator/context".to_string(),
                                });
                            }
                            found_index = Some(index);
                        }
                    }
                }
                match found_index {
                    Some(index) => {
                        let checked = read::checked_frozen(current, &linked, index, grant)?;
                        Ok((true, checked.map(|value| value.school().clone())))
                    }
                    None => Ok((false, None)),
                }
            })
            .map_err(|error| format!("frozen group {group} failed: {error}"))?;
        let (found, school) = result;
        if found {
            if row_found {
                return Err("duplicate retained locator/context across groups".to_string());
            }
            row_found = true;
            observed = school;
        }
    }
    if !row_found {
        return Err("no exact retained locator/context match".to_string());
    }
    Ok(observed)
}

fn main() {
    let mut args = env::args_os();
    let _program = args.next();
    let first = args.next();
    let second = args.next();
    if first.is_none() || args.next().is_some() {
        eprintln!("usage: smoke_result_school_proof <manifest.json> [<manifest2.json>]");
        process::exit(2);
    }
    let mut failed = false;
    for argument in [first, second].into_iter().flatten() {
        let path = Path::new(&argument);
        match replay_manifest(path) {
            Ok(school) => println!("verified {}: {school:?}", path.display()),
            Err(error) => {
                eprintln!("failed {}: {error}", path.display());
                failed = true;
            }
        }
    }
    if failed {
        process::exit(1);
    }
}
