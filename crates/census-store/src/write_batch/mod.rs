mod append;
mod budget;
#[cfg(test)]
mod byte_tests;
mod commit;
mod conditional;
mod journal;
mod revision;

pub use revision::{RevisionedApplication, StoreRevision};

pub(crate) use journal::encode_publication_entry;

use super::Store;
use super::Table;
pub use budget::{
    ObservationBudget, ObservationChunk, MAX_OBSERVATION_BATCH_BYTES, MAX_OBSERVATION_BATCH_ROWS,
    MAX_OBSERVATION_RECORD_BYTES,
};
pub use conditional::ConditionalBatch;

struct Page {
    table: Table,
    records: Vec<(String, Vec<u8>)>,
}

struct Replacement {
    table: Table,
    records: Vec<Vec<u8>>,
}

pub struct StoreBatch<'s> {
    store: &'s Store,
    pages: Vec<Page>,
    journal: Vec<(Vec<u8>, Vec<u8>)>,
    replacements: Vec<Replacement>,
}

impl Store {
    pub fn write_batch(&self) -> StoreBatch<'_> {
        StoreBatch {
            store: self,
            pages: Vec::new(),
            journal: Vec::new(),
            replacements: Vec::new(),
        }
    }
}

impl StoreBatch<'_> {
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.journal.is_empty() && self.replacements.is_empty()
    }
}
