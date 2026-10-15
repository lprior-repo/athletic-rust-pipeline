use super::{decode, encode, invalid, read, ObjectRef, ReportResult, INVENTORY_BYTES};
use census_store::{StoreError, StoreResult};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const MAX_ENTRIES: usize = 8192;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Inventory {
    version: u32,
    entries: BTreeMap<String, ObjectRef>,
    bytes: u64,
}

pub(in super::super) struct Sink {
    directory: PathBuf,
    entries: BTreeMap<String, ObjectRef>,
    bytes: u64,
}

impl Sink {
    pub(in super::super) fn new(directory: &Path) -> Self {
        Self {
            directory: directory.to_path_buf(),
            entries: BTreeMap::new(),
            bytes: 0,
        }
    }

    pub(in super::super) fn retain(
        &mut self,
        body_digest: &str,
        body: &[u8],
        metadata_ref: &str,
        metadata: &[u8],
    ) -> StoreResult<()> {
        let metadata_path = metadata_ref
            .strip_prefix("archive/")
            .ok_or_else(|| invalid("retained metadata reference leaves archive namespace"))?;
        self.install(&format!("bodies/{body_digest}.body"), body)?;
        self.install(metadata_path, metadata)
    }

    fn install(&mut self, path: &str, bytes: &[u8]) -> StoreResult<()> {
        let (expected, ceiling) = object(path)?;
        let length = u64::try_from(bytes.len()).map_err(|_| StoreError::CounterOverflow)?;
        if length == 0 || length > ceiling || super::super::archive::hash(bytes) != expected {
            return Err(invalid(
                "retained archive object does not match its original name or bound",
            ));
        }
        let reference = ObjectRef {
            digest: expected.to_string(),
            bytes: length,
        };
        if let Some(previous) = self.entries.get(path) {
            if previous != &reference {
                return Err(invalid("frozen archive path changes immutable content"));
            }
            return Ok(());
        }
        if self.entries.len() >= MAX_ENTRIES {
            return Err(invalid("frozen archive inventory exceeds 8192 entries"));
        }
        super::super::charge(
            &mut self.bytes,
            bytes.len(),
            super::super::MAX_ARCHIVE_BYTES,
            "frozen affiliation archive bytes",
        )?;
        let installed =
            crate::export::proof_objects::install(&self.directory, bytes).map_err(store_error)?;
        if installed != expected {
            return Err(invalid("frozen archive object installation changed digest"));
        }
        self.entries.insert(path.to_string(), reference);
        Ok(())
    }

    pub(in super::super) fn finish(self) -> ReportResult<ObjectRef> {
        encode(
            &self.directory,
            &Inventory {
                version: 1,
                entries: self.entries,
                bytes: self.bytes,
            },
            INVENTORY_BYTES,
        )
    }
}

pub(in super::super) struct Source {
    directory: PathBuf,
    inventory: Inventory,
}

impl Source {
    pub(super) fn open(directory: &Path, reference: &ObjectRef) -> ReportResult<Self> {
        let inventory: Inventory = decode(directory, reference, INVENTORY_BYTES)?;
        if inventory.version != 1 || inventory.entries.len() > MAX_ENTRIES {
            return Err(invalid("frozen archive inventory version or count is invalid").into());
        }
        let mut bytes = 0;
        for (path, reference) in &inventory.entries {
            let (expected, ceiling) = object(path)?;
            reference.validate(ceiling)?;
            if reference.digest != expected {
                return Err(invalid("frozen archive path and object digest disagree").into());
            }
            let length =
                usize::try_from(reference.bytes).map_err(|_| StoreError::CounterOverflow)?;
            super::super::charge(
                &mut bytes,
                length,
                super::super::MAX_ARCHIVE_BYTES,
                "frozen affiliation archive bytes",
            )?;
        }
        if bytes != inventory.bytes {
            return Err(invalid("frozen archive inventory byte count is inexact").into());
        }
        Ok(Self {
            directory: directory.to_path_buf(),
            inventory,
        })
    }

    pub(in super::super) fn read(
        &self,
        path: &str,
        ceiling: usize,
        admitted: &mut u64,
    ) -> StoreResult<Option<Vec<u8>>> {
        let Some(reference) = self.inventory.entries.get(path) else {
            return Ok(None);
        };
        let length = usize::try_from(reference.bytes).map_err(|_| StoreError::CounterOverflow)?;
        if length > ceiling {
            return Err(invalid(
                "frozen archive object exceeds requested capture bound",
            ));
        }
        super::super::charge(
            admitted,
            length,
            super::super::MAX_ARCHIVE_BYTES,
            "current affiliation archive bytes",
        )?;
        read(
            &self.directory,
            reference,
            u64::try_from(ceiling).map_err(|_| StoreError::CounterOverflow)?,
        )
        .map(Some)
        .map_err(store_error)
    }

    pub(in super::super) fn metadata(&self, capture: &str) -> StoreResult<Vec<String>> {
        let prefix = format!("captures/{capture}/");
        let mut paths = Vec::new();
        for path in self
            .inventory
            .entries
            .range(prefix.clone()..)
            .map(|(path, _)| path)
            .take_while(|path| path.starts_with(&prefix))
        {
            if paths.len() >= 128 {
                return Err(invalid(
                    "frozen capture metadata inventory exceeds 128 objects",
                ));
            }
            paths.push(path.clone());
        }
        Ok(paths)
    }
}

fn object(path: &str) -> StoreResult<(&str, u64)> {
    if let Some(digest) = path
        .strip_prefix("bodies/")
        .and_then(|value| value.strip_suffix(".body"))
    {
        if super::super::note::digest(digest) {
            return Ok((digest, 8 * 1024 * 1024));
        }
    }
    if let Some(rest) = path.strip_prefix("captures/") {
        if let Some((body, metadata)) = rest.split_once('/') {
            if let Some(digest) = metadata.strip_suffix(".meta.json") {
                if super::super::note::digest(body) && super::super::note::digest(digest) {
                    return Ok((digest, 64 * 1024));
                }
            }
        }
    }
    Err(invalid(
        "frozen archive inventory contains a foreign or malformed path",
    ))
}

fn store_error(error: crate::report::ReportError) -> StoreError {
    match error {
        crate::report::ReportError::Store(error) => error,
        crate::report::ReportError::Io { path, source } => StoreError::Io { path, source },
        other => invalid(&format!("frozen archive object refused: {other}")),
    }
}
