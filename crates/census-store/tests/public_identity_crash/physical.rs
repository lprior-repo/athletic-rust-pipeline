use std::collections::BTreeMap;
use std::path::Path;

use fjall::{Database, KeyspaceCreateOptions, Readable};

use super::TestResult;

pub(super) type Rows = BTreeMap<Vec<u8>, Vec<u8>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct State(BTreeMap<String, Rows>);

impl State {
    pub fn rows(&self, name: &str) -> TestResult<&Rows> {
        self.0
            .get(name)
            .ok_or_else(|| format!("missing keyspace {name}").into())
    }
}

pub(super) fn read(root: &Path) -> TestResult<State> {
    check!(root.join("fjall/version").is_file());
    let db = Database::builder(root.join("fjall"))
        .cache_size(8_388_608)
        .open()?;
    let snapshot = db.snapshot();
    let mut spaces = BTreeMap::new();
    for name in ["entities", "journal", "meta", "receipts"] {
        check!(db.keyspace_exists(name), "missing existing keyspace {name}");
        let keyspace = db.keyspace(name, KeyspaceCreateOptions::default)?;
        let mut rows = BTreeMap::new();
        for guard in snapshot.prefix(&keyspace, b"").take(2_049) {
            check!(rows.len() < 2_048, "fixture row bound exceeded");
            let key = guard.key()?;
            check!(key.len() <= 4_096);
            let size = snapshot
                .size_of(&keyspace, &key)?
                .ok_or("missing sized row")?;
            check!(size <= 65_536, "fixture value bound exceeded");
            let value = snapshot
                .get(&keyspace, &key)?
                .ok_or("missing fixture row")?;
            check!(eq; value.len(), usize::try_from(size)?);
            check!(rows.insert(key.to_vec(), value.to_vec()).is_none());
        }
        check!(spaces.insert(name.to_owned(), rows).is_none());
    }
    Ok(State(spaces))
}
