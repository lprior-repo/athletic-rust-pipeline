use super::fixture::TestResult;
use fjall::Readable;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct LogicalImage {
    sequence: u64,
    keyspaces: Vec<Vec<Row>>,
}

#[derive(Debug, PartialEq, Eq)]
struct Row {
    key: Vec<u8>,
    value: Vec<u8>,
}

pub(super) fn logical_image(root: &Path) -> TestResult<LogicalImage> {
    let db = fjall::Database::builder(root.join("fjall")).open()?;
    let snapshot = db.snapshot();
    let mut keyspaces = Vec::new();
    for name in ["entities", "journal", "meta", "receipts"] {
        let keyspace = db.keyspace(name, fjall::KeyspaceCreateOptions::default)?;
        let mut rows = Vec::new();
        for guard in snapshot.prefix(&keyspace, b"").take(513) {
            if rows.len() == 512 {
                return Err("logical image exceeds fixture row bound".into());
            }
            let (key, value) = guard.into_inner()?;
            rows.push(Row {
                key: key.to_vec(),
                value: value.to_vec(),
            });
        }
        keyspaces.push(rows);
    }
    Ok(LogicalImage {
        sequence: snapshot.seqno(),
        keyspaces,
    })
}
