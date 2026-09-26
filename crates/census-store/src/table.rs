//! The store's table vocabulary: which canonical collections exist, and what an entity must do to
//! live in one.
//!
//! `Table` is the store's own naming for its collections — the names ride in observation
//! keys and in sidecar file names, so they are wire vocabulary rather than an implementation
//! detail. `Entity` is the merge contract every canonical row obeys: the store appends observations
//! and merges them at read time, so a row type must state how a second observation folds into the
//! first, and the contract rules are applied on the merged value rather than on each observation.

use serde::{de::DeserializeOwned, Deserialize, Serialize};

/// Hard ceiling on the observations one table may hold. A batch that would take a table past it is
/// refused: an appender by the sequence the batch would reach — the counter is the row count for an
/// append-only table — and a derived writer by the rows the batch itself hands the table, since a
/// derived batch is a whole row set. A table larger than this — one restored from a backup written
/// before that refusal, say — aborts the scan with a typed error instead of exhausting memory: the
/// bound is what keeps Rule 2 (bounded control flow) honest for a store whose input size is not
/// known in advance.
pub const MAX_ROWS_PER_TABLE: u64 = 20_000_000;

/// Longest entity id the store accepts. Ids ride verbatim inside observation keys, and Fjall
/// asserts keys stay under 64 KiB; this ceiling keeps that assertion unreachable for callers.
pub const MAX_ID_BYTES: usize = 512;

/// Longest resume-journal key the store accepts: a phase, its separator, and one unit of work's own
/// key, which is an adapter's URL or file path. Fjall asserts keys stay under 64 KiB; a control
/// record nobody can key is not worth storing, so the bound sits far below the assertion.
pub const MAX_JOURNAL_KEY_BYTES: usize = 4 * 1024;

/// Longest serialized resume-journal entry the store accepts. The journal is a resume ledger: it
/// records *that* a unit of work finished and what it yielded, not the work itself, so a payload
/// which outgrows this is a caller appending evidence where it meant to journal completion.
pub const MAX_JOURNAL_VALUE_BYTES: usize = 1024 * 1024;

/// How a table is written, and therefore what a write to it means, what its ceiling bounds, and how
/// its rows relate to the sequence counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageMode {
    /// Append-only evidence: every write reserves the next sequence, so the counter points at the
    /// sequence the table's next append will use and a batch is bounded by the sequence space it would
    /// reach (`base + count`).
    ObservationLog,
    /// Derived state written whole: rows are keyed under sequence zero, one row per id, and a write
    /// names the table's entire new content — the rows it does not name are removed with it, because a
    /// finding the newest derivation does not contain no longer exists.
    DerivedSnapshot,
    /// Derived state written in part: rows are keyed under sequence zero, one row per id, and a write
    /// upserts the ids it names and leaves the rows it does not name standing.
    DerivedMap,
}

impl StorageMode {
    /// The observations a table in this mode has appended, given the rows it holds.
    ///
    /// An append-only table appends one observation per row, so the rows it holds are the observations
    /// it committed. A derived table appends none: every row it holds is state a pass materialized, and
    /// a report that counted those rows as observations would be counting recalculation as discovery.
    pub const fn appended_observations(self, rows: u64) -> u64 {
        match self {
            StorageMode::ObservationLog => rows,
            StorageMode::DerivedSnapshot | StorageMode::DerivedMap => 0,
        }
    }
}

/// The store's sixteen collections.
///
/// `Serialize`/`Deserialize` spell a table exactly as [`Table::file`] does — the wire name an
/// `Ingest` request carries — which `a_table_serializes_as_its_wire_name` holds to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Table {
    Schools,
    Teams,
    Coaches,
    Athletes,
    Meets,
    Events,
    Performances,
    /// Source-object identities: the §31 join from a provider's own id to a canonical row.
    SourceIdentities,
    /// Conflicts the merge retained: two rows one stored key says are the same subject.
    Conflicts,
    /// Cases the review lane owns, one row per finding, keyed so a repeated finding reuses its case.
    ReviewCases,
    /// Coverage measurements per jurisdiction and per source namespace.
    Coverage,
    /// One row per finished pass over the store.
    Snapshots,
    /// Access conditions a source imposed on this client: one row per blocked `(kind, host)`, so a
    /// repeated run sees the block it already paid for instead of re-discovering it request by
    /// request.
    SourceAccess,
    /// Adjudications the review lane recorded, one row per case, keyed by the case id so a re-asked
    /// case overwrites its earlier verdict instead of accumulating answers.
    IdentityVerdicts,
    /// Meets a source enumerated, before their results were read (§30 `source_meets`).
    SourceMeets,
    /// What a source itself published about the school and athlete objects it owns, keyed by the
    /// provider's own object id (§30 `source_observations`). Evidence, not derived state: a source
    /// read again on a later day appends a second sighting, and a canonical merge that turns out to
    /// be wrong is re-decided from these rows without reading the provider again.
    SourceObservations,
    /// Applied Rust identity decisions, bound to exact source-subject and verdict evidence.
    AthleteIdentityDecisions,
}

