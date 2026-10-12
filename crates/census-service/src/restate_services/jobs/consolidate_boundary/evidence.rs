use std::io::{BufReader, Read};
use std::path::Path;

use census_store::Store;
use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::restate_services::wire::ConsolidatedTable;

use super::{io, BoundaryError};

const DIGEST_BUFFER_BYTES: usize = 64 * 1024;

#[derive(Serialize)]
pub(super) struct TableEvidence {
    table: String,
    rows: usize,
    digest: String,
}

pub(super) fn gather(
    store: &Store,
    tables: &[ConsolidatedTable],
) -> Result<Vec<TableEvidence>, BoundaryError> {
    let out = store.out_dir();
    tables.iter().try_fold(
        Vec::new(),
        |mut evidence, table| -> Result<Vec<TableEvidence>, BoundaryError> {
            evidence
                .try_reserve(1)
                .map_err(|_| BoundaryError::Allocation)?;
            let path = out.join(format!("{}.jsonl", table.table));
            evidence.push(TableEvidence {
                table: table.table.clone(),
                rows: table.rows,
                digest: digest_file(&path)?,
            });
            Ok(evidence)
        },
    )
}

fn digest_file(path: &Path) -> Result<String, BoundaryError> {
    let file = std::fs::File::open(path)
        .map_err(|source| io("reading a consolidated snapshot", path, source))?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; DIGEST_BUFFER_BYTES];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|source| io("reading a consolidated snapshot", path, source))?;
        if read == 0 {
            break;
        }
        let chunk = buffer.get(..read).ok_or(BoundaryError::Digest)?;
        hasher.update(chunk);
    }
    Ok(format!("{:x}", hasher.finalize()))
}
