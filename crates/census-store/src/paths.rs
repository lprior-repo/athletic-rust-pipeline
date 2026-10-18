use super::{Store, Table};
use std::path::{Path, PathBuf};

impl Store {
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn http_cache_dir(&self) -> PathBuf {
        self.root.join("http")
    }

    pub fn out_dir(&self) -> PathBuf {
        self.root.join("out")
    }

    pub fn table_path(&self, table: Table) -> PathBuf {
        self.root
            .join("entities")
            .join(format!("{}.jsonl", table.file()))
    }
}
