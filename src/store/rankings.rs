pub(crate) mod backend;
pub(crate) mod backend_stats;
pub(crate) mod common;
mod key_builders;
mod key_prefixes;
mod keys;
mod presence_keys;
pub(crate) mod types;

pub use types::{
    RankingCandidateEntry, RankingCandidateKind, RankingCollectionStats, RankingEventStats,
    RankingPageIndex, RankingLookup, RankingRecordRef, RankingRosterObservation,
    RankingSourceRow,
};

use crate::domain::identity::{AthleteId, EvidenceDigest};
use crate::domain::name::CanonicalName;
use crate::store::backend::StoreInner;
use crate::store::error::Result;

/// Public storage API for ranking index operations.
pub struct RankingsStore<'a> {
    inner: &'a StoreInner,
}

impl<'a> RankingsStore<'a> {
    pub fn new(inner: &'a StoreInner) -> Self {
        Self { inner }
    }

    /// Put a validated rankings page index into the collection.
    ///
    /// Idempotent: duplicate publication leaves counts unchanged.
    /// Conflicting page or sealed namespace returns explicit error.
    pub fn put_rankings_page(&self, index: &RankingPageIndex) -> Result<()> {
        backend::put_rankings_page(self.inner, index)
    }

    /// Lookup name refs within a collection by normalized name prefix.
    ///
    /// Reads limit+1 to set truncated flag; does not silently truncate.
    pub fn ranking_name_refs(
        &self,
        collection: &EvidenceDigest,
        name: &CanonicalName,
        limit: usize,
    ) -> Result<RankingLookup> {
        backend::ranking_name_refs(self.inner, collection, name, limit)
    }

    /// Lookup all observed names/events/provenance for a real athlete ID.
    pub fn ranking_athlete_refs(
        &self,
        collection: &EvidenceDigest,
        athlete: AthleteId,
        limit: usize,
    ) -> Result<RankingLookup> {
        backend::ranking_athlete_refs(self.inner, collection, athlete, limit)
    }

    /// Compute event-level stats via streaming key-prefix scans.
    pub fn ranking_event_stats(
        &self,
        collection: &EvidenceDigest,
        event_short: &str,
    ) -> Result<RankingEventStats> {
        backend_stats::ranking_event_stats(self.inner, collection, event_short)
    }

    /// Count distinct real ID presence keys across a collection.
    pub fn ranking_collection_stats(
        &self,
        collection: &EvidenceDigest,
    ) -> Result<RankingCollectionStats> {
        backend_stats::ranking_collection_stats(self.inner, collection)
    }

    /// Seal a rankings collection as immutable.
    ///
    /// Idempotent: same seal returns Ok. Conflicting seal rejected.
    /// New pages after seal returns explicit error.
    pub fn seal_rankings(
        &self,
        collection: &EvidenceDigest,
        snapshot: &EvidenceDigest,
    ) -> Result<()> {
        backend::seal_rankings(self.inner, collection, snapshot)
    }

    /// Return the immutable seal digest if one exists.
    ///
    /// Returns moving progress: only the seal, not page markers.
    pub fn ranking_snapshot(
        &self,
        collection: &EvidenceDigest,
    ) -> Result<Option<EvidenceDigest>> {
        backend::ranking_snapshot(self.inner, collection)
    }
}
