//! A caller's page of work as one commit: appends across tables, and the journal entries that name
//! them.
//!
//! An adapter that finishes a unit of work writes rows into several tables and then records the unit in
//! the journal, so a resume skips it. Written one call at a time that is one durability boundary per
//! call — the Athletic.net bio flush reaches seventy `fdatasync` calls for a sixty-four-unit page, each
//! of them free to land while its neighbours have not — and a reader between two of them sees half a
//! unit. [`Store::write_batch`] takes the whole page as one batch instead: every table's observations,
//! the marks those reservations move, and the journal entries all reach the database in a single
//! commit, so the page is one `fdatasync` and a reader sees the page or none of it. Counted on this
//! machine over a store test: the same sixty-four-unit page costs seventy `fdatasync` calls written one
//! call at a time, and one written as a batch.
//!
//! The batch validates as it buffers — every record encoded and keyed, every journal entry inside its
//! ceiling — so a refused call leaves the store untouched and a batch that reaches its commit holds
//! only rows the store can read back. The tables' row ceilings are applied to every table before the
//! first reservation moves, so a batch refused for one table's size leaves every counter where it was.
//!
//! A batch can also be one *operation*: [`StoreBatch::commit_once`] binds the whole page to the
//! caller's operation id and payload digest and writes the receipt beside the rows, so a replay that
//! lost its acknowledgement finds the work already applied instead of appending it again. See
//! [`crate::Receipt`].
//!
//! An evidence page carries appends. A batch that *derives* does something else: a review pass answers
//! a set of cases and must write the verdicts and the cases' new state as one thing, because a crash
//! between two writes leaves a verdict standing against a case that still reads as open.
//! [`StoreBatch::replace_many`] stages a table's whole content into the batch for exactly that, so a
//! derivation's tables land in one commit — and, with [`StoreBatch::commit_once`], under one receipt
//! that a replay finds.

mod append;
mod commit;
mod journal;

use super::Store;
use super::Table;

/// One table's buffered page, already encoded and keyed.
struct Page {
    table: Table,
    records: Vec<(String, Vec<u8>)>,
}

/// One table's staged whole content, already encoded.
///
/// A replacement is a derivation, not evidence: it says what the table holds now, so the rows of a
/// snapshot table that it does not name are removed in the same commit.
struct Replacement {
    table: Table,
    records: Vec<Vec<u8>>,
}

/// Appends across tables, the journal entries that name them, and the derived tables a pass derives
/// together, committed as one.
///
/// Started by [`Store::write_batch`]. Nothing is written until [`StoreBatch::commit`], and a batch
/// dropped without one leaves the store exactly as it found it.
pub struct StoreBatch<'s> {
    store: &'s Store,
    pages: Vec<Page>,
    journal: Vec<(Vec<u8>, Vec<u8>)>,
    replacements: Vec<Replacement>,
}

impl Store {
    /// Start a batch: the appends and journal entries buffered on it commit in one call.
    pub fn write_batch(&self) -> StoreBatch<'_> {
        StoreBatch {
            store: self,
            pages: Vec::new(),
            journal: Vec::new(),
            replacements: Vec::new(),
        }
    }
}

impl StoreBatch<'_> {
    /// Whether the batch holds nothing to write.
    pub fn is_empty(&self) -> bool {
        self.pages.is_empty() && self.journal.is_empty() && self.replacements.is_empty()
    }
}
