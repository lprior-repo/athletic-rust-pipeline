//! The pre-Fjall resume ledger's import: `<root>/journal/<phase>.jsonl`, one entry per finished unit
//! of work, under the same policy as the entity journals.

use fjall::PersistMode;
use std::io::BufReader;

use super::super::{Store, StoreError, StoreResult};
use super::cursor::{read_legacy_line, LineEnding};

impl Store {
    /// Import the pre-Fjall resume ledger: one `<root>/journal/<phase>.jsonl` per phase, each line a
    /// JSON object naming the unit of work it recorded (`key`) and what that work produced.
    ///
    /// The policy is the entity journals' policy, because the stakes are the same: this ledger is the
    /// store's only record that a unit of work finished, so an entry that vanishes becomes work a
    /// later run repeats. A complete line imports, byte for byte. A line that is not a JSON object, or
    /// carries no `key`, fails the import naming the file and the line: it is either corruption or a
    /// writer's bug, and passing over it would report work the file recorded as work that never
    /// happened. One unterminated trailing line is the head of a line a writer died in the middle of,
    /// which was never an entry: the import stops at the line boundary before it.
    ///
    /// The completion mark rides in the same batch as the entries it describes, so the two land
    /// together or not at all: an import that refused on any line leaves no mark and runs again,
    /// while an import that finished is never repeated.
    pub(super) fn import_legacy_resume_journals(&self) -> StoreResult<()> {
        let dir = self.root.join("journal");
        if !dir.exists() {
            return Ok(());
        }
        if self
            .meta
            .contains_key("imported:resume-journals")
            .map_err(|source| StoreError::Read { source })?
        {
            return Ok(());
        }
        let entries = std::fs::read_dir(&dir).map_err(|source| StoreError::Io {
            path: dir.clone(),
            source,
        })?;
        let mut batch = self.db.batch();
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io {
                path: dir.clone(),
                source,
            })?;
            let path = entry.path();
            let Some(phase) = path.file_stem().and_then(|stem| stem.to_str()) else {
                return Err(StoreError::Legacy {
                    detail: format!("{} names no phase to key its entries by", path.display()),
                });
            };
            let file = std::fs::File::open(&path).map_err(|source| StoreError::Io {
                path: path.clone(),
                source,
            })?;
            let mut reader = BufReader::new(file);
            let mut line = Vec::new();
            let mut line_no = 0_u64;
            loop {
                line.clear();
                line_no = line_no.saturating_add(1);
                let Some(ending) = read_legacy_line(&mut reader, &mut line, &path, line_no)? else {
                    break;
                };
                if ending == LineEnding::Truncated {
                    tracing::warn!(
                        path = %path.display(),
                        line = line_no,
                        "stopped at a legacy resume-journal line its writer never finished"
                    );
                    break;
                }
                let trimmed = line.trim_ascii();
                if trimmed.is_empty() {
                    continue;
                }
                let value: serde_json::Value =
                    serde_json::from_slice(trimmed).map_err(|error| StoreError::Legacy {
                        detail: format!("{} line {line_no}: {error}", path.display()),
                    })?;
                let key = value
                    .get("key")
                    .and_then(|key| key.as_str())
                    .ok_or_else(|| StoreError::Legacy {
                        detail: format!(
                            "{} line {line_no}: a resume-journal entry carries no key",
                            path.display()
                        ),
                    })?;
                batch.insert(&self.journal, Self::journal_key(phase, key), trimmed);
            }
        }
        batch.insert(&self.meta, "imported:resume-journals", b"1".as_slice());
        batch
            .durability(Some(PersistMode::SyncData))
            .commit()
            .map_err(|source| StoreError::Write { source })
    }
}
