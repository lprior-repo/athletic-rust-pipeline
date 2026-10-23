use super::{invalid, io_error, present, require_directory, ReportResult};
use std::path::{Path, PathBuf};

pub(super) enum Preparation {
    New,
    EmptyExisting,
}

pub(super) enum Input {
    Prepare(Preparation),
    Reopen,
}

pub(super) fn preparation(pending: &Path, mode: super::super::InputMode) -> ReportResult<Input> {
    if !present(pending)? {
        if mode == super::super::InputMode::RecoverExisting {
            return Err(invalid(
                "recovering publication requires an existing export job",
            ));
        }
        return Ok(Input::Prepare(Preparation::New));
    }
    if mode == super::super::InputMode::RecoverExisting
        && !present(&pending.join("commitment.json"))?
    {
        return Ok(Input::Prepare(Preparation::EmptyExisting));
    }
    Ok(Input::Reopen)
}

pub(super) fn directories(pending: &Path, stage: Preparation) -> ReportResult<PathBuf> {
    let objects = super::super::super::proof_directory(&pending.join("input.json"))?;
    match stage {
        Preparation::New => {
            std::fs::create_dir(pending).map_err(|source| io_error(pending, source))?;
            super::super::sync_directory(
                pending
                    .parent()
                    .ok_or_else(|| invalid("pending selected job has no parent"))?,
            )?;
            std::fs::create_dir(&objects).map_err(|source| io_error(&objects, source))?;
        }
        Preparation::EmptyExisting => require_empty(pending, &objects)?,
    }
    Ok(objects)
}

fn require_empty(pending: &Path, objects: &Path) -> ReportResult<()> {
    require_directory(pending)?;
    let mut entries = std::fs::read_dir(pending).map_err(|source| io_error(pending, source))?;
    let first = entries
        .next()
        .transpose()
        .map_err(|source| io_error(pending, source))?;
    let second = entries
        .next()
        .transpose()
        .map_err(|source| io_error(pending, source))?;
    if first.is_none_or(|entry| entry.file_name() != "input.json.proofs") || second.is_some() {
        return Err(invalid(
            "uncommitted selected staging is not exactly empty proof storage",
        ));
    }
    require_directory(objects)?;
    if std::fs::read_dir(objects)
        .map_err(|source| io_error(objects, source))?
        .next()
        .transpose()
        .map_err(|source| io_error(objects, source))?
        .is_some()
    {
        return Err(invalid(
            "uncommitted selected staging contains retained proof objects",
        ));
    }
    Ok(())
}
