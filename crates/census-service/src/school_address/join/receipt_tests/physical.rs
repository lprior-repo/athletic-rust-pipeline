use super::*;
use fjall::{Database, KeyspaceCreateOptions};

pub(super) type Rows = Vec<(Vec<u8>, Vec<u8>)>;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct Physical {
    spaces: BTreeMap<String, Rows>,
}

pub(super) fn checkpoint(store: Store) -> TestResult<(Store, Physical)> {
    let root = store.root().to_path_buf();
    drop(store);
    let physical = read(&root)?;
    Ok((Store::open(root)?, physical))
}

fn read(root: &Path) -> TestResult<Physical> {
    let database = Database::builder(root.join("fjall")).open()?;
    let mut spaces = BTreeMap::new();
    for name in ["entities", "journal", "receipts", "meta"] {
        let keyspace = database.keyspace(name, KeyspaceCreateOptions::default)?;
        let mut rows = Vec::new();
        for guard in keyspace.iter() {
            let (key, value) = guard.into_inner()?;
            rows.push((key.to_vec(), value.to_vec()));
        }
        rows.sort();
        spaces.insert(name.to_string(), rows);
    }
    Ok(Physical { spaces })
}

impl Physical {
    pub(super) fn rows(&self, keyspace: &str) -> TestResult<&Rows> {
        self.spaces
            .get(keyspace)
            .ok_or_else(|| "missing physical keyspace".into())
    }

    pub(super) fn schools(&self) -> TestResult<Rows> {
        let prefix = format!("{}\0", Table::Schools.file());
        Ok(self
            .rows("entities")?
            .iter()
            .filter(|(key, _)| key.starts_with(prefix.as_bytes()))
            .cloned()
            .collect())
    }

    pub(super) fn applications(&self) -> TestResult<Rows> {
        Ok(self
            .rows("receipts")?
            .iter()
            .filter(|(key, _)| key.starts_with(OPERATION_PREFIX.as_bytes()))
            .cloned()
            .collect())
    }
}
