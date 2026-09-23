//! Import the researched official coach-contact dataset into canonical entities.
//!
//! The dataset (`data/coach-contacts.csv` in the research workspace) is the consolidated output of the
//! official-source contact graph: one row per (school, sport, role) with the professional email that
//! the school or state association published for that role. Importing it is an *artifact import*, not
//! a crawl: the rows already carry `source_url` + `last_observed`, so each canonical entity can cite
//! the exact page it came from.
//!
//! What is deliberately not read, even when the upstream capture contained it: home phone numbers,
//! home addresses, cell numbers, athlete contacts. Those columns are not part of this schema, so they
//! cannot leak through the importer.
//!
//! Layout: `parse` carries the published label vocabulary, `wire` the dataset's row shape and source
//! coordinates, `entities` mints the canonical school and coaches, and `import` walks the CSV and
//! writes them.

mod entities;
mod import;
mod parse;
mod wire;

pub use entities::row_entities;
pub use import::import_csv;
pub use parse::{parse_role, parse_sport};
pub use wire::{CoachContactRow, RowEntities};

#[cfg(test)]
mod tests;
