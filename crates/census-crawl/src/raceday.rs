//! RaceDay Scoring result exports.
//!
//! RaceDay publishes one HTML page per race with a `<table class="data-display">` grid and the race
//! title in the preceding `<h3>`:
//!
//! ```text
//! <h3>WIAA D2 XC Sectionals - Boys Race Team Finish List-XC</h3>
//! ... Place | Qualifier | Bib | Name | Year | Team Name | Score | Team Member Place | Time | Time | Finish
//! ... 1     | TM        | 2584| Jack Hefty | 11 | Whitewater | 1 | 1 | 05:21.42 | 11:02.38 | 17:13.69
//! ```
//!
//! Columns are matched **by header label**, never by position: the same format publishes team
//! summaries, split tables and finish lists with different column counts. Because the format carries
//! no date anywhere, the caller supplies the archive year; the meet is then stamped with year
//! precision rather than with an invented day.

mod parse;
mod regexes;
mod table;
mod title;

pub use parse::parse;

// The test file reads the division helper through `use super::*`, and `parse` binds it on its
// own, so it is bound here for the tests instead of widening the helper's visibility.
#[cfg(test)]
use title::division_of;

// The test file reads the model and result types through `use super::*`; the parser imports them
// for its own use, so they are bound for tests only instead of widening the parser's imports.
#[cfg(test)]
use census_domain::model::{CentiSeconds, EventKind, Gender, Grade, Mark, SourceRef};

#[cfg(test)]
mod tests;
