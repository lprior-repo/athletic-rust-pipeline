mod append;
mod commit;
mod journal;

use super::Store;
use super::Table;

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