impl Table {
    pub fn file(self) -> &'static str {
        match self {
            Table::Schools => "schools",
            Table::Teams => "teams",
            Table::Coaches => "coaches",
            Table::Athletes => "athletes",
            Table::Meets => "meets",
            Table::Events => "events",
            Table::Performances => "performances",
            Table::SourceIdentities => "source_identities",
            Table::Conflicts => "conflicts",
            Table::ReviewCases => "review_cases",
            Table::Coverage => "coverage",
            Table::Snapshots => "snapshots",
            Table::SourceAccess => "source_access",
            Table::IdentityVerdicts => "identity_verdicts",
            Table::SourceMeets => "source_meets",
            Table::SourceObservations => "source_observations",
            Table::AthleteIdentityDecisions => "athlete_identity_decisions",
        }
    }

    pub const ALL: [Table; 17] = [
        Table::Schools,
        Table::Teams,
        Table::Coaches,
        Table::Athletes,
        Table::Meets,
        Table::Events,
        Table::Performances,
        Table::SourceIdentities,
        Table::Conflicts,
        Table::ReviewCases,
        Table::Coverage,
        Table::Snapshots,
        Table::SourceAccess,
        Table::IdentityVerdicts,
        Table::SourceMeets,
        Table::SourceObservations,
        Table::AthleteIdentityDecisions,
    ];

    /// Parse a wire name (`"schools"`) back into a table. Unknown names are rejected so a typo in an
    /// ingest request cannot silently create a table nobody scans.
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.file() == name)
    }

    /// How this table is written, which decides what a write to it means and what its writer's
    /// ceiling bounds — see [`StorageMode`].
    ///
    /// The split follows the writer each table has. The tables the adapters and the meet walks feed
    /// through [`Store::append_many`](crate::Store::append_many) are evidence and spend a
    /// sequence per row. The tables the index pass derives whole — identities, conflicts and coverage
    /// are rebuilt from the merged rows on every pass, so the pass's set *is* the table — are written
    /// as snapshots. The tables a pass or a sweep writes in part — review cases the index and the
    /// review lane each own half of, snapshots keyed by phase and day, access conditions a sweep
    /// learned and verdicts the lane reached — upsert by id and keep what they do not name.
    pub const fn storage_mode(self) -> StorageMode {
        match self {
            Table::Schools
            | Table::Teams
            | Table::Coaches
            | Table::Athletes
            | Table::Meets
            | Table::Events
            | Table::Performances
            | Table::SourceMeets
            | Table::SourceObservations => StorageMode::ObservationLog,
            Table::SourceIdentities | Table::Conflicts | Table::Coverage | Table::AthleteIdentityDecisions => {
                StorageMode::DerivedSnapshot
            }
            Table::ReviewCases
            | Table::Snapshots
            | Table::SourceAccess
            | Table::IdentityVerdicts => StorageMode::DerivedMap,
        }
    }
}

/// An entity that knows its own canonical id and how to absorb a duplicate observation.
pub trait Entity: Serialize + DeserializeOwned + Clone {
    fn entity_id(&self) -> &str;
    fn merge(&mut self, other: Self);

    /// Apply the collection contract to a merged entity. Every read of the store goes through
    /// [`Store::scan`](crate::Store::scan), so a rule that lives here holds for the report,
    /// the workbook, the snapshot and the Restate handlers at once.
    fn publish(&mut self) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The wire name a table serializes as is the name every other reader uses (`table.file()`, the
    /// name an `Ingest` request and a sidecar file carry). A table whose serde spelling drifted would
    /// be a second name for one collection, so this is held rather than assumed.
    #[test]
    fn a_table_serializes_as_its_wire_name() {
        for table in Table::ALL {
            let encoded = serde_json::to_string(&table).expect("a table encodes");
            assert_eq!(encoded, format!("\"{}\"", table.file()));
            let decoded: Table = serde_json::from_str(&encoded).expect("a table decodes");
            assert_eq!(decoded, table);
        }
    }
}
