//! Page identity: which checkpoint of a page the collection currently accepts.
//!
//! A page marker is the only authority on whether page-derived index data is
//! live. `put_rankings_page` writes the marker and every page-derived key of the
//! capture in one durable batch, and the marker names the checkpoint it accepts.
//! `drop_rankings_page` removes the marker alone, so an abandoned capture's keys
//! stay behind as unreferenced garbage: readers resolve the accepted checkpoints
//! from the markers and keep only the keys that carry one, which is why a
//! dropped capture is invisible even though its keys are still present.

use super::common::DIGEST_BYTES;
use super::keys::{parse_page_marker_value, validate_event_short};
use crate::store::backend::StoreInner;
use crate::store::StoreError;
use fjall::{Readable, Snapshot};
use std::collections::{BTreeMap, BTreeSet};

/// The captures one event's page markers accept.
#[derive(Default)]
struct AcceptedEvent {
    pages: u64,
    checkpoints: BTreeSet<Vec<u8>>,
}

/// The page markers under one prefix, resolved to the captures they accept.
pub(in crate::store) struct AcceptedPages {
    events: BTreeMap<String, AcceptedEvent>,
    checkpoints: BTreeSet<Vec<u8>>,
}

impl AcceptedPages {
    /// Number of pages the resolved markers publish for one event.
    pub(in crate::store) fn pages_for(&self, event_short: &str) -> u64 {
        self.events.get(event_short).map_or(0, |event| event.pages)
    }

    /// Reports whether a marker of the collection accepts `checkpoint`.
    pub(in crate::store) fn retains(&self, checkpoint: &[u8]) -> bool {
        self.checkpoints.contains(checkpoint)
    }

    /// Reports whether a marker of `event_short` accepts `checkpoint`.
    pub(in crate::store) fn retains_for(&self, event_short: &str, checkpoint: &[u8]) -> bool {
        self.events
            .get(event_short)
            .is_some_and(|event| event.checkpoints.contains(checkpoint))
    }
}

/// Resolve every page marker under `marker_prefix`.
///
/// A marker whose key is not `<marker_prefix><event short>\0<page(4B BE)>` with a
/// valid event and a nonzero page, whose value is not a checkpoint/hash pair, or
/// that certifies a checkpoint another page of the same event already certifies,
/// is corruption rather than absence: the scan fails closed.
pub(in crate::store) fn accepted_pages(
    snap: &Snapshot,
    store: &StoreInner,
    marker_prefix: &[u8],
) -> Result<AcceptedPages, StoreError> {
    let prefix_len = marker_prefix.len();
    let mut events: BTreeMap<String, AcceptedEvent> = BTreeMap::new();
    let mut checkpoints = BTreeSet::new();

    for guard in snap.prefix(&store.rankings, marker_prefix) {
        let (key, value) = guard.into_inner().map_err(|_| StoreError::CorruptData)?;
        let suffix = key.get(prefix_len..).ok_or(StoreError::CorruptData)?;
        // The suffix is the event short and its separator, then the page.
        let page_start = suffix
            .len()
            .checked_sub(std::mem::size_of::<u32>())
            .ok_or(StoreError::CorruptData)?;
        let (event, page) = suffix.split_at(page_start);
        let event_short = event
            .strip_suffix(&[0])
            .filter(|short| !short.is_empty())
            .and_then(|short| std::str::from_utf8(short).ok())
            .ok_or(StoreError::CorruptData)?;
        validate_event_short(event_short)?;
        let page: [u8; 4] = page.try_into().map_err(|_| StoreError::CorruptData)?;
        if u32::from_be_bytes(page) == 0 {
            return Err(StoreError::CorruptData);
        }
        let (checkpoint, _) = parse_page_marker_value(value.as_ref())?;
        let accepted = events.entry(event_short.to_owned()).or_default();
        if !accepted
            .checkpoints
            .insert(checkpoint.as_str().as_bytes().to_vec())
        {
            return Err(StoreError::CorruptData);
        }
        accepted.pages = accepted
            .pages
            .checked_add(1)
            .ok_or(StoreError::CorruptData)?;
        let _ = checkpoints.insert(checkpoint.as_str().as_bytes().to_vec());
    }

    Ok(AcceptedPages {
        events,
        checkpoints,
    })
}

/// Borrow the checkpoint segment of a page-derived key at `offset`.
///
/// The segment is exactly one digest and must be followed by its separator, so a
/// key that omits or misplaces the checkpoint fails closed instead of being read
/// as if it belonged to the accepted capture.
pub(in crate::store) fn checkpoint_at(key: &[u8], offset: usize) -> Result<&[u8], StoreError> {
    let end = offset
        .checked_add(DIGEST_BYTES)
        .ok_or(StoreError::CorruptData)?;
    let checkpoint = key.get(offset..end).ok_or(StoreError::CorruptData)?;
    (key.get(end) == Some(&0))
        .then_some(checkpoint)
        .ok_or(StoreError::CorruptData)
}
