use crate::model::MatchRecord;
use anyhow::Result;
use std::{collections::HashMap, path::Path};

pub fn load_latest(path: &Path) -> Result<HashMap<String, MatchRecord>> {
    crate::jsonl::load(path, |record: &MatchRecord| record.source_key.clone())
}

pub fn append(path: &Path, record: &MatchRecord) -> Result<()> {
    crate::jsonl::append(path, record)
}
