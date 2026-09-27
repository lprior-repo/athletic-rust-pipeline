use census_domain::model::CanonicalSchool;
use census_domain::UsJurisdiction;
use census_store::{Store, StoreError, Table, MAX_ROWS_PER_TABLE};
use fjall::{Database, KeyspaceCreateOptions, PersistMode};
use serde::{Serialize, Serializer};
use std::error::Error;
use std::path::Path;
use std::sync::Barrier;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

struct Aligned<'a> {
    school: &'a CanonicalSchool,
    ready: &'a Barrier,
}

impl Serialize for Aligned<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        self.ready.wait();
        self.school.serialize(serializer)
    }
}

fn main() -> Result<()> {
    let root = tempfile::tempdir()?;
    seed(root.path())?;
    let store = Store::open(root.path())?;
    let ready = Barrier::new(2);
    let left = Aligned {
        school: &CanonicalSchool::new(UsJurisdiction::Wisconsin, "Left", "left").0,
        ready: &ready,
    };
    let right = Aligned {
        school: &CanonicalSchool::new(UsJurisdiction::Wisconsin, "Right", "right").0,
        ready: &ready,
    };
    let outcomes = std::thread::scope(|scope| {
        let first = scope.spawn(|| store.append(Table::Schools, &left));
        let second = scope.spawn(|| store.append(Table::Schools, &right));
        [first.join(), second.join()]
    });
    let outcomes = outcomes.map(|outcome| outcome.map_err(|_| "writer panicked"));
    let [left, right] = outcomes;
    let outcomes = [left?, right?];
    let committed = outcomes.iter().filter(|outcome| outcome.is_ok()).count();
    let refused = outcomes
        .iter()
        .filter(|outcome| matches!(outcome, Err(StoreError::TooManyRows { .. })))
        .count();
    drop(store);
    let store = Store::open(root.path())?;
    let mut raw_rows = 0;
    store
        .snapshot()
        .for_each_observation::<CanonicalSchool>(Table::Schools, |_| {
            raw_rows += 1;
            Ok(())
        })?;
    drop(store);
    let mark = persisted_mark(root.path())?;
    serde_json::to_writer(
        std::io::stdout().lock(),
        &serde_json::json!({
            "committed": committed, "refused": refused, "reopened_raw_rows": raw_rows,
            "persisted_mark": mark, "ceiling": MAX_ROWS_PER_TABLE,
            "aligned_after_preflight": true
        }),
    )?;
    if committed != 1 || refused != 1 || raw_rows != 1 || mark != MAX_ROWS_PER_TABLE {
        return Err("row-ceiling race violated the append boundary".into());
    }
    Ok(())
}

fn seed(root: &Path) -> Result<()> {
    drop(Store::open(root)?);
    let db = Database::builder(root.join("fjall")).open()?;
    let meta = db.keyspace("meta", KeyspaceCreateOptions::default)?;
    let mut batch = db.batch();
    batch.insert(
        &meta,
        "sequence:schools",
        (MAX_ROWS_PER_TABLE - 1).to_string().as_bytes(),
    );
    batch.durability(Some(PersistMode::SyncData)).commit()?;
    Ok(())
}

fn persisted_mark(root: &Path) -> Result<u64> {
    let db = Database::builder(root.join("fjall")).open()?;
    let meta = db.keyspace("meta", KeyspaceCreateOptions::default)?;
    let bytes = meta
        .get("sequence:schools")?
        .ok_or("sequence mark missing")?;
    Ok(std::str::from_utf8(&bytes)?.parse()?)
}
