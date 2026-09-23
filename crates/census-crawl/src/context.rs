//! The per-run context every adapter takes, and the school half of the observation funnel.
//!
//! The context is the whole of what an adapter is handed: the fetcher, the store, the run's
//! coordinates and where its rows go. It lives in its own module — split out of `lib.rs`, which
//! keeps the error type, the module table and the shared helpers — so the routing seam (a walk whose
//! rows are recorded instead of written) sits beside the funnel that stamps them, rather than in the
//! file every adapter declaration shares.
//!
//! [`observe_schools_of`] is the school half of the funnel; [`crate::athlete_observations`] holds
//! the athlete half.

use crate::athlete_observations::observe_athletes_of;
use crate::net::{FetchOptions, Fetcher};
use crate::recording::{Recording, RowBatch, RowSink};
use crate::CrawlResult;
use census_domain::model::{
    CanonicalAthlete, CanonicalSchool, SourceNamespace, SourceObservation, SourceSchoolObservation,
};
use census_store::{Store, Table};

/// Shared per-run context handed to every adapter.
pub struct AdapterContext<'a> {
    pub fetcher: &'a Fetcher,
    pub store: &'a Store,
    pub refresh: bool,
    pub school_year: census_domain::model::SchoolYear,
    pub observed_on: String,
    /// Where this context's rows go: `None` writes them to `store` as the walk produces them, and
    /// `Some` holds them for the caller that posts them to its source's `Ingest` object. The walk
    /// does not know which it is: it asks for a batch either way.
    pub recording: Option<&'a Recording>,
}

impl<'a> AdapterContext<'a> {
    /// Start a batch on this context's sink, the one place an adapter writes rows.
    ///
    /// The commit boundary is the store's own: a batch dropped without [`RowBatch::commit`] leaves
    /// the store untouched, and a recording without a trace.
    pub fn write_batch(&self) -> RowBatch<'_> {
        self.sink().write_batch()
    }

    /// The sink this context routes to: the recording when the run posts its rows, the store when
    /// the run owns it.
    fn sink(&self) -> RowSink<'_> {
        match self.recording {
            Some(recording) => RowSink::Record(recording),
            None => RowSink::Store(self.store),
        }
    }

    pub fn fetch_options(&self) -> FetchOptions {
        FetchOptions {
            refresh: self.refresh,
            allow_not_found: false,
            headers: Vec::new(),
        }
    }

    /// Record what a source itself published about a school, beside the canonical row the adapter
    /// minted for it.
    ///
    /// The observation is keyed by the provider's own object id, so a canonical merge that turns out
    /// to be wrong is re-decided by reading these rows instead of reading the provider again. A row
    /// that carries no identity for `namespace` writes nothing and is not counted: an observation
    /// filed under a key the source never published is worse than no observation at all.
    ///
    /// Returns how many observations it wrote, so an adapter can state the figure rather than infer it.
    pub fn observe_schools(
        &self,
        namespace: &SourceNamespace,
        schools: &[CanonicalSchool],
    ) -> CrawlResult<usize> {
        observe_schools_of(self.store, namespace, schools, &self.observed_on)
    }

    /// Record what a source itself published about one school.
    pub fn observe_school(
        &self,
        namespace: &SourceNamespace,
        school: &CanonicalSchool,
    ) -> CrawlResult<usize> {
        self.observe_schools(namespace, std::slice::from_ref(school))
    }

    /// Record what a source itself published about an athlete, beside the canonical row the adapter
    /// minted for them.
    ///
    /// The observation is keyed by the provider's own athlete id, so a canonical merge that turns out
    /// to be wrong is re-decided by reading these rows instead of reading the provider again. A row
    /// that carries no identity for `namespace` writes nothing and is not counted: an observation
    /// filed under a key the source never published is worse than no observation at all.
    ///
    /// `schools` are the school rows the pass placed those athletes at, which is where the source's
    /// own spelling of the school is, so an observation names the school as the source wrote it.
    ///
    /// Returns how many observations it wrote, so an adapter can state the figure rather than infer it.
    pub fn observe_athletes<'s>(
        &self,
        namespace: &SourceNamespace,
        athletes: &[CanonicalAthlete],
        schools: impl IntoIterator<Item = &'s CanonicalSchool>,
    ) -> CrawlResult<usize> {
        observe_athletes_of(self.store, namespace, athletes, schools, &self.observed_on)
    }
}

/// The funnel itself, for the callers that hold a store rather than an [`AdapterContext`].
///
/// `namespace` is what the walk just read — never something inferred from the rows: a school that
/// carries an identity for a provider this run did not fetch would otherwise be filed as that
/// provider's sighting of a day it was never read on.
pub fn observe_schools_of(
    store: &Store,
    namespace: &SourceNamespace,
    schools: &[CanonicalSchool],
    observed_on: &str,
) -> CrawlResult<usize> {
    let rows: Vec<SourceObservation> = schools
        .iter()
        .filter_map(|school| SourceSchoolObservation::of_school(namespace, school, observed_on))
        .map(SourceObservation::School)
        .collect();
    store.append_many(Table::SourceObservations, &rows)?;
    Ok(rows.len())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch() -> (tempfile::TempDir, Store, Fetcher) {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = Store::open(dir.path().join("store")).expect("store");
        let fetcher = Fetcher::new(
            dir.path().join("http"),
            None,
            std::time::Duration::from_millis(1),
            std::collections::HashMap::new(),
            Vec::new(),
        )
        .expect("fetcher");
        (dir, store, fetcher)
    }

    fn context<'a>(
        store: &'a Store,
        fetcher: &'a Fetcher,
        recording: Option<&'a Recording>,
    ) -> AdapterContext<'a> {
        AdapterContext {
            fetcher,
            store,
            refresh: false,
            school_year: census_domain::model::SchoolYear::new(2026).expect("2026 is a season"),
            observed_on: "2026-09-23".to_string(),
            recording,
        }
    }

    /// A context with no recording writes what its batch commits; one with a recording writes
    /// nothing and holds the rows instead. This is the whole seam: an adapter's rows follow the
    /// context it was handed, not the walk it sits in.
    #[test]
    fn the_context_routes_its_batch_to_its_sink() {
        let (_dir, store, fetcher) = scratch();
        let direct_ctx = context(&store, &fetcher, None);
        let mut direct = direct_ctx.write_batch();
        direct
            .append_many(Table::Meets, &[json!({"id": "m1"})])
            .expect("buffered");
        direct.commit().expect("committed");
        assert_eq!(store.walk_table(Table::Meets).expect("walk").rows, 1);

        let recording = Recording::new();
        let routed_ctx = context(&store, &fetcher, Some(&recording));
        let mut routed = routed_ctx.write_batch();
        routed
            .append_many(Table::Meets, &[json!({"id": "m2"})])
            .expect("recorded");
        routed.commit().expect("committed");
        assert_eq!(
            store.walk_table(Table::Meets).expect("walk").rows,
            1,
            "the routed walk wrote nothing"
        );
        assert_eq!(recording.rows(), 1);
    }
}
