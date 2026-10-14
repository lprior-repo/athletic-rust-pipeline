use serde::{Deserialize, Deserializer};

use super::super::legacy_wire::{self, LegacyRoot};
use super::super::object_wire::{self, JournalPath};
use super::namespace::{allocation_refused, validate_text};
use super::JournalNamespace;
use crate::journal::{parse_chunk_key, OBJECT_ALIAS_TYPE, OBJECT_RECORD_BYTES_LIMIT};
use crate::{JournalReadBudget, StoreError, StoreResult, MAX_JOURNAL_KEY_BYTES};

pub(super) struct RootTracker<'a> {
    namespace: JournalNamespace<'a>,
    previous: Vec<u8>,
    state: RootState,
    roots: u64,
}

enum RootState {
    Empty,
    Unchunked,
    Chunked { expected: usize, seen: usize },
}

impl<'a> RootTracker<'a> {
    pub(super) fn new(namespace: JournalNamespace<'a>) -> StoreResult<Self> {
        let mut previous = Vec::new();
        previous
            .try_reserve_exact(MAX_JOURNAL_KEY_BYTES)
            .map_err(|_| allocation_refused())?;
        Ok(Self {
            namespace,
            previous,
            state: RootState::Empty,
            roots: 0,
        })
    }

    pub(super) fn accept(
        &mut self,
        key: &[u8],
        raw: &[u8],
        visit: &mut impl FnMut(&str) -> StoreResult<()>,
    ) -> StoreResult<()> {
        let logical = self.namespace.logical_suffix(key)?;
        match logical.iter().position(|byte| *byte == 1) {
            Some(separator) => self.chunk(logical, separator),
            None => self.root(key, logical, raw, visit),
        }
    }

    fn chunk(&mut self, suffix: &[u8], separator: usize) -> StoreResult<()> {
        if separator.checked_add(5) != Some(suffix.len()) {
            return Err(self
                .namespace
                .refusal("chunk key has an unsupported suffix"));
        }
        let (logical, ordinal) = parse_chunk_key(suffix)
            .ok_or_else(|| self.namespace.refusal("chunk key is malformed"))?;
        if logical != self.previous.as_slice() {
            return Err(self
                .namespace
                .refusal("chunk has no preceding logical root"));
        }
        match &mut self.state {
            RootState::Chunked { expected, seen } if ordinal == *seen && *seen < *expected => {
                *seen = seen.checked_add(1).ok_or(StoreError::CounterOverflow)?;
                Ok(())
            }
            RootState::Empty | RootState::Unchunked | RootState::Chunked { .. } => Err(self
                .namespace
                .refusal("chunk is orphaned, missing, or surplus")),
        }
    }

    fn root(
        &mut self,
        physical: &[u8],
        logical: &[u8],
        raw: &[u8],
        visit: &mut impl FnMut(&str) -> StoreResult<()>,
    ) -> StoreResult<()> {
        self.complete_previous()?;
        if physical.len() > MAX_JOURNAL_KEY_BYTES {
            return Err(StoreError::RowTooLarge {
                bytes: physical.len(),
                max: MAX_JOURNAL_KEY_BYTES,
            });
        }
        let key = std::str::from_utf8(logical)
            .map_err(|_| self.namespace.refusal("logical root key is not utf-8"))?;
        validate_text(key)?;
        let path = JournalPath {
            phase: self.namespace.phase(),
            key,
        };
        self.state = classify(path, raw)?;
        self.previous.clear();
        self.previous.extend_from_slice(logical);
        self.roots = self
            .roots
            .checked_add(1)
            .ok_or(StoreError::CounterOverflow)?;
        visit(key)
    }

    fn complete_previous(&self) -> StoreResult<()> {
        match self.state {
            RootState::Chunked { expected, seen } if expected != seen => Err(self
                .namespace
                .refusal("logical root has missing physical chunks")),
            RootState::Empty | RootState::Unchunked | RootState::Chunked { .. } => Ok(()),
        }
    }

    pub(super) fn finish(self) -> StoreResult<u64> {
        self.complete_previous()?;
        Ok(self.roots)
    }
}

fn classify(path: JournalPath<'_>, raw: &[u8]) -> StoreResult<RootState> {
    let header: RootHeader = serde_json::from_slice(raw).map_err(|source| StoreError::Decode {
        key: format!("journal root type for {}/{}", path.phase, path.key),
        source,
    })?;
    if let RootType::Typed(kind) = &header.kind {
        if kind == OBJECT_ALIAS_TYPE {
            object_wire::alias_object(path, raw)?;
            return Ok(RootState::Unchunked);
        }
    }
    let budget = JournalReadBudget::new(OBJECT_RECORD_BYTES_LIMIT)?;
    match legacy_wire::root(path, raw, budget)? {
        LegacyRoot::Inline(_) => Ok(RootState::Unchunked),
        LegacyRoot::Chunked(bounds) => Ok(RootState::Chunked {
            expected: bounds.chunks,
            seen: 0,
        }),
    }
}

#[derive(Deserialize)]
struct RootHeader {
    #[serde(rename = "type", default)]
    kind: RootType,
}

#[derive(Default)]
enum RootType {
    #[default]
    Legacy,
    Typed(String),
}

impl<'de> Deserialize<'de> for RootType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::Typed)
    }
}
