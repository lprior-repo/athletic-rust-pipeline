use super::super::fixtures::TestResult;
use super::super::objects;
use census_store::{InputRecord, LeaseRecord, Store};
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RetainedEvidence {
    marker: Vec<u8>,
    objects: BTreeMap<OsString, Vec<u8>>,
    records: BTreeMap<String, Option<InputRecord>>,
    leases: BTreeMap<String, Option<LeaseRecord>>,
    sequence: u64,
}

impl RetainedEvidence {
    pub(super) fn capture(store: &Store, directory: &Path, attempts: &[&str]) -> TestResult<Self> {
        Ok(Self {
            marker: std::fs::read(directory.join("root.json"))?,
            objects: object_bytes(directory)?,
            records: attempts
                .iter()
                .map(|attempt| -> TestResult<_> {
                    Ok(((*attempt).to_string(), store.get_input_record(attempt)?))
                })
                .collect::<TestResult<_>>()?,
            leases: attempts
                .iter()
                .map(|attempt| -> TestResult<_> {
                    Ok(((*attempt).to_string(), store.get_lease_record(attempt)?))
                })
                .collect::<TestResult<_>>()?,
            sequence: store.snapshot().sequence(),
        })
    }

    pub(super) fn unchanged(&self, store: &Store, directory: &Path) -> TestResult {
        let attempts: Vec<_> = self.records.keys().map(String::as_str).collect();
        check!(eq; Self::capture(store, directory, &attempts)?, *self);
        Ok(())
    }
}

pub(super) fn object_bytes(directory: &Path) -> TestResult<BTreeMap<OsString, Vec<u8>>> {
    std::fs::read_dir(objects::directory(directory))?
        .map(|entry| -> TestResult<_> {
            let entry = entry?;
            Ok((entry.file_name(), std::fs::read(entry.path())?))
        })
        .collect()
}
