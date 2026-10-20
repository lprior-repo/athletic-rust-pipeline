use super::frozen::FrozenRevision;
use super::*;
use std::fs;

pub(super) fn proof_refusal(revision: &FrozenRevision) -> TestResult {
    let proofs_dir = {
        let parent = revision
            .path
            .parent()
            .ok_or("frozen path has no parent directory")?;
        let name = revision
            .path
            .file_name()
            .ok_or("frozen path has no file name")?
            .to_str()
            .ok_or("frozen file name is not UTF-8")?;
        parent.join(format!("{}.proofs", name))
    };
    let payload: Value = serde_json::from_slice(&fs::read(&revision.path)?)?;

    for kind in ["archives", "roster_family"] {
        let path = {
            let digest = payload["selected_proofs"]["affiliation"][kind]["digest"]
                .as_str()
                .ok_or("mandatory affiliation proof digest missing")?;
            proofs_dir.join(digest)
        };
        probe(&path, revision)?;
    }

    let reopened = ExportDataset::reopen_frozen_with_digest(&revision.path, &revision.digest)?;
    check!(eq; reopened.archive_digest()?, revision.digest);
    check!(eq; reopened.lineage, revision.lineage);
    Ok(())
}

fn probe(path: &Path, revision: &FrozenRevision) -> TestResult {
    let mut bytes = fs::read(path)?;
    fs::remove_file(path)?;
    let r = ExportDataset::reopen_frozen_with_digest(&revision.path, &revision.digest);
    if r.is_ok() {
        return Err("cold reopen succeeded despite missing proof".into());
    }
    fs::write(path, &bytes)?;

    *bytes.first_mut().ok_or("empty mandatory proof")? ^= 1;
    fs::write(path, &bytes)?;
    let r = ExportDataset::reopen_frozen_with_digest(&revision.path, &revision.digest);
    if r.is_ok() {
        return Err("cold reopen succeeded despite tampered proof".into());
    }
    *bytes.first_mut().ok_or("empty mandatory proof")? ^= 1;
    fs::write(path, &bytes)?;
    Ok(())
}
