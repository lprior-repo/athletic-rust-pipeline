//! The store's table vocabulary: which canonical collections exist, and what an entity must do to
//! live in one.
//!
//! `Table` is the store's own naming for its collections — the names ride in observation
//! keys and in sidecar file names, so they are wire vocabulary rather than an implementation
//! detail. `Entity` is the merge contract every canonical row obeys: the store appends observations
//! and merges them at read time, so a row type must state how a second observation folds into the
//! first, and the contract rules are applied on the merged value rather than on each observation.

use serde::{de::DeserializeOwned, Serialize};

/// Hard ceiling on the observations one table may hold. A table larger than this aborts the scan
/// with a typed error instead of exhausting memory: the bound is what keeps Rule 2 (bounded control
/// flow) honest for a store whose input size is not known in advance.
pub const MAX_ROWS_PER_TABLE: u64 = 20_000_000;

/// Longest entity id the store accepts. Ids ride verbatim inside observation keys, and Fjall
/// asserts keys stay under 64 KiB; this ceiling keeps that assertion unreachable for callers.
pub const MAX_ID_BYTES: usize = 512;

/// The store's thirteen collections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
        }
    }

    pub const ALL: [Table; 15] = [
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
    ];

    /// Parse a wire name (`"schools"`) back into a table. Unknown names are rejected so a typo in an
    /// ingest request cannot silently create a table nobody scans.
    pub fn from_wire(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|table| table.file() == name)
    }
}

/// An entity that knows its own canonical id and how to absorb a duplicate observation.
pub trait Entity: Serialize + DeserializeOwned + Clone {
    fn entity_id(&self) -> &str;
    fn merge(&mut self, other: Self);

    /// Apply the collection contract to a merged entity. Every read of the store goes through
    /// [`Store::scan`](crate::store::Store::scan), so a rule that lives here holds for the report,
    /// the workbook, the snapshot and the Restate handlers at once.
    fn publish(&mut self) {}

    /// How many of this entity's rows carry something the contract withheld.
    /// [`Store::consolidate`](crate::store::Store::consolidate) sums this in the same pass that
    /// writes the snapshot, so reporting the count never re-scans the table.
    fn withheld_mailboxes(&self) -> usize {
        0
    }
}
